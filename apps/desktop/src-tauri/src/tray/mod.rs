use crate::{
    service::{enable_inner, select_profiles_inner, set_enabled_inner},
    state::{err, AppState, CommandResult},
};
use codex_switch_core::profiles::Profile;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

#[cfg(target_os = "macos")]
mod persistent_menu;

const TRAY_ID: &str = "codex-switch";
const PROFILE_PREFIX: &str = "profile:";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feedback {
    message: String,
    is_error: bool,
}

#[derive(Debug, PartialEq)]
enum Action {
    Enable(bool),
    ToggleSelection(String),
    Recover,
}
impl Action {
    fn parse(id: &str) -> Option<Self> {
        match id {
            "enable" => Some(Self::Enable(true)),
            "disable" => Some(Self::Enable(false)),
            "recover" => Some(Self::Recover),
            _ => id
                .strip_prefix(PROFILE_PREFIX)
                .filter(|id| !id.is_empty())
                .map(|id| Self::ToggleSelection(id.to_owned())),
        }
    }
}

#[derive(Debug)]
struct ProfileItem {
    id: String,
    label: String,
    checked: bool,
}

#[derive(Clone)]
struct LiveMenu {
    enabled: bool,
    profiles: Vec<(String, String)>,
    status: MenuItem<tauri::Wry>,
    toggle: MenuItem<tauri::Wry>,
    checks: Vec<CheckMenuItem<tauri::Wry>>,
}

// Keep menu identities stable while selecting; replacing the native menu ends tracking.
#[derive(Default)]
struct MenuState(Mutex<Option<LiveMenu>>);

// Keep the tray and homepage selection in sync, including while the service is off.
fn profile_items(profiles: &[Profile], selected: &[String]) -> Vec<ProfileItem> {
    profiles
        .iter()
        .map(|p| ProfileItem {
            id: format!("{PROFILE_PREFIX}{}", p.id),
            label: format!("{} · {} 个模型", p.name, p.models.len()),
            checked: selected.contains(&p.id),
        })
        .collect()
}

fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// All native quit entries use the same restore-and-exit command as Settings.
pub fn request_quit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let s = app.state::<AppState>();
        if let Err(message) = crate::commands::quit(app.clone(), s.clone()).await {
            *s.tray_feedback.lock().unwrap_or_else(|e| e.into_inner()) = Some(Feedback {
                message,
                is_error: true,
            });
            let _ = app.emit("tray-state-changed", ());
            show(&app);
        }
    });
}

