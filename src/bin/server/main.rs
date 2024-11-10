use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::any,
    Router,
};
use eyre::OptionExt;
use resrv::config;
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

    info!("serve cfg: {:?}", cfg);

    serve(make_router(&cfg.dir), addr).await;
}

fn make_router(dir: &Path) -> Router {
    Router::new()
        .fallback_service(ServeDir::new(dir).append_index_html_on_directories(true))
        .route("/notifyreload", any(ws_handler))
}

async fn serve(app: Router, addr: SocketAddr) {
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    debug!("listening on {}", listener.local_addr().unwrap());

    let app = app
        .layer(TraceLayer::new_for_http())
        .into_make_service_with_connect_info::<SocketAddr>();

    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    info!("ws upgrade");

    ws.on_upgrade(move |socket| handle_socket(socket))
}

async fn handle_socket(mut socket: WebSocket) {
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        info!("Pinged ws client");
    } else {
        error!("Failed to ping ws client");
    }
}
