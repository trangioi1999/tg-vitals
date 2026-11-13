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
            commands::quit_app,
        ])
        .setup(|app| {
            tray::create(app)?;
            state::start_sampler(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // Closing the main window keeps the app alive in the menu bar.
            (tray::MAIN, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            (tray::POPUP, WindowEvent::Focused(false)) => tray::on_popup_blur(window.app_handle()),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tg-vitals")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let RunEvent::Reopen { .. } = event {
                tray::show_main(app);
            }
            let _ = (app, event);
        });
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
