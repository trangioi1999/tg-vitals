//! Care: find junk (read-only) and move it to the Trash (only via [`trasher`]).
pub mod app_inventory;
pub mod path_guard;
pub mod projects;
pub mod protected_cache;
pub mod scanner;
pub mod targets;
pub mod trasher;

use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItem {
    /// Absolute path.
    pub path: String,
    pub name: String,
    /// Allocated bytes on disk (matches Finder).
    pub size: u64,
    /// unix seconds
    pub modified: Option<u64>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    System,
    Xcode,
    Devtools,
    Projects,
    Files,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupGroup {
    pub id: String,
    pub title: String,
    pub category: Category,
    pub note: String,
    /// Pre-selected in the UI. Only for things that are always safe to regenerate.
    pub safe: bool,
    pub items: Vec<CleanupItem>,
}

impl CleanupGroup {
    pub fn total(&self) -> u64 {
        self.items.iter().map(|i| i.size).sum()
    }
}

/// Progress of a cleanup scan, streamed to the UI while `Scanner::scan_all_with` runs.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ScanEvent {
    /// Step `index` (0-based) of `total` started.
    Step { index: usize, total: usize, title: String },
    /// A folder inside the current step is being read.
    Path { path: String },
    /// Something worth cleaning was found inside the current step (shown as it is found).
    Found { path: String, bytes: u64 },
    /// The current step finished; `bytes` / `items` found (0 when nothing).
    StepDone { title: String, bytes: u64, items: usize },
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct TrashFailure {
    pub path: String,
    pub reason: String,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct TrashReport {
    pub trashed: usize,
    pub bytes: u64,
    pub failed: Vec<TrashFailure>,
}
