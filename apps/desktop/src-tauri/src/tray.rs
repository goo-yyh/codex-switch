use crate::{enable_inner, enable_profiles, err, set_enabled_inner, AppState, CommandResult};
use codex_switch_core::{profiles::Profile, providers::Connection};
use serde::Serialize;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

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
    Switch(String),
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
                .map(|id| Self::Switch(id.to_owned())),
        }
    }
}

#[derive(Debug)]
struct ProfileItem {
    id: String,
    label: String,
    checked: bool,
}

// Derive checks from the applied snapshot, never from the homepage's draft checkboxes.
fn profile_items(profiles: &[Profile], applied: &[Connection], enabled: bool) -> Vec<ProfileItem> {
    profiles
        .iter()
        .map(|p| ProfileItem {
            id: format!("{PROFILE_PREFIX}{}", p.id),
            label: format!("{} · {} 个模型", p.name, p.models.len()),
            checked: enabled
                && !p.models.is_empty()
                && p.routes().iter().all(|route| {
                    applied.iter().any(|old| {
                        serde_json::to_value(old).ok() == serde_json::to_value(route).ok()
                    })
                }),
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
        if let Err(message) = crate::quit(app.clone(), s.clone()).await {
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
    let (profiles, applied, selected) = {
        let store = s.store.lock().map_err(err)?;
        (
            store.profiles().map_err(err)?,
            store.applied_routes().map_err(err)?,
            store.selected_profiles().map_err(err)?,
        )
    };
    let pending = *s.pending.lock().map_err(err)? && crate::platform::app_info().running;
    let menu = Menu::new(app).map_err(err)?;
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
    menu.append(&MenuItem::with_id(app, "status", status, false, None::<&str>).map_err(err)?)
        .map_err(err)?;
    menu.append(
        &MenuItem::with_id(
            app,
            if enabled { "disable" } else { "enable" },
            if enabled {
                "关闭并恢复原配置"
            } else {
                "开启所选配置"
            },
            enabled || !selected.is_empty(),
            None::<&str>,
        )
        .map_err(err)?,
    )
    .map_err(err)?;
    let submenu = Submenu::new(app, "切换并开启配置（单选）", !profiles.is_empty()).map_err(err)?;
    for item in profile_items(&profiles, &applied, enabled) {
        submenu
            .append(
                &CheckMenuItem::with_id(app, item.id, item.label, true, item.checked, None::<&str>)
                    .map_err(err)?,
            )
            .map_err(err)?;
    }
    menu.append(&submenu).map_err(err)?;
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
    menu.append(
        &MenuItem::with_id(app, "show", "打开 Codex Switch…", true, None::<&str>).map_err(err)?,
    )
    .map_err(err)?;
    menu.append(
        &MenuItem::with_id(app, "quit", "退出 Codex Switch", true, None::<&str>).map_err(err)?,
    )
    .map_err(err)?;
    tray.set_menu(Some(menu)).map_err(err)?;
    tray.set_tooltip(Some(status)).map_err(err)
}

async fn execute(s: &AppState, action: Action) -> CommandResult<String> {
    // Reject clicks during validation/application instead of queuing surprising later changes.
    let _op = s
        .operation
        .try_lock()
        .map_err(|_| "正在处理其他操作，请稍后再试。".to_string())?;
    match action {
        Action::Enable(enabled) => {
            set_enabled_inner(s, enabled, false).await?;
            Ok(if enabled {
                "Codex Switch 已开启。"
            } else {
                "已关闭并恢复开启前的配置。"
            }
            .into())
        }
        Action::Switch(id) => {
            enable_profiles(s, Some(&[id])).await?;
            Ok("已切换配置，并应用其中的全部模型。".into())
        }
        Action::Recover => {
            if !s.config.enabled().map_err(err)? {
                return Err("配置已关闭，请先开启。".into());
            }
            enable_inner(s).await?;
            Ok("后台连接已恢复。".into())
        }
    }
}

pub fn install(app: &AppHandle) -> CommandResult<()> {
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
                if let Err(message) = sync(&app) {
                    feedback
                        .message
                        .push_str(&format!("托盘状态刷新失败：{message}"));
                    feedback.is_error = true;
                }
                let failed = feedback.is_error;
                *s.tray_feedback.lock().unwrap_or_else(|e| e.into_inner()) = Some(feedback);
                let _ = app.emit("tray-state-changed", ());
                if failed {
                    show(&app);
                }
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
            Some(Action::Switch("disable".into()))
        );
        assert_eq!(Action::parse("disable"), Some(Action::Enable(false)));
        assert_eq!(Action::parse("enable"), Some(Action::Enable(true)));
        assert_eq!(Action::parse("recover"), Some(Action::Recover));
        assert_eq!(Action::parse("profile:"), None);
        assert_eq!(Action::parse("status"), None);
        assert_eq!(Action::parse("quit"), None);
        assert_eq!(
            Action::parse("profile:quit"),
            Some(Action::Switch("quit".into()))
        );
    }
    #[test]
    fn checks_only_current_applied_revisions_and_all_models() {
        let a = profile("a");
        let b = profile("b");
        let mut applied = a.routes();
        let profiles = vec![a.clone(), b.clone()];
        let items = profile_items(&profiles, &applied, true);
        assert!(items[0].checked);
        assert!(!items[1].checked);
        assert_eq!(items[0].label, "a · 2 个模型");
        assert!(profile_items(&profiles, &applied, false)
            .iter()
            .all(|p| !p.checked));
        applied.pop();
        assert!(!profile_items(&profiles, &applied, true)[0].checked);
        applied.extend(b.routes());
        assert!(profile_items(&profiles, &applied, true)[1].checked);
        let mut edited = b;
        edited.name = "New name".into();
        assert!(!profile_items(&[edited], &applied, true)[0].checked);
    }
}