pub fn sync(app: &AppHandle) -> CommandResult<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let s = app.state::<AppState>();
    let enabled = s.config.enabled().map_err(err)?;
    let (profiles, selected) = {
        let store = s.store.lock().map_err(err)?;
        (
            store.profiles().map_err(err)?,
            store.selected_profiles().map_err(err)?,
        )
    };
    let pending = *s.pending.lock().map_err(err)? && crate::platform::app_info().running;
    let status = if pending {
        if enabled {
            "已开启 · 模型列表待刷新"
        } else {
            "已关闭 · 需重新打开 Codex 生效"
        }
    } else if enabled {
        "Codex Switch 已开启"
    } else {
        "Codex Switch 已关闭"
    };
    let items = profile_items(&profiles, &selected);
    let identity = items
        .iter()
        .map(|p| (p.id.clone(), p.label.clone()))
        .collect::<Vec<_>>();
    let cached = app.state::<MenuState>().0.lock().map_err(err)?.clone();
    if let Some(cached) = cached.filter(|m| m.enabled == enabled && m.profiles == identity) {
        let status_changed = cached.status.text().map_err(err)? != status;
        if status_changed {
            cached.status.set_text(status).map_err(err)?;
        }
        cached
            .toggle
            .set_enabled(enabled || !selected.is_empty())
            .map_err(err)?;
        for (check, item) in cached.checks.iter().zip(&items) {
            check.set_checked(item.checked).map_err(err)?;
            check.set_enabled(!enabled).map_err(err)?;
        }
        #[cfg(target_os = "macos")]
        persistent_menu::sync(app, &tray, &identity)?;
        // A selection-triggered frontend snapshot must not resize the status item
        // while the user is still tracking its menu.
        if status_changed {
            tray.set_tooltip(Some(status)).map_err(err)?;
        }
        return Ok(());
    }
    let menu = Menu::new(app).map_err(err)?;
    let status_item = MenuItem::with_id(app, "status", status, false, None::<&str>).map_err(err)?;
    menu.append(&status_item).map_err(err)?;
    let submenu =
        Submenu::new(app, "选择配置（可多选）", !enabled && !profiles.is_empty()).map_err(err)?;
    let mut checks = Vec::with_capacity(items.len());
    for item in items {
        let can_toggle = !enabled;
        let check = CheckMenuItem::with_id(
            app,
            item.id,
            item.label,
            can_toggle,
            item.checked,
            None::<&str>,
        )
        .map_err(err)?;
        submenu.append(&check).map_err(err)?;
        checks.push(check);
    }
    menu.append(&submenu).map_err(err)?;
    let toggle = MenuItem::with_id(
        app,
        if enabled { "disable" } else { "enable" },
        if enabled {
            "关闭并恢复原配置"
        } else {
            "开启 Codex Switch"
        },
        enabled || !selected.is_empty(),
        None::<&str>,
    )
    .map_err(err)?;
    menu.append(&toggle).map_err(err)?;
    if profiles.is_empty() {
        menu.append(
            &MenuItem::with_id(app, "empty", "请先在主界面添加配置", false, None::<&str>)
                .map_err(err)?,
        )
        .map_err(err)?;
    }
    if enabled {
        menu.append(
            &MenuItem::with_id(app, "recover", "恢复后台连接", true, None::<&str>).map_err(err)?,
        )
        .map_err(err)?;
    }
    menu.append(&PredefinedMenuItem::separator(app).map_err(err)?)
        .map_err(err)?;
    menu.append(&MenuItem::with_id(app, "show", "打开面板", true, None::<&str>).map_err(err)?)
        .map_err(err)?;
    menu.append(&MenuItem::with_id(app, "quit", "退出应用", true, None::<&str>).map_err(err)?)
        .map_err(err)?;
    tray.set_menu(Some(menu)).map_err(err)?;
    #[cfg(target_os = "macos")]
    persistent_menu::sync(app, &tray, &identity)?;
    *app.state::<MenuState>().0.lock().map_err(err)? = Some(LiveMenu {
        enabled,
        profiles: identity,
        status: status_item,
        toggle,
        checks,
    });
    tray.set_tooltip(Some(status)).map_err(err)
}

fn toggle_selection_inner(s: &AppState, id: &str) -> CommandResult<String> {
    let mut selected = s
        .store
        .lock()
        .map_err(err)?
        .selected_profiles()
        .map_err(err)?;
    if selected.iter().any(|selected_id| selected_id == id) {
        selected.retain(|selected_id| selected_id != id);
    } else {
        selected.push(id.to_owned());
    }
    select_profiles_inner(s, &selected)?;
    Ok("已更新所选配置。".into())
}

// Native checkbox actions execute during AppKit menu tracking. Passing them through
// Tauri's event loop would defer all clicks until dismissal, losing rapid toggles.
#[cfg(target_os = "macos")]
fn select_from_native_menu(app: &AppHandle, id: &str) -> Option<bool> {
    let s = app.state::<AppState>();
    let result = (|| {
        let _op = s
            .operation
            .try_lock()
            .map_err(|_| "正在处理其他操作，请稍后再试。".to_string())?;
        toggle_selection_inner(&s, id)
    })();
    // During tracking, update only selection state. Updating status-bar geometry
    // (including its tooltip) can dismiss the active menu on macOS.
    let result = result.and_then(|message| {
        let selected = s
            .store
            .lock()
            .map_err(err)?
            .selected_profiles()
            .map_err(err)?;
        let cached = app.state::<MenuState>().0.lock().map_err(err)?.clone();
        if let Some(cached) = cached {
            for check in &cached.checks {
                let id = check
                    .id()
                    .as_ref()
                    .strip_prefix(PROFILE_PREFIX)
                    .unwrap_or_default();
                check
                    .set_checked(selected.iter().any(|p| p == id))
                    .map_err(err)?;
            }
            cached
                .toggle
                .set_enabled(!selected.is_empty())
                .map_err(err)?;
        }
        Ok(message)
    });
    let failed = result.is_err();
    *s.tray_feedback.lock().unwrap_or_else(|e| e.into_inner()) = Some(match result {
        Ok(message) => Feedback {
            message,
            is_error: false,
        },
        Err(message) => Feedback {
            message,
            is_error: true,
        },
    });
    let _ = app.emit("tray-state-changed", ());
    if failed {
        show(app);
    }
    s.store
        .lock()
        .ok()
        .and_then(|store| store.selected_profiles().ok())
        .map(|ids| ids.iter().any(|selected| selected == id))
}

