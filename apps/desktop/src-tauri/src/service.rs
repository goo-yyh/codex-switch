//! Shared service operations used by IPC commands and the native tray.
use crate::{
    platform::{self, Vault},
    state::{err, AppState, CommandResult},
};
use codex_switch_core::{
    connections::Credentials,
    gateway::{Gateway, Route},
    profiles::ensure_editable,
    providers::RoutingSettings,
    store::Store,
};

pub(crate) fn routing_settings(store: &Store) -> CommandResult<RoutingSettings> {
    store
        .setting("routing_settings")
        .map_err(err)?
        .map(|v| serde_json::from_str(&v).map_err(err))
        .transpose()
        .map(|v| v.unwrap_or_default())
}

pub(crate) async fn enable_inner(s: &AppState) -> CommandResult<()> {
    let (connections, mut aliases) = {
        let store = s.store.lock().map_err(err)?;
        let connections = store.selected_routes().map_err(err)?;
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
            store.commit_profiles(&connections, port)
        },
    )
    .await
    .map_err(err)?;
    *s.pending.lock().unwrap_or_else(|e| e.into_inner()) = running;
    Ok(())
}

pub(crate) async fn set_enabled_inner(s: &AppState, enabled: bool) -> CommandResult<()> {
    // Restore configuration before stopping the gateway. Existing Codex sessions
    // may still use the local endpoint until they exit, so keep it alive for them.
    if enabled {
        enable_inner(s).await
    } else {
        s.config.disable(true).map_err(err)?;
        let running = platform::app_info().running;
        *s.pending.lock().map_err(err)? = running;
        if !running {
            s.runtime.lock().await.take();
        }
        Ok(())
    }
}

// Called with the operation lock held, including the synchronous native menu action.
// Selection persists IDs only; activation is always an explicit separate operation.
pub(crate) fn select_profiles_inner(s: &AppState, ids: &[String]) -> CommandResult<()> {
    ensure_editable(s.config.enabled().map_err(err)?).map_err(err)?;
    s.store
        .lock()
        .map_err(err)?
        .select_profiles(ids)
        .map_err(err)
}
