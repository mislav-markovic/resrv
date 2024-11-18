use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
    Router,
};
use http::StatusCode;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;
use tracing::{debug, error, info, warn};

pub fn serve_dir_reloadable(dir: &Path) -> Router {
    let state = InjectedHtmlServeDir::new(dir, INJECT.to_owned());
    let state = Arc::new(state);
    let service = ServeDir::new(dir).append_index_html_on_directories(true);

    let service = ServiceBuilder::new()
        .layer(middleware::from_fn_with_state(state, html_reload_injection))
        .service(service);

    Router::new().nest_service("/", service)
}

async fn html_reload_injection(
    State(state): State<InjectServiceState>,
    request: Request,
    next: Next,
) -> Response {
    let uri_path = {
        let mut uri = request.uri().path().to_owned();

        if uri.ends_with('/') {
            uri.push_str("index.html");
        }

        uri
    };

    let asset_path = state
        .asset_dir
        .as_path()
        .join(uri_path.trim_start().trim_start_matches('/'));

    if is_inject_candidate(&asset_path) {
        info!("loading html");
        let content = load_html_from(&asset_path);

        info!("injecting js into html");
        let content = inject_js_into_html(&content, &state.inject_content);

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html")
            .body(content.into())
            .unwrap()
    } else {
        debug!("path NOT candidate for reload injection");
        let resp = next.run(request).await;

        resp
    }
}

fn is_inject_candidate(path: &Path) -> bool {
    info!("is inject candidate? {:?}", path);

    if !path.is_file() {
        info!("no injection -- not a file");
        return false;
    }
    if !path.extension().map(|ext| ext == "html").unwrap_or(false) {
        info!("no injection -- wrong extension: {:?}", path.extension());
        return false;
    }
    if !path.try_exists().unwrap_or(false) {
        info!("no injection -- does not exist");
        return false;
    }

    return true;
}

fn load_html_from(html_path: &Path) -> String {
    match std::fs::read_to_string(html_path) {
        Err(io_err) => {
            error!(
                "failed to load file for injection: {:?}; {}",
                html_path, io_err
            );
            panic!("TODO handle failure to load file");
        }
        Ok(file) => file,
    }
}

fn inject_js_into_html(html: &str, js: &str) -> String {
    const BODY_END_DELIMITER: &'static str = "</body>";

    if let Some((before, after)) = html.split_once(BODY_END_DELIMITER) {
        let total_size =
            before.len() + BODY_END_DELIMITER.len() + 1 /* newline */ + js.len() + after.len();

        let mut buffer = String::with_capacity(total_size);

        buffer.push_str(before);
        buffer.push_str(BODY_END_DELIMITER);
        buffer.push('\n');
        buffer.push_str(js);
        buffer.push_str(after);

        buffer
    } else {
        warn!("could not find where to inject js into html");
        html.to_owned()
    }
}

const INJECT: &'static str = include_str!("injected.html");
type InjectServiceState = Arc<InjectedHtmlServeDir>;

struct InjectedHtmlServeDir {
    asset_dir: PathBuf,
    inject_content: String,
}

impl InjectedHtmlServeDir {
    fn new(asset_dir: impl AsRef<Path>, inject_content: String) -> Self {
        let mut dir = PathBuf::from(".");
        dir.push(asset_dir.as_ref());
        Self {
            asset_dir: dir,
            inject_content,
        }
    }
}
