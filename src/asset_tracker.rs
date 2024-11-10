use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tracing::{debug, error, info, warn};

const ASSET_TRACKER_BUFFER_SIZE: usize = 128;

pub enum ReloadEvent {
    FileChange,
}

pub struct AssetTracker {
    _tx: tokio::sync::mpsc::Sender<ReloadEvent>,
    rx: tokio::sync::mpsc::Receiver<ReloadEvent>,
    _watcher: RecommendedWatcher,
}

impl AssetTracker {
    pub fn new_for_dir(asset_dir: &Path) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<ReloadEvent>(ASSET_TRACKER_BUFFER_SIZE);

        let event_tx = tx.clone();
        let rt = tokio::runtime::Handle::current();
        // event handler spawns a task that asynchrounsly pushes file change notification
        let mut watcher = notify::recommended_watcher(move |event| {
            info!("enter event handler");
            let tx = event_tx.clone();

            match event {
                Ok(event) => {
                    debug!("notify event: {:?}", event);

                    rt.spawn(async move {
                        debug!("sending reload event");
                        tx.send(ReloadEvent::FileChange).await.unwrap();
                        debug!("sent reload event");
                    });
                }
                Err(err) => {
                    error!("notify event error: {err:?}");
                }
            };
        })
        .unwrap();

        watcher.watch(asset_dir, RecursiveMode::Recursive).unwrap();

        debug!("watcher started on {:?}", asset_dir);

        let tracker = Self {
            _tx: tx.clone(),
            rx,
            _watcher: watcher,
        };
        tracker
    }

    pub async fn track_change(&mut self) -> ReloadEvent {
        self.rx
            .recv()
            .await
            .expect("asset tracker receiver should never see channel close error")
    }
}
