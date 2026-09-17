//! Native tray smoke test: cargo run -p codex-switch-desktop --example tray-smoke
//! Uses in-memory profiles and temporary configuration files; never reads keys,
//! launches Codex, starts a gateway, or touches the real application data directory.
use codex_switch_core::{
    config::ConfigManager,
    profiles::{ensure_editable, Profile},
    providers::Protocol,
    store::Store,
};
use std::sync::Mutex;
use tauri::{Listener, State};
#[path = "../src/tray.rs"]
mod tray;
type CommandResult<T> = Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
struct AppState {
    store: Mutex<Store>,
    config: ConfigManager,
    operation: tokio::sync::Mutex<()>,
    pending: Mutex<bool>,
    tray_feedback: Mutex<Option<tray::Feedback>>,
    #[cfg(test)]
    runtime: tokio::sync::Mutex<Option<codex_switch_core::gateway::Gateway>>,
    #[cfg(test)]
    unavailable: Mutex<Vec<codex_switch_core::connections::UnavailableConnection>>,
    #[cfg(test)]
    checks: Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
}
mod platform {
    pub struct AppInfo {
        pub running: bool,
    }
    pub fn app_info() -> AppInfo {
        AppInfo { running: false }
    }
}
fn select_profiles_inner(s: &AppState, ids: &[String]) -> CommandResult<()> {
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    s.store
        .lock()
        .map_err(err)?
        .select_profiles(ids)
        .map_err(err)?;
    println!("SELECTION {ids:?}");
    Ok(())
}
async fn enable_inner(_: &AppState) -> CommandResult<()> {
    Err("隔离测试不启动服务。".into())
}
async fn set_enabled_inner(_: &AppState, _: bool) -> CommandResult<()> {
    Err("隔离测试不启动服务。".into())
}
async fn quit(app: tauri::AppHandle, _: State<'_, AppState>) -> CommandResult<()> {
    app.exit(0);
    Ok(())
}
fn main() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::memory().unwrap();
    for (id, name) in [
        ("a", "测试配置 A"),
        ("b", "测试配置 B"),
        ("c", "测试配置 C"),
    ] {
        store
            .save_profile(&Profile {
                id: id.into(),
                name: name.into(),
                preset_id: "custom".into(),
                endpoint: "https://example.invalid/v1".into(),
                models: vec!["test-model".into()],
                protocol: Protocol::Chat,
                context_window: 32768,
                options: Default::default(),
                credential_id: format!("test-{id}"),
                route_ids: vec![format!("route-{id}")],
            })
            .unwrap();
    }
    let state = AppState {
        store: Mutex::new(store),
        config: ConfigManager::new(temp.path().join("codex"), temp.path().join("state")),
        operation: tokio::sync::Mutex::new(()),
        pending: Mutex::new(false),
        tray_feedback: Mutex::new(None),
        #[cfg(test)]
        runtime: Default::default(),
        #[cfg(test)]
        unavailable: Default::default(),
        #[cfg(test)]
        checks: Default::default(),
    };
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    context.config_mut().identifier = "com.codexdocs.switch.tray-smoke".into();
    context.config_mut().product_name = Some("Tray Smoke".into());
    tauri::Builder::default()
        .manage(state)
        .menu(|app| {
            let open = tauri::menu::MenuItem::with_id(
                app,
                "test-open",
                "Open test tray",
                true,
                None::<&str>,
            )?;
            let submenu = tauri::menu::Submenu::with_items(app, "Test", true, &[&open])?;
            tauri::menu::Menu::with_items(app, &[&submenu])
        })
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "test-open" {
                let _ = app
                    .tray_by_id("codex-switch")
                    .unwrap()
                    .with_inner_tray_icon(|inner| inner.show_menu());
            }
        })
        .setup(|app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External("about:blank".parse().unwrap()),
            )
            .title("Tray Smoke — isolated test")
            .inner_size(360.0, 160.0)
            .build()?;
            tray::install(app.handle())?;
            // Simulate the main panel refreshing its snapshot after each selection.
            let handle = app.handle().clone();
            app.listen("tray-state-changed", move |_| {
                tray::sync(&handle).expect("refresh menu after selection");
            });
            app.tray_by_id("codex-switch")
                .unwrap()
                .set_title(Some("TEST"))?;
            Ok(())
        })
        .run(context)
        .unwrap();
    drop(temp);
}
