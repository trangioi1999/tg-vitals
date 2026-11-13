//! Menu bar item: live text (CPU · RAM · network · temperature) + a Pulse-style popup.
use crate::state::AppState;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, Rect};
use vitals_core::format::{compact_bytes, Load};
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

/// What each menu bar item is; picks its label color.
#[derive(Clone, Copy)]
enum Kind {
    Cpu,
    Ram,
    Net,
    Temp,
}

/// One menu bar item: label + value, and how worrying the value is.
struct Metric {
    kind: Kind,
    label: &'static str,
    value: String,
    load: Load,
}

/// Label/value pairs keep the menu bar compact without making CPU and RAM ambiguous.
fn metrics(s: &Snapshot) -> Vec<Metric> {
    let mem = if s.memory.total > 0 { s.memory.used() as f64 / s.memory.total as f64 } else { 0.0 };
    let mut items = vec![
        Metric { kind: Kind::Cpu, label: "CPU", value: format!("{:.0}%", s.cpu.total * 100.0), load: Load::cpu(s.cpu.total) },
        Metric { kind: Kind::Ram, label: "RAM", value: format!("{:.0}%", mem * 100.0), load: Load::memory(mem) },
        Metric { kind: Kind::Net, label: "↓", value: compact_bytes(s.network.in_per_sec), load: Load::Normal },
    ];
    if let Some(temp) = s.cpu.temperature.filter(|t| t.is_finite()) {
        items.push(Metric { kind: Kind::Temp, label: "🌡", value: format!("{temp:.0}°"), load: Load::temperature(temp) });
    }
    items
}

pub fn title(s: &Snapshot) -> String {
    metrics(s).iter().map(|m| format!("{} {}", m.label, m.value)).collect::<Vec<_>>().join("  ")
}

pub fn update_title(app: &AppHandle, s: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let plain = title(s);
    let tooltip = format!(
        "tg-vitals\nCPU: {:.0}%\nRAM: {}\nTải xuống: {}/s\nTải lên: {}/s\nNhiệt độ: {}\nBấm để xem chi tiết",
        s.cpu.total * 100.0, metrics(s)[1].value,
        compact_bytes(s.network.in_per_sec), compact_bytes(s.network.out_per_sec),
        s.cpu.temperature.filter(|t| t.is_finite()).map(|t| format!("{t:.0} °C")).unwrap_or_else(|| "Chưa có dữ liệu".into())
    );
    let _ = tray.set_tooltip(Some(tooltip));
    #[cfg(target_os = "macos")]
    {
        let items = metrics(s);
        // Tauri dispatches this entire closure to the main thread. Apply plain text first
        // so the tray backend updates its hit-test dimensions, then style in the same turn.
        let _ = tray.with_inner_tray_icon(move |inner| {
            use objc2::{AnyThread, MainThreadMarker};
            use objc2_app_kit::{NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName};
            use objc2_foundation::{NSMutableAttributedString, NSRange, NSString};
            inner.set_title(Some(&plain));
            let Some(mtm) = MainThreadMarker::new() else { return };
            let Some(item) = inner.ns_status_item() else { return };
            let Some(button) = item.button(mtm) else { return };
            let text = NSMutableAttributedString::initWithString(
                NSMutableAttributedString::alloc(), &NSString::from_str(&plain));
            let whole = NSRange::new(0, plain.encode_utf16().count());
            // Labels: lighter weight in each metric's own color. Values: bold, white while normal,
            // yellow / red when high / critical, so a problem stands out at a glance.
            let label_font = NSFont::systemFontOfSize_weight(11.0, 0.3);
            let value_font = NSFont::monospacedDigitSystemFontOfSize_weight(12.5, 0.56);
            let label_color = |kind: Kind| match kind {
                Kind::Cpu => NSColor::systemCyanColor(),
                Kind::Ram => NSColor::systemGreenColor(),
                Kind::Net => NSColor::systemPurpleColor(),
                Kind::Temp => NSColor::systemOrangeColor(),
            };
            let value_color = |load: Load| match load {
                Load::Normal => NSColor::labelColor(),
                Load::High => NSColor::systemYellowColor(),
                Load::Critical => NSColor::systemRedColor(),
            };
            // SAFETY: Font/color attributes receive the documented AppKit object types.
            // Every range is measured in UTF-16, matching NSString, and stays in bounds.
            unsafe {
                text.addAttribute_value_range(NSFontAttributeName, &label_font, whole);
                text.addAttribute_value_range(NSForegroundColorAttributeName, &NSColor::labelColor(), whole);
                let mut offset = 0;
                for m in &items {
                    let label_len = m.label.encode_utf16().count();
                    let value_len = m.value.encode_utf16().count();
                    let label = NSRange::new(offset, label_len);
                    let value = NSRange::new(offset + label_len + 1, value_len);
                    text.addAttribute_value_range(NSForegroundColorAttributeName, &label_color(m.kind), label);
                    text.addAttribute_value_range(NSFontAttributeName, &value_font, value);
                    text.addAttribute_value_range(NSForegroundColorAttributeName, &value_color(m.load), value);
                    offset += label_len + 1 + value_len + 2;
                }
            }
            button.setAttributedTitle(&text);
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = tray.set_title(Some(plain));
}

pub fn show_main(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window(POPUP) {
        let _ = popup.hide();
    }
    // Back in the Dock and app switcher while the window is open.
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(main) = app.get_webview_window(MAIN) {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
}

/// Hide the main window and drop the Dock icon; the menu bar item keeps running.
pub fn hide_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window(MAIN) {
        let _ = main.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
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
