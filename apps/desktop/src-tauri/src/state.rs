use crate::tray;
use codex_switch_core::{config::ConfigManager, gateway::Gateway, store::Store};
use std::sync::Mutex;

pub(crate) struct AppState {
    pub(crate) store: Mutex<Store>,
    pub(crate) config: ConfigManager,
    pub(crate) runtime: tokio::sync::Mutex<Option<Gateway>>,
    // Serialize mutations across IPC and tray actions; never hold a Store guard across await.
    pub(crate) operation: tokio::sync::Mutex<()>,
    pub(crate) pending: Mutex<bool>,
    pub(crate) tray_feedback: Mutex<Option<tray::Feedback>>,
    pub(crate) checks:
        Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
}
pub(crate) type CommandResult<T> = Result<T, String>;
pub(crate) fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
