use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::{any, get},
    Router,
};
use eyre::OptionExt;
use futures::{SinkExt, StreamExt};
use resrv::{
    asset_tracker::{self, AssetTracker, ReloadEvent},
    config,
    serve_dir_reload::serve_dir_reloadable,
};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::Path,
    sync::{atomic::AtomicU32, Arc},
};
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
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
    let _track_result = tracker_task.await;
}

type TrackerTx = tokio::sync::watch::Sender<ReloadEvent>;
type TrackerRx = tokio::sync::watch::Receiver<ReloadEvent>;

fn make_router(dir: &Path, rx: TrackerRx) -> Router {
    let counter = Arc::new(AtomicU32::new(0));
    let state = TrackerState { rx, counter };

    Router::new()
        .route("/notifyreload", any(ws_handler))
        .route(
            "/foo",
            get(|| async {
                info!("hello from foo");
            }),
        )
        .with_state(state)
        .merge(serve_dir_reloadable(&dir))
}

#[derive(Clone)]
struct TrackerState {
    rx: TrackerRx,
    counter: Arc<AtomicU32>,
}

async fn broadcast_asset_change(mut tracker: AssetTracker, tx: TrackerTx) {
    loop {
        let event = tracker.track_change().await;
        let _ = tx.send_replace(event);
    }
}

async fn notfiy_reload(ws: WebSocket, mut rx: TrackerRx, id: u32) {
    if let Ok(true) = rx.has_changed() {
        debug!("[{id}] discarding initial changed notification");
        rx.mark_unchanged();
    }

    let (mut ws_tx, mut ws_rx) = ws.split();

    let mut send_task = tokio::spawn(async move {
        while let Ok(()) = rx.changed().await {
            let ws_msg = Message::Text("reload".to_string());
            if let Err(e) = ws_tx.send(ws_msg).await {
                error!("failed to send message on web socket: {:?}", e);
                break;
            } else {
                debug!("[{id}] sent reload event");
            }
        }

        warn!("stopping notify reload ws sender");
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            if let Message::Close(_close_frame_option) = msg {
                warn!("[{id}] received close message");
                break;
            }
        }
    });

    tokio::select! {
        _tx_rv = (&mut send_task) => recv_task.abort(),
        _rx_rv = (&mut recv_task) => send_task.abort()
    };

    info!("[{id}] ending notify reload handler");
}

async fn serve(app: Router, addr: SocketAddr) {
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("listening on {}", listener.local_addr().unwrap());

    let app = app
        .layer(TraceLayer::new_for_http())
        .into_make_service_with_connect_info::<SocketAddr>();

    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(TrackerState { rx, counter }): State<TrackerState>,
) -> impl IntoResponse {
    let counter = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    ws.on_upgrade(move |socket| notfiy_reload(socket, rx, counter))
}
