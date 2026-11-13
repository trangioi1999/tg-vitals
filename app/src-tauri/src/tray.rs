//! Menu bar item: live text (CPU · RAM · network · temperature) + a Pulse-style popup.
use crate::state::AppState;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, Rect};
use vitals_core::format::compact_bytes;
use vitals_core::Snapshot;

const TRAY_ID: &str = "vitals";
pub const POPUP: &str = "tray";
pub const MAIN: &str = "main";

pub fn create(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Mở tg-vitals", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Thoát tg-vitals", true, Some("Cmd+Q"))?;
    let menu = Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .icon_as_template(true)
        .title("…")
        .tooltip("tg-vitals")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, rect, .. } = event {
                toggle_popup(tray.app_handle(), rect);
            }
        })
        .build(app)?;
    Ok(())
}

/// Menu bar text, e.g. `23% 61% ↓1.2M 47°`.
pub fn title(s: &Snapshot) -> String {
    let mem = if s.memory.total > 0 { s.memory.used() as f64 / s.memory.total as f64 } else { 0.0 };
    let mut t = format!("{:.0}% {:.0}% ↓{}", s.cpu.total * 100.0, mem * 100.0, compact_bytes(s.network.in_per_sec));
    if let Some(temp) = s.cpu.temperature {
        t.push_str(&format!(" {temp:.0}°"));
    }
    t
}

pub fn update_title(app: &AppHandle, s: &Snapshot) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(title(s)));
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window(POPUP) {
        let _ = popup.hide();
    }
    if let Some(main) = app.get_webview_window(MAIN) {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
}

fn toggle_popup(app: &AppHandle, rect: Rect) {
    let Some(popup) = app.get_webview_window(POPUP) else { return };
    if popup.is_visible().unwrap_or(false) {
        let _ = popup.hide();
        return;
    }
    // The click that blurred (and hid) the popup also lands here: don't reopen it.
    let state = app.state::<AppState>();
    if state.popup_hidden_at.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(300)) {
        return;
    }
    let scale = popup.scale_factor().unwrap_or(2.0);
    let icon_pos = rect.position.to_physical::<f64>(scale);
    let icon_size = rect.size.to_physical::<f64>(scale);
    let width = popup.outer_size().map(|s| s.width as f64).unwrap_or(320.0 * scale);
    let x = icon_pos.x + icon_size.width / 2.0 - width / 2.0;
    let y = icon_pos.y + icon_size.height + 4.0 * scale;
    let _ = popup.set_position(PhysicalPosition::new(x.max(0.0), y));
    let _ = popup.show();
    let _ = popup.set_focus();
}

pub fn on_popup_blur(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window(POPUP) {
        let _ = popup.hide();
    }
    *app.state::<AppState>().popup_hidden_at.lock().unwrap() = Some(Instant::now());
}
