//! Commands called from the frontend (`app/src/api/index.ts`).
use crate::state::{home, AppState};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};
use tauri::ipc::Channel;
use vitals_core::care::{scanner::Scanner, trasher, CleanupGroup, ScanEvent, TrashFailure, TrashReport};
#[cfg(target_os = "macos")]
use vitals_core::care::{app_inventory::{self, AppInfo}, apps::{self, AppLeftover}};
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
pub async fn scan_cleanup(on_event: Channel<ScanEvent>, state: State<'_, AppState>) -> Result<Vec<CleanupGroup>, String> {
    // Progress is best effort: a closed window must not abort the scan.
    let groups = tauri::async_runtime::spawn_blocking(move || Scanner::new(home()).scan_all_with(&mut |e| { let _ = on_event.send(e); }))
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

/// Quits tg-vitals itself (tray popup button).
#[tauri::command]
pub fn quit_self(app: AppHandle) {
    app.exit(0);
}

// ---------- Gỡ app ----------

/// Installed apps, biggest first. Streams the app being read as `ScanEvent::Path`.
/// Remembers the result: only these bundles (and leftovers found later) may be uninstalled.
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn list_apps(on_event: Channel<ScanEvent>, state: State<'_, AppState>) -> Result<Vec<AppInfo>, String> {
    let roots = vec![std::path::PathBuf::from("/Applications"), home().join("Applications")];
    let (apps, installed) = tauri::async_runtime::spawn_blocking(move || {
        let apps = app_inventory::list_apps_with(&roots, &mut |path| {
            let _ = on_event.send(ScanEvent::Path { path: path.to_string() });
        });
        (apps, app_inventory::installed_identities(&roots))
    })
    .await
    .map_err(|e| e.to_string())?;
    *state.app_paths.lock().unwrap() =
        apps.iter().map(|a| (a.path.clone(), HashMap::from([(a.path.clone(), a.size)]))).collect();
    *state.installed.lock().unwrap() = installed;
    *state.apps.lock().unwrap() = apps.clone();
    Ok(apps)
}

/// Data an app left in ~/Library and /Library, matched by bundle id / team id.
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn app_leftovers(bundle_path: String, state: State<'_, AppState>) -> Result<Vec<AppLeftover>, String> {
    let apps = state.apps.lock().unwrap().clone();
    let target = apps.iter().find(|a| a.path == bundle_path).ok_or("App không có trong danh sách gần nhất")?;
    let me = target.identity();
    // Everything that stays installed (any depth, removable or not). The app itself appears once
    // in that list; drop one entry with its id so a *second* copy still blocks shared data.
    let mut others = state.installed.lock().unwrap().clone();
    if let Some(i) = others.iter().position(|o| o.bundle_id == me.bundle_id) {
        others.remove(i);
    }
    let leftovers = tauri::async_runtime::spawn_blocking(move || apps::find_leftovers(&me, &others, &apps::leftover_roots(&home())))
        .await
        .map_err(|e| e.to_string())?;
    let mut allowed = state.app_paths.lock().unwrap();
    let mine = allowed.entry(bundle_path.clone()).or_default();
    for l in &leftovers {
        mine.insert(l.path.clone(), l.size.unwrap_or(0));
    }
    Ok(leftovers)
}

/// Asks a running app to quit (it may still show its own "save changes?" dialog).
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn quit_app(bundle_id: String, app: AppHandle) {
    let _ = app.run_on_main_thread(move || {
        use objc2_app_kit::NSRunningApplication;
        use objc2_foundation::NSString;
        for running in NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(&bundle_id)) {
            running.terminate();
        }
    });
}

/// Moves the chosen bundle + leftovers to the Trash (Finder asks for the admin password when
/// needed), then forgets the app's installer receipts. Refuses while the app runs.
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn uninstall_app(bundle_path: String, paths: Vec<String>, state: State<'_, AppState>) -> Result<TrashReport, String> {
    let bundle_id = state.apps.lock().unwrap().iter().find(|a| a.path == bundle_path).map(|a| a.bundle_id.clone());
    // Only this app's own bundle + leftovers may be touched.
    let allowed = state.app_paths.lock().unwrap().get(&bundle_path).cloned();
    if let Err(reason) = apps::check_uninstall(&bundle_path, allowed.is_some(), &vitals_core::process::running_bundles()) {
        return Ok(TrashReport {
            failed: paths.into_iter().map(|path| TrashFailure { path, reason: reason.into() }).collect(),
            ..Default::default()
        });
    }
    let allowed = allowed.unwrap_or_default();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let report = trasher::recycle_with_auth(&paths, &allowed, &home());
        if let Some(id) = bundle_id.filter(|_| !std::path::Path::new(&bundle_path).exists()) {
            trasher::forget_receipts(&id);
        }
        report
    })
    .await
    .map_err(|e| e.to_string())?;
    // The listing is stale now; the page lists again.
    state.app_paths.lock().unwrap().clear();
    Ok(report)
}
