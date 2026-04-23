use std::path::PathBuf;

/// RAII guard that removes a path on drop unless `commit()` is called.
/// Uses sync fs because Drop can't be async; this only runs on the error
/// path and is a single local unlink.
pub struct PartialFileGuard {
    path: Option<PathBuf>,
}

impl PartialFileGuard {
    pub fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    pub fn commit(mut self) {
        self.path = None;
    }
}

impl Drop for PartialFileGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_file(&path);
        }
    }
}