//! Shared app state + the background sampling thread.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use vitals_core::Snapshot;

/// How long after the UI last asked for data we keep sampling the process list.
pub const DETAIL_GRACE: Duration = Duration::from_secs(3);

pub struct AppState {
    pub latest: Mutex<Option<Snapshot>>,
    /// Last time a window asked for a snapshot (process list is skipped when nobody looks).
    pub last_request: Mutex<Instant>,
    /// Paths from the last cleanup scan → size. Only these may be trashed.
    pub scanned: Mutex<HashMap<String, u64>>,
    /// Bundle path → PNG bytes.
    pub icons: Mutex<HashMap<String, Arc<Vec<u8>>>>,
    /// When the tray popup was last hidden because it lost focus (to ignore the same click).
    pub popup_hidden_at: Mutex<Option<Instant>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            latest: Mutex::new(None),
            last_request: Mutex::new(Instant::now()),
            scanned: Mutex::new(HashMap::new()),
            icons: Mutex::new(HashMap::new()),
            popup_hidden_at: Mutex::new(None),
        }
    }
}

pub fn home() -> std::path::PathBuf {
    std::env::var_os("HOME").map(Into::into).unwrap_or_else(|| "/var/empty".into())
}

#[cfg(target_os = "macos")]
pub fn start_sampler(app: tauri::AppHandle) {
    use tauri::Manager;
    use vitals_core::{format::encode_path, machine, sampler::Sampler};

    let model_name: Arc<Mutex<Option<String>>> = Arc::default();
    {
        let model_name = model_name.clone();
        std::thread::spawn(move || {
            if let Some(name) = machine::marketing_name() {
                *model_name.lock().unwrap() = Some(name);
            }
        });
    }

    std::thread::Builder::new()
        .name("sampler".into())
        .spawn(move || {
            let mut sampler = Sampler::new();
            let icon_url = |bundle: &str| format!("appicon://localhost{}", encode_path(bundle));
            loop {
                let started = Instant::now();
                if let Some(name) = model_name.lock().unwrap().take() {
                    sampler.set_model_name(name);
                }
                let state = app.state::<AppState>();
                let wanted = state.last_request.lock().unwrap().elapsed() < DETAIL_GRACE;
                let snapshot = sampler.sample(wanted, &icon_url);
                crate::tray::update_title(&app, &snapshot);
                *state.latest.lock().unwrap() = Some(snapshot);
                std::thread::sleep(Duration::from_secs(1).saturating_sub(started.elapsed()));
            }
        })
        .expect("spawn sampler thread");
}

#[cfg(not(target_os = "macos"))]
pub fn start_sampler(_app: tauri::AppHandle) {}
