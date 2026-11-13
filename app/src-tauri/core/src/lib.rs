//! tg-vitals core: reads system metrics and finds/trashes junk.
//!
//! Rules (see CLAUDE.md):
//! - No UI code here. Pure calculations are plain functions with tests.
//! - Deleting files happens ONLY in [`care::trasher`], and only to the Trash.

pub mod care;
pub mod format;
pub mod model;

pub mod cpu;
pub mod memory;
pub mod network;
pub mod process;
pub mod machine;
pub mod sensors;

#[cfg(target_os = "macos")]
mod iokit;
#[cfg(target_os = "macos")]
pub mod sys;
#[cfg(target_os = "macos")]
pub mod icon;
#[cfg(target_os = "macos")]
pub mod sampler;

pub use model::*;
