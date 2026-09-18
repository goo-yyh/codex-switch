#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(target_os = "macos")]
mod app_menu;
mod commands;
mod locale;
mod platform;
mod service;
mod state;
mod tray;
mod updates;
use codex_switch_core::{config::ConfigManager, store::Store};
use state::AppState;
use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;

fn main() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.menu(app_menu::build);
    builder
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            let home = std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".codex"));
            let config = ConfigManager::new(home, data.clone());
            let was_enabled = config.enabled().unwrap_or(false);
            let store = Store::open(&data.join("connections.db"))?;
            app.manage(AppState {
                store: Mutex::new(store),
                config,
                runtime: tokio::sync::Mutex::new(None),
                operation: tokio::sync::Mutex::new(()),
                pending: Mutex::new(was_enabled),
                tray_feedback: Mutex::new(None),
                checks: Mutex::new(std::collections::HashMap::new()),
            });
            #[cfg(target_os = "macos")]
            {
                app.set_menu(app_menu::build(app.handle())?)?;
                app_menu::configure_visibility()?;
            }
            tray::install(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            updates::check_update,
            commands::set_locale,
            commands::cancel_validation,
            commands::save_profile,
            commands::save_routing_settings,
            commands::probe_endpoint,
            commands::set_enabled,
            commands::select_profiles,
            commands::delete_profile,
            commands::open_codex,
            commands::restart_codex,
            commands::open_link,
            commands::set_autostart,
            commands::quit,
        ])
        .build(tauri::generate_context!())
        .expect("Unable to initialize Codex Switch")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } = event
            {
                api.prevent_exit();
                tray::request_quit(app);
            }
        });
}
