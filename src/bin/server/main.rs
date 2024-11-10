use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::any,
    Router,
};
use eyre::OptionExt;
use resrv::{
    asset_tracker::{self, AssetTracker, ReloadEvent},
    config,
};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::Path,
};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = config::load_cfg().unwrap();
    let addr: SocketAddr = cfg
        .url
        .to_socket_addrs()
        .unwrap()
        .next()
        .ok_or_eyre("failed to resolve url to socket addr")
        .unwrap();

    info!("cfg: {:?}", cfg);
    let tracker = asset_tracker::AssetTracker::new_for_dir(&cfg.dir);
    let (tx, rx) = tokio::sync::watch::channel(ReloadEvent::FileChange);

    let tracker_task = tokio::spawn(broadcast_asset_change(tracker, tx));

    serve(make_router(&cfg.dir, rx), addr).await;
    let _ = tracker_task.await;
}

type TrackerTx = tokio::sync::watch::Sender<ReloadEvent>;
type TrackerRx = tokio::sync::watch::Receiver<ReloadEvent>;

fn make_router(dir: &Path, rx: TrackerRx) -> Router {
    let state = TrackerState { rx };
    Router::new()
        .fallback_service(ServeDir::new(dir).append_index_html_on_directories(true))
        .route("/notifyreload", any(ws_handler))
        .with_state(state)
}

#[derive(Clone)]
struct TrackerState {
    rx: TrackerRx,
}

async fn broadcast_asset_change(mut tracker: AssetTracker, tx: TrackerTx) {
    loop {
        let event = tracker.track_change().await;
        let _ = tx.send_replace(event);
    }
}

async fn notfiy_reload(mut ws: WebSocket, mut rx: TrackerRx) {
    while let Ok(()) = rx.changed().await {
        {
            let _event = rx.borrow_and_update();
        }

        let ws_msg = Message::Text("reload".to_string());
        if let Err(e) = ws.send(ws_msg).await {
            error!("failed to send message on web socket: {:?}", e);
            return;
        }
    }
}

async fn serve(app: Router, addr: SocketAddr) {
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    debug!("listening on {}", listener.local_addr().unwrap());

    let app = app
        .layer(TraceLayer::new_for_http())
        .into_make_service_with_connect_info::<SocketAddr>();

    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(TrackerState { rx }): State<TrackerState>,
) -> impl IntoResponse {
    info!("ws upgrade");

    ws.on_upgrade(move |socket| notfiy_reload(socket, rx))
}
