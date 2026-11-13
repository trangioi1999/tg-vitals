//! Care: find junk (read-only) and move it to the Trash (only via [`trasher`]).
pub mod path_guard;
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
