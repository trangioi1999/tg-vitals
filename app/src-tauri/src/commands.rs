//! Commands called from the frontend (`app/src/api/index.ts`).
use crate::state::{home, AppState};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};
use vitals_core::care::{scanner::Scanner, trasher, CleanupGroup, TrashReport};
use vitals_core::Snapshot;

/// Latest snapshot. Waits briefly at startup, or when the process list was paused
/// because no window was looking.
#[tauri::command]
pub async fn snapshot(app: AppHandle) -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        *state.last_request.lock().unwrap() = Instant::now();
        let deadline = Instant::now() + Duration::from_millis(2500);
        loop {
            let latest = state.latest.lock().unwrap().clone();
            match latest {
                Some(s) if !s.processes.is_empty() || Instant::now() > deadline => return Ok(s),
                None if Instant::now() > deadline => return Err("Chưa có số liệu".into()),
                _ => std::thread::sleep(Duration::from_millis(100)),
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn kill_process(pid: i32, force: bool) -> Result<(), String> {
    if pid <= 1 || pid == std::process::id() as i32 {
        return Err("Không thể dừng process này".into());
    }
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    // SAFETY: plain syscall.
    if unsafe { libc::kill(pid, signal) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

/// Shows a file in Finder (`open -R`). Read-only.
#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if !p.is_absolute() || !p.exists() {
        return Err("Không tìm thấy file".into());
    }
    std::process::Command::new("/usr/bin/open").arg("-R").arg(p).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Read-only scan. Remembers the result: only these paths may be trashed afterwards.
#[tauri::command]
pub async fn scan_cleanup(state: State<'_, AppState>) -> Result<Vec<CleanupGroup>, String> {
    let groups = tauri::async_runtime::spawn_blocking(|| Scanner::new(home()).scan_all())
        .await
        .map_err(|e| e.to_string())?;
    *state.scanned.lock().unwrap() =
        groups.iter().flat_map(|g| g.items.iter().map(|i| (i.path.clone(), i.size))).collect();
    Ok(groups)
}

/// Moves the given paths to the Trash (never deletes permanently).
#[tauri::command]
pub async fn trash_paths(paths: Vec<String>, state: State<'_, AppState>) -> Result<TrashReport, String> {
    let allowed = state.scanned.lock().unwrap().clone();
    let report = tauri::async_runtime::spawn_blocking(move || trasher::trash(&paths, &allowed, &home()))
        .await
        .map_err(|e| e.to_string())?;
    // Everything is stale after a clean; force a new scan before the next one.
    state.scanned.lock().unwrap().clear();
    Ok(report)
}

#[tauri::command]
pub fn show_main(app: AppHandle) {
    crate::tray::show_main(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