async fn execute(s: &AppState, action: Action) -> CommandResult<String> {
    // Reject clicks during validation/application instead of queuing surprising later changes.
    let _op = s
        .operation
        .try_lock()
        .map_err(|_| "正在处理其他操作，请稍后再试。".to_string())?;
    match action {
        Action::Enable(enabled) => {
            set_enabled_inner(s, enabled).await?;
            Ok(if enabled {
                "Codex Switch 已开启。"
            } else {
                "已关闭并恢复开启前的配置。"
            }
            .into())
        }
        Action::ToggleSelection(id) => toggle_selection_inner(s, &id),
        Action::Recover => {
            if !s.config.enabled().map_err(err)? {
                return Err("配置已关闭，请先开启。".into());
            }
            enable_inner(s).await?;
            Ok("后台连接已恢复。".into())
        }
    }
}

fn finish_action(app: &AppHandle, result: CommandResult<String>) {
    let s = app.state::<AppState>();
    let mut feedback = match result {
        Ok(message) => Feedback {
            message,
            is_error: false,
        },
        Err(message) => Feedback {
            message,
            is_error: true,
        },
    };
    if !feedback.is_error
        && *s.pending.lock().unwrap_or_else(|e| e.into_inner())
        && crate::platform::app_info().running
    {
        feedback
            .message
            .push_str(if s.config.enabled().unwrap_or(false) {
                "后续请求已使用当前配置；重新打开 Codex 可刷新模型列表。"
            } else {
                "请在当前任务结束后重新打开 Codex，恢复原设置。"
            });
    }
    if let Err(message) = sync(app) {
        feedback
            .message
            .push_str(&format!("托盘状态刷新失败：{message}"));
        feedback.is_error = true;
    }
    let failed = feedback.is_error;
    *s.tray_feedback.lock().unwrap_or_else(|e| e.into_inner()) = Some(feedback);
    let _ = app.emit("tray-state-changed", ());
    if failed {
        show(app);
    }
}

