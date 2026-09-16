#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(target_os = "macos")]
mod app_menu;
mod platform;
mod tray;
use codex_switch_core::{
    config::ConfigManager,
    connections::{persist_connection, Credentials, UnavailableConnection},
    gateway::{probe, Gateway, Route},
    profiles::{ensure_editable, ensure_selection, prepare_profile, Profile},
    providers::{presets, Preset, RoutingSettings},
    store::Store,
};
use platform::Vault;
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::{Manager, State};
use tauri_plugin_autostart::ManagerExt;

struct AppState {
    store: Mutex<Store>,
    config: ConfigManager,
    runtime: tokio::sync::Mutex<Option<Gateway>>,
    operation: tokio::sync::Mutex<()>,
    pending: Mutex<bool>,
    tray_feedback: Mutex<Option<tray::Feedback>>,
    unavailable: Mutex<Vec<UnavailableConnection>>,
    checks: Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
}
type CommandResult<T> = Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    profiles: Vec<Profile>,
    presets: Vec<Preset>,
    selected_profiles: Vec<String>,
    needs_apply: bool,
    enabled: bool,
    routing: bool,
    pending_reload: bool,
    active_requests: usize,
    app: platform::AppInfo,
    config_path: String,
    autostart: bool,
    routing_settings: RoutingSettings,
    unavailable: Vec<UnavailableConnection>,
    tray_feedback: Option<tray::Feedback>,
}
#[tauri::command]
async fn snapshot(app: tauri::AppHandle, s: State<'_, AppState>) -> CommandResult<Snapshot> {
    let mut runtime = s.runtime.lock().await;
    let app_info = platform::app_info();
    if !app_info.running
        && !s.config.enabled().map_err(err)?
        && runtime.as_ref().is_none_or(|g| g.requests() == 0)
    {
        runtime.take();
        let _ = s.config.prune_catalogs();
    }
    let store = s.store.lock().map_err(err)?;
    let result = Snapshot {
        profiles: store.profiles().map_err(err)?,
        presets: presets(),
        selected_profiles: store.selected_profiles().map_err(err)?,
        needs_apply: serde_json::to_string(&store.selected_routes().map_err(err)?).map_err(err)?
            != serde_json::to_string(&store.applied_routes().map_err(err)?).map_err(err)?,
        enabled: s.config.enabled().map_err(err)?,
        routing: runtime.is_some(),
        pending_reload: *s.pending.lock().map_err(err)? && app_info.running,
        active_requests: runtime.as_ref().map(Gateway::requests).unwrap_or(0),
        app: app_info,
        config_path: s.config.path().display().to_string(),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        routing_settings: routing_settings(&store)?,
        unavailable: s.unavailable.lock().map_err(err)?.clone(),
        tray_feedback: s.tray_feedback.lock().map_err(err)?.take(),
    };
    drop(store);
    drop(runtime);
    tray::sync(&app)?;
    Ok(result)
}
fn routing_settings(store: &Store) -> CommandResult<RoutingSettings> {
    store
        .setting("routing_settings")
        .map_err(err)?
        .map(|v| serde_json::from_str(&v).map_err(err))
        .transpose()
        .map(|v| v.unwrap_or_default())
}
#[tauri::command]
async fn save_routing_settings(
    s: State<'_, AppState>,
    settings: RoutingSettings,
) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    let store = s.store.lock().map_err(err)?;
    let profiles = store.profiles().map_err(err)?;
    let mut seen = std::collections::HashSet::new();
    if settings.fallback_profiles.len() > 8
        || settings
            .fallback_profiles
            .iter()
            .any(|id| !seen.insert(id) || !profiles.iter().any(|p| &p.id == id))
    {
        return Err("备用队列包含无效或重复的配置，最多 8 个。".into());
    }
    if settings.failover_enabled && settings.fallback_profiles.is_empty() {
        return Err("请先添加备用队列。".into());
    }
    store
        .set(
            "routing_settings",
            &serde_json::to_string(&settings).map_err(err)?,
        )
        .map_err(err)
}
#[tauri::command]
async fn probe_endpoint(
    s: State<'_, AppState>,
    profile: Profile,
    key: String,
    request_id: String,
) -> CommandResult<codex_switch_core::gateway::ProbeReceipt> {
    let _op = s.operation.lock().await;
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    uuid::Uuid::parse_str(&request_id).map_err(|_| "测试标识无效。")?;
    let cancel = s
        .checks
        .lock()
        .map_err(err)?
        .entry(request_id.clone())
        .or_default()
        .clone();
    let result = async {
        let existing = s.store.lock().map_err(err)?.profiles().map_err(err)?;
        let (profile, key) = prepare_profile(profile, &existing, &key, &Vault).map_err(err)?;
        let started = std::time::Instant::now();
        for route in profile.routes() {
            let mut receipt = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err("配置测试已取消。".to_owned()),
                receipt = probe(&route, &key) => receipt,
            };
            if !receipt.ok {
                receipt.message = format!("{}：{}", route.model, receipt.message);
                return Ok(receipt);
            }
        }
        Ok(codex_switch_core::gateway::ProbeReceipt {
            ok: true,
            status: Some(200),
            message: format!("配置测试通过，{} 个模型请求成功。", profile.models.len()),
            elapsed_ms: started.elapsed().as_millis(),
        })
    }
    .await;
    s.checks.lock().map_err(err)?.remove(&request_id);
    result
}
#[tauri::command]
async fn save_profile(
    s: State<'_, AppState>,
    profile: Profile,
    key: String,
) -> CommandResult<Profile> {
    let _op = s.operation.lock().await;
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    let existing = s.store.lock().map_err(err)?.profiles().map_err(err)?;
    let (profile, resolved) = prepare_profile(profile, &existing, &key, &Vault).map_err(err)?;
    let is_new = existing
        .iter()
        .all(|p| p.credential_id != profile.credential_id);
    // Save only validates local fields and persists credentials/metadata. Network
    // requests are exclusively triggered by the explicit probe_endpoint command.
    let mut credential = profile.routes()[0].clone();
    credential.id = profile.credential_id.clone();
    persist_connection(&credential, &resolved, is_new, &Vault, || {
        s.store
            .lock()
            .map_err(|_| codex_switch_core::message("存储不可用。"))?
            .save_profile(&profile)
    })
    .map_err(err)?;
    Ok(profile)
}
#[tauri::command]
fn cancel_validation(s: State<'_, AppState>, request_id: String) -> CommandResult<()> {
    uuid::Uuid::parse_str(&request_id).map_err(|_| "验证标识无效。")?;
    let mut checks = s.checks.lock().map_err(err)?;
    if checks.len() > 16 {
        return Err("验证请求过多。".into());
    }
    checks.entry(request_id).or_default().cancel();
    Ok(())
}
async fn enable_inner(s: &AppState) -> CommandResult<()> {
    enable_profiles(s, None).await
}
async fn enable_profiles(s: &AppState, profile_ids: Option<&[String]>) -> CommandResult<()> {
    let (connections, mut aliases) = {
        let store = s.store.lock().map_err(err)?;
        let connections = if let Some(ids) = profile_ids {
            store.routes_for_profiles(ids)
        } else {
            store.selected_routes()
        }
        .map_err(err)?;
        let aliases = store.aliases_for_routes(&connections).map_err(err)?;
        (connections, aliases)
    };
    if connections.is_empty() {
        return Err("请至少勾选一个配置。".into());
    }
    let mut routes = vec![];
    for c in &connections {
        let credential = s
            .store
            .lock()
            .map_err(err)?
            .credential_for(&c.id)
            .map_err(err)?;
        let key = Vault
            .read(&credential)
            .map_err(|_| format!("{} 的密钥不可用，请编辑配置重新填写。", c.name))?;
        if key.trim().is_empty() {
            return Err(format!("{} 的密钥为空。", c.name));
        }
        routes.push(Route {
            connection: c.clone(),
            key,
            aliases: aliases.remove(&c.id).unwrap_or_default(),
        });
    }
    let id = connections[0].id.clone();
    let running = platform::app_info().running;
    let mut runtime = s.runtime.lock().await;
    let port = s
        .store
        .lock()
        .map_err(err)?
        .setting("port")
        .map_err(err)?
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(0);
    let settings = routing_settings(&*s.store.lock().map_err(err)?)?;
    let mut fallback = vec![];
    if settings.failover_enabled {
        let store = s.store.lock().map_err(err)?;
        let profiles = store.profiles().map_err(err)?;
        for id in &settings.fallback_profiles {
            let profile = profiles
                .iter()
                .find(|p| &p.id == id)
                .ok_or("备用配置已删除，请更新通用设置。")?;
            let connection = profile
                .routes()
                .into_iter()
                .next()
                .ok_or("备用配置缺少模型。")?;
            let key = Vault.read(&profile.credential_id).map_err(err)?;
            if key.trim().is_empty() {
                return Err("备用配置的密钥为空。".into());
            }
            fallback.push(Route {
                connection,
                key,
                aliases: vec![],
            });
        }
        if fallback.is_empty() {
            return Err("备用队列为空，请更新通用设置。".into());
        }
    }
    codex_switch_core::activation::activate_with_options(
        &s.config,
        &mut runtime,
        routes,
        &id,
        codex_switch_core::activation::ActivationOptions {
            can_prune: !running,
            remote_compaction: settings.remote_compaction,
        },
        |routes| Gateway::start(routes, port, uuid::Uuid::new_v4().simple().to_string()),
        |port| {
            let store = s
                .store
                .lock()
                .map_err(|_| codex_switch_core::message("连接存储不可用。"))?;
            if let Some(ids) = profile_ids {
                store.commit_profile_switch(&connections, port, ids)
            } else {
                store.commit_profiles(&connections, port)
            }
        },
    )
    .await
    .map_err(err)?;
    if let Some(gateway) = runtime.as_ref() {
        gateway.state.set_failover(fallback).await;
    }
    *s.pending.lock().unwrap_or_else(|e| e.into_inner()) = running;
    Ok(())
}

