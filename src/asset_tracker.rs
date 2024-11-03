use notify::{RecursiveMode, Watcher};
use std::path::Path;

const ASSET_TRACKER_BUFFER_SIZE: usize = 128;

pub enum ReloadEvent {
    FileChange,
}

pub struct AssetTracker {
    _tx: tokio::sync::mpsc::Sender<ReloadEvent>,
    rx: tokio::sync::mpsc::Receiver<ReloadEvent>,
}

impl AssetTracker {
    pub fn new_for_dir(asset_dir: &Path) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<ReloadEvent>(ASSET_TRACKER_BUFFER_SIZE);
        let tracker = Self {
            _tx: tx.clone(),
            rx,
        };

        // event handler spawns a task that asynchrounsly pushes file change notification
        let mut watcher = notify::recommended_watcher(move |_event| {
            let tx = tx.clone();

            tokio::spawn(async move { tx.send(ReloadEvent::FileChange).await });
        })
        .unwrap();

        watcher.watch(&asset_dir, RecursiveMode::Recursive).unwrap();

        tracker
    }

    pub async fn track_change(&mut self) -> ReloadEvent {
        self.rx
            .recv()
            .await
            .expect("asset tracker receiver should never see channel close error")
    }
}