pub fn install(app: &AppHandle) -> CommandResult<()> {
    app.manage(MenuState::default());
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Codex Switch")
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                request_quit(app);
                return;
            }
            if event.id.as_ref() == "show" {
                show(app);
                return;
            }
            let Some(action) = Action::parse(event.id.as_ref()) else {
                return;
            };
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let s = app.state::<AppState>();
                let result = execute(&s, action).await;
                finish_action(&app, result);
            });
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app).map_err(err)?;
    sync(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_switch_core::providers::Protocol;

    fn profile(id: &str) -> Profile {
        Profile {
            id: id.into(),
            name: id.into(),
            preset_id: "kimi".into(),
            endpoint: "https://example.com/v1".into(),
            models: vec!["m1".into(), "m2".into()],
            protocol: Protocol::Chat,
            context_window: 32768,
            options: Default::default(),
            credential_id: format!("key-{id}"),
            route_ids: vec![format!("{id}-1"), format!("{id}-2")],
        }
    }
    #[test]
    fn routes_menu_ids_without_using_user_supplied_names_as_actions() {
        assert_eq!(
            Action::parse("profile:disable"),
            Some(Action::ToggleSelection("disable".into()))
        );
        assert_eq!(Action::parse("disable"), Some(Action::Enable(false)));
        assert_eq!(Action::parse("enable"), Some(Action::Enable(true)));
        assert_eq!(Action::parse("recover"), Some(Action::Recover));
        assert_eq!(Action::parse("profile:"), None);
        assert_eq!(Action::parse("status"), None);
        assert_eq!(Action::parse("quit"), None);
        assert_eq!(
            Action::parse("profile:quit"),
            Some(Action::ToggleSelection("quit".into()))
        );
    }
    #[test]
    fn checks_all_selected_profiles_independently_of_applied_routes() {
        let profiles = vec![profile("a"), profile("b"), profile("c")];
        let items = profile_items(&profiles, &["a".into(), "b".into()]);
        assert!(items[0].checked);
        assert!(items[1].checked);
        assert!(!items[2].checked);
        assert_eq!(items[0].label, "a · 2 个模型");
        assert!(profile_items(&profiles, &[]).iter().all(|p| !p.checked));
    }

    #[tokio::test]
    async fn selecting_multiple_profiles_while_disabled_never_activates() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join("codex");
        let state_dir = dir.path().join("state");
        std::fs::create_dir(&config_dir).unwrap();
        let original = "# Keep the user's original configuration\nmodel = \"original\"\n";
        std::fs::write(config_dir.join("config.toml"), original).unwrap();
        let store = crate::Store::memory().unwrap();
        store.save_profile(&profile("a")).unwrap();
        store.save_profile(&profile("b")).unwrap();
        let s = AppState {
            store: std::sync::Mutex::new(store),
            config: crate::ConfigManager::new(config_dir, state_dir.clone()),
            runtime: tokio::sync::Mutex::new(None),
            operation: tokio::sync::Mutex::new(()),
            pending: std::sync::Mutex::new(false),
            tray_feedback: std::sync::Mutex::new(None),
            checks: std::sync::Mutex::new(Default::default()),
        };
        for (id, expected) in [
            ("a", vec!["a"]),
            ("b", vec!["a", "b"]),
            ("a", vec!["b"]),
            ("b", vec![]),
        ] {
            execute(&s, Action::ToggleSelection(id.into()))
                .await
                .unwrap();
            assert_eq!(
                s.store.lock().unwrap().selected_profiles().unwrap(),
                expected
            );
            assert!(!s.config.enabled().unwrap());
            assert!(s.runtime.lock().await.is_none());
            assert!(s.store.lock().unwrap().applied_routes().unwrap().is_empty());
            assert_eq!(std::fs::read_to_string(s.config.path()).unwrap(), original);
            assert!(
                !state_dir.exists(),
                "selection must not create activation state"
            );
        }
        assert!(execute(&s, Action::ToggleSelection("missing".into()))
            .await
            .is_err());
        assert!(s
            .store
            .lock()
            .unwrap()
            .selected_profiles()
            .unwrap()
            .is_empty());

        // Enabling via a temporary configuration must lock tray and command selection alike.
        execute(&s, Action::ToggleSelection("a".into()))
            .await
            .unwrap();
        s.config
            .enable(
                "http://127.0.0.1:12345",
                "synthetic",
                "m1",
                &dir.path().join("catalog.json"),
            )
            .unwrap();
        let active_config = std::fs::read(s.config.path()).unwrap();
        for id in ["a", "b"] {
            let error = execute(&s, Action::ToggleSelection(id.into()))
                .await
                .unwrap_err();
            assert!(error.contains("请先关闭"));
        }
        assert!(select_profiles_inner(&s, &["b".into()])
            .unwrap_err()
            .contains("请先关闭"));
        assert_eq!(
            s.store.lock().unwrap().selected_profiles().unwrap(),
            vec!["a"]
        );
        assert_eq!(std::fs::read(s.config.path()).unwrap(), active_config);
        s.config.disable(true).unwrap();
        execute(&s, Action::ToggleSelection("b".into()))
            .await
            .unwrap();
        assert_eq!(
            s.store.lock().unwrap().selected_profiles().unwrap(),
            vec!["a", "b"]
        );
    }
}