#[tauri::command]
async fn set_enabled(s: State<'_, AppState>, enabled: bool, force: bool) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    set_enabled_inner(&s, enabled, force).await
}
async fn set_enabled_inner(s: &AppState, enabled: bool, force: bool) -> CommandResult<()> {
    if enabled {
        enable_inner(s).await
    } else {
        s.config.disable(force).map_err(err)?;
        let running = platform::app_info().running;
        *s.pending.lock().map_err(err)? = running;
        if !running {
            s.runtime.lock().await.take();
        }
        Ok(())
    }
}
#[tauri::command]
async fn select_profiles(s: State<'_, AppState>, ids: Vec<String>) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    let enabled = s.config.enabled().map_err(err)?;
    ensure_selection(enabled, &ids).map_err(err)?;
    if enabled {
        enable_profiles(&s, Some(&ids)).await
    } else {
        s.store
            .lock()
            .map_err(err)?
            .select_profiles(&ids)
            .map_err(err)
    }
}
#[tauri::command]
async fn delete_profile(s: State<'_, AppState>, id: String) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    s.store
        .lock()
        .map_err(err)?
        .delete_profile(&id)
        .map_err(err)
}
#[tauri::command]
async fn recover_connection(s: State<'_, AppState>) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    if !s.config.enabled().map_err(err)? {
        return Err("配置已关闭，请先开启。".into());
    }
    enable_inner(&s).await
}
#[tauri::command]
async fn restart_codex(s: State<'_, AppState>) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    if s.config.enabled().map_err(err)? && s.runtime.lock().await.is_none() {
        enable_inner(&s).await?;
    }
    if s.runtime
        .lock()
        .await
        .as_ref()
        .is_some_and(|g| g.requests() > 0)
    {
        return Err("还有请求正在进行，请等待当前任务结束后再重启。".into());
    }
    platform::close_codex().await?;
    if !s.config.enabled().map_err(err)? {
        s.runtime.lock().await.take();
    }
    platform::open_codex()?;
    *s.pending.lock().map_err(err)? = false;
    Ok(())
}
#[tauri::command]
async fn open_codex(s: State<'_, AppState>) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    if s.config.enabled().map_err(err)? && s.runtime.lock().await.is_none() {
        enable_inner(&s).await?;
    }
    if *s.pending.lock().map_err(err)? && platform::app_info().running {
        return Err(
            "请先在 Codex 中完成任务并正常退出，再点击打开。原有任务不会被强制结束。".into(),
        );
    }
    if !s.config.enabled().map_err(err)? {
        s.runtime.lock().await.take();
    }
    platform::open_codex()?;
    *s.pending.lock().map_err(err)? = false;
    Ok(())
}
#[tauri::command]
fn open_link(url: String) -> CommandResult<()> {
    platform::open_url(&url)
}
#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> CommandResult<()> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|_| "无法设置登录时启动。".into())
}
#[tauri::command]
async fn quit(app: tauri::AppHandle, s: State<'_, AppState>) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    if platform::app_info().running
        && (s.config.enabled().map_err(err)? || s.runtime.lock().await.is_some())
    {
        return Err("请先退出 Codex；关闭本窗口会继续在后台提供连接。".into());
    }
    s.config.disable(false).map_err(err)?;
    s.runtime.lock().await.take();
    app.exit(0);
    Ok(())
}

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
            #[cfg(target_os = "macos")]
            app_menu::hide_edit_actions()?;
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
                unavailable: Mutex::new(vec![]),
                checks: Mutex::new(std::collections::HashMap::new()),
            });
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
            snapshot,
            cancel_validation,
            save_profile,
            save_routing_settings,
            probe_endpoint,
            set_enabled,
            select_profiles,
            delete_profile,
            open_codex,
            restart_codex,
            recover_connection,
            open_link,
            set_autostart,
            quit
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
