//! tg-vitals app shell: windows, menu bar item, commands. Metrics live in `vitals-core`.
mod commands;
mod state;
mod tray;

use state::AppState;
use std::sync::Arc;
use tauri::http::{header, Response, StatusCode};
use tauri::{Manager, RunEvent, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .register_asynchronous_uri_scheme_protocol("appicon", |ctx, request, responder| {
            let path = vitals_core::format::decode_path(request.uri().path());
            serve_icon(ctx.app_handle().clone(), path, responder);
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::kill_process,
            commands::reveal_path,
            commands::scan_cleanup,
            commands::trash_paths,
            commands::show_main,
            commands::quit_self,
            commands::list_apps,
            commands::app_leftovers,
            commands::quit_app,
            commands::uninstall_app,
        ])
        .menu(app_menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_HIDE => tray::hide_main(app),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .setup(|app| {
            tray::create(app)?;
            state::start_sampler(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // Closing the main window keeps the app alive in the menu bar.
            (tray::MAIN, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                tray::hide_main(window.app_handle());
            }
            (tray::POPUP, WindowEvent::Focused(false)) => tray::on_popup_blur(window.app_handle()),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tg-vitals")
        .run(|app, event| match event {
            // An exit request without a code (if the runtime ever reports one for Cmd+Q / Dock
            // "Quit") only hides the window. `app.exit(0)` (tray / ⌥⌘Q) carries a code and quits.
            RunEvent::ExitRequested { code: None, api, .. } => {
                api.prevent_exit();
                tray::hide_main(app);
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => tray::show_main(app),
            _ => {}
        });
}

const MENU_HIDE: &str = "hide_main";
const MENU_QUIT: &str = "quit_all";

/// App menu with Cmd+Q bound to "hide to the menu bar" instead of the default
/// `terminate:` item, which kills the process before Tauri can intervene.
/// ⌥⌘Q (and the tray menu) really quit.
fn app_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem as P, Submenu};
    let name = app.package_info().name.clone();
    let app_menu = Submenu::with_items(
        app,
        &name,
        true,
        &[
            &P::about(app, None, None)?,
            &P::separator(app)?,
            &P::hide(app, None)?,
            &P::hide_others(app, None)?,
            &P::separator(app)?,
            &MenuItem::with_id(app, MENU_HIDE, "Thu về thanh menu", true, Some("CmdOrCtrl+Q"))?,
            &MenuItem::with_id(app, MENU_QUIT, "Thoát hẳn tg-vitals", true, Some("CmdOrCtrl+Alt+Q"))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[&P::undo(app, None)?, &P::redo(app, None)?, &P::separator(app)?, &P::cut(app, None)?,
          &P::copy(app, None)?, &P::paste(app, None)?, &P::select_all(app, None)?],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[&P::minimize(app, None)?, &P::close_window(app, None)?],
    )?;
    Menu::with_items(app, &[&app_menu, &edit, &window])
}

/// `appicon://localhost/<bundle path>` → 64 px PNG of that app's icon (cached).
fn serve_icon(app: tauri::AppHandle, path: String, responder: tauri::UriSchemeResponder) {
    let respond = |responder: tauri::UriSchemeResponder, png: Option<Arc<Vec<u8>>>| {
        let response = match png {
            Some(bytes) => Response::builder()
                .header(header::CONTENT_TYPE, "image/png")
                .header(header::CACHE_CONTROL, "max-age=86400")
                .body(bytes.as_ref().clone()),
            None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
        };
        responder.respond(response.expect("valid response"));
    };

    if let Some(cached) = app.state::<AppState>().icons.lock().unwrap().get(&path).cloned() {
        return respond(responder, Some(cached));
    }
    if !path.ends_with(".app") || !std::path::Path::new(&path).is_dir() {
        return respond(responder, None);
    }

    #[cfg(target_os = "macos")]
    {
        let handle = app.clone();
        // AppKit drawing on the main thread to be safe.
        let _ = app.run_on_main_thread(move || {
            let png = vitals_core::icon::png_for_path(&path, 64).map(Arc::new);
            if let Some(png) = &png {
                handle.state::<AppState>().icons.lock().unwrap().insert(path, png.clone());
            }
            respond(responder, png);
        });
    }
    #[cfg(not(target_os = "macos"))]
    respond(responder, None);
}
