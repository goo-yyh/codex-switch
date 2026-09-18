//! Tauri IPC boundary: validate input and hold the operation lock before mutations.
use crate::{
    locale::Locale,
    model_registry::RegistryState,
    platform::{self, Vault},
    service::{enable_inner, routing_settings, select_profiles_inner, set_enabled_inner},
    state::{err, AppState, CommandResult},
    tray,
};
use codex_switch_core::{
    connections::persist_connection,
    gateway::{probe, Gateway},
    profiles::{ensure_editable, prepare_profile, Profile},
    providers::{Preset, RoutingSettings},
};
use serde::Serialize;
use tauri::State;
use tauri_plugin_autostart::ManagerExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    locale: Locale,
    profiles: Vec<Profile>,
    presets: Vec<Preset>,
    model_candidates: std::collections::BTreeMap<String, Vec<String>>,
    selected_profiles: Vec<String>,
    enabled: bool,
    routing: bool,
    pending_reload: bool,
    active_requests: usize,
    app: platform::AppInfo,
    config_path: String,
    autostart: bool,
    routing_settings: RoutingSettings,
    tray_feedback: Option<tray::Feedback>,
}
#[tauri::command]
pub(crate) async fn snapshot(
    app: tauri::AppHandle,
    s: State<'_, AppState>,
    registry: State<'_, RegistryState>,
) -> CommandResult<Snapshot> {
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
    let catalog = registry.0.lock().map_err(err)?.clone();
    let result = Snapshot {
        locale: Locale::read(&store).map_err(err)?,
        profiles: store.profiles().map_err(err)?,
        presets: catalog.presets,
        model_candidates: catalog.candidates,
        selected_profiles: store.selected_profiles().map_err(err)?,
        enabled: s.config.enabled().map_err(err)?,
        routing: runtime.is_some(),
        pending_reload: *s.pending.lock().map_err(err)? && app_info.running,
        active_requests: runtime.as_ref().map(Gateway::requests).unwrap_or(0),
        app: app_info,
        config_path: s.config.path().display().to_string(),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        routing_settings: routing_settings(&store)?,
        tray_feedback: s.tray_feedback.lock().map_err(err)?.take(),
    };
    drop(store);
    drop(runtime);
    tray::sync(&app)?;
    Ok(result)
}

#[tauri::command]
pub(crate) async fn save_routing_settings(
    s: State<'_, AppState>,
    settings: RoutingSettings,
) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    let store = s.store.lock().map_err(err)?;
    store
        .set(
            "routing_settings",
            &serde_json::to_string(&settings).map_err(err)?,
        )
        .map_err(err)
}
#[tauri::command]
pub(crate) async fn probe_endpoint(
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
pub(crate) async fn save_profile(
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
    persist_connection(&profile.credential_id, &resolved, is_new, &Vault, || {
        s.store
            .lock()
            .map_err(|_| codex_switch_core::message("存储不可用。"))?
            .save_profile(&profile)
    })
    .map_err(err)?;
    Ok(profile)
}
#[tauri::command]
pub(crate) fn cancel_validation(s: State<'_, AppState>, request_id: String) -> CommandResult<()> {
    uuid::Uuid::parse_str(&request_id).map_err(|_| "验证标识无效。")?;
    let mut checks = s.checks.lock().map_err(err)?;
    if checks.len() > 16 {
        return Err("验证请求过多。".into());
    }
    checks.entry(request_id).or_default().cancel();
    Ok(())
}

#[tauri::command]
pub(crate) async fn set_enabled(s: State<'_, AppState>, enabled: bool) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    set_enabled_inner(&s, enabled).await
}

#[tauri::command]
pub(crate) async fn select_profiles(s: State<'_, AppState>, ids: Vec<String>) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    select_profiles_inner(&s, &ids)
}

#[tauri::command]
pub(crate) async fn delete_profile(s: State<'_, AppState>, id: String) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    s.store
        .lock()
        .map_err(err)?
        .delete_profile(&id)
        .map_err(err)
}
#[tauri::command]
pub(crate) async fn restart_codex(s: State<'_, AppState>) -> CommandResult<()> {
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
pub(crate) async fn open_codex(s: State<'_, AppState>) -> CommandResult<()> {
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
pub(crate) fn open_link(url: String) -> CommandResult<()> {
    platform::open_url(&url)
}
#[tauri::command]
pub(crate) async fn set_autostart(
    app: tauri::AppHandle,
    s: State<'_, AppState>,
    enabled: bool,
) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|_| "无法设置登录时启动。".into())
}
#[tauri::command]
pub(crate) async fn quit(app: tauri::AppHandle, s: State<'_, AppState>) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    if platform::app_info().running
        && (s.config.enabled().map_err(err)? || s.runtime.lock().await.is_some())
    {
        return Err("请先退出 Codex；关闭本窗口会继续在后台提供连接。".into());
    }
    s.config.disable(true).map_err(err)?;
    s.runtime.lock().await.take();
    app.exit(0);
    Ok(())
}

/// Persist before reporting success so the panel and native menus share one preference.
#[tauri::command]
pub(crate) async fn set_locale(
    app: tauri::AppHandle,
    s: State<'_, AppState>,
    locale: Locale,
) -> CommandResult<()> {
    let _op = s.operation.lock().await;
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    s.store
        .lock()
        .map_err(err)?
        .set("locale", locale.code())
        .map_err(err)?;
    tray::sync(&app)?;
    #[cfg(target_os = "macos")]
    {
        let (send, receive) = tokio::sync::oneshot::channel();
        let handle = app.clone();
        app.run_on_main_thread(move || {
            let result = (|| -> CommandResult<()> {
                handle
                    .set_menu(crate::app_menu::build(&handle).map_err(err)?)
                    .map_err(err)?;
                crate::app_menu::configure_visibility().map_err(err)
            })();
            let _ = send.send(result);
        })
        .map_err(err)?;
        receive.await.map_err(err)??;
    }
    Ok(())
}
