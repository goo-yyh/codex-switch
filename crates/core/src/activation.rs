//! Activation coordinates a staged runtime, config and one atomic metadata commit.
//! Tests supply a runtime with no sockets and a failing metadata store.
use crate::{
    config::{atomic_write, ConfigManager},
    gateway::{Gateway, Route},
    message,
    providers::catalog,
    Result,
};
use std::future::Future;

pub trait Runtime {
    fn port(&self) -> u16;
    fn token(&self) -> &str;
    fn requests(&self) -> usize;
    fn publish(&self, routes: Vec<Route>) -> impl Future<Output = ()> + Send;
}
impl Runtime for Gateway {
    fn port(&self) -> u16 {
        self.port
    }
    fn token(&self) -> &str {
        &self.state.token
    }
    fn requests(&self) -> usize {
        self.requests()
    }
    async fn publish(&self, routes: Vec<Route>) {
        self.state.replace_routes(routes).await;
    }
}

pub async fn activate<R: Runtime, F: Future<Output = Result<R>>>(
    config: &ConfigManager,
    runtime: &mut Option<R>,
    routes: Vec<Route>,
    selected: &str,
    can_prune: bool,
    start: impl FnOnce(Vec<Route>) -> F,
    commit: impl FnOnce(u16) -> Result<()>,
) -> Result<()> {
    activate_with_options(
        config,
        runtime,
        routes,
        selected,
        ActivationOptions {
            can_prune,
            remote_compaction: false,
        },
        start,
        commit,
    )
    .await
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ActivationOptions {
    pub can_prune: bool,
    pub remote_compaction: bool,
}

pub async fn activate_with_options<R: Runtime, F: Future<Output = Result<R>>>(
    config: &ConfigManager,
    runtime: &mut Option<R>,
    routes: Vec<Route>,
    selected: &str,
    options: ActivationOptions,
    start: impl FnOnce(Vec<Route>) -> F,
    commit: impl FnOnce(u16) -> Result<()>,
) -> Result<()> {
    let ActivationOptions {
        can_prune,
        remote_compaction,
    } = options;
    let connection = routes
        .iter()
        .find(|r| r.connection.id == selected)
        .ok_or_else(|| message("请选择一个凭据可用的连接。"))?
        .connection
        .clone();
    let staged = if runtime.is_none() {
        Some(start(routes.clone()).await?)
    } else {
        None
    };
    let gateway = runtime.as_ref().or(staged.as_ref()).unwrap();
    let catalog_path = config.state_dir.join(format!(
        "codex-switch-catalog-{}.json",
        uuid::Uuid::new_v4()
    ));
    atomic_write(
        &catalog_path,
        &serde_json::to_vec(&catalog(
            &routes
                .iter()
                .map(|r| r.connection.clone())
                .collect::<Vec<_>>(),
        ))?,
    )?;
    let result = config.enable_with_options(
        &format!("http://127.0.0.1:{}/v1", gateway.port()),
        gateway.token(),
        &connection.alias(),
        &catalog_path,
        remote_compaction,
        || commit(gateway.port()),
    );
    if let Err(error) = result {
        if can_prune && gateway.requests() == 0 {
            let _ = config.prune_catalogs();
        }
        return Err(error);
    }
    // Route publication is infallible and occurs only after durable writes.
    gateway.publish(routes).await;
    if can_prune && gateway.requests() == 0 {
        let _ = config.prune_catalogs();
    }
    if let Some(gateway) = staged {
        *runtime = Some(gateway);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        providers::{Connection, Protocol},
        store::Store,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };
    struct FakeRuntime {
        routes: Arc<Mutex<Vec<String>>>,
        dropped: Arc<AtomicUsize>,
    }
    impl Runtime for FakeRuntime {
        fn port(&self) -> u16 {
            12345
        }
        fn token(&self) -> &str {
            "synthetic"
        }
        fn requests(&self) -> usize {
            0
        }
        async fn publish(&self, routes: Vec<Route>) {
            *self.routes.lock().unwrap() = routes.iter().map(|r| r.connection.id.clone()).collect();
        }
    }
    impl Drop for FakeRuntime {
        fn drop(&mut self) {
            self.dropped.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn route(id: &str) -> Route {
        Route {
            key: "synthetic".into(),
            aliases: vec![],
            connection: Connection {
                id: id.into(),
                name: id.into(),
                preset_id: "custom".into(),
                endpoint: "https://example.com".into(),
                model: id.into(),
                protocol: Protocol::Chat,
                context_window: 32768,
                options: Default::default(),
            },
        }
    }

    #[tokio::test]
    async fn failed_first_activation_stops_staged_runtime_and_restores_absence() {
        let t = tempfile::tempdir().unwrap();
        let config = ConfigManager::new(t.path().join("config"), t.path().join("state"));
        let drops = Arc::new(AtomicUsize::new(0));
        let mut runtime = None;
        let result = activate(
            &config,
            &mut runtime,
            vec![route("new")],
            "new",
            true,
            |_| async {
                Ok(FakeRuntime {
                    routes: Arc::default(),
                    dropped: drops.clone(),
                })
            },
            |_| Err(message("injected database failure")),
        )
        .await;
        assert!(result.is_err());
        assert!(runtime.is_none());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(!config.path().exists());
        assert!(!config.enabled().unwrap());
        assert!(!std::fs::read_dir(&config.state_dir).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("codex-switch-catalog-")));
    }
    #[tokio::test]
    async fn catalog_write_failure_drops_staged_runtime_without_touching_config() {
        let t = tempfile::tempdir().unwrap();
        let config = ConfigManager::new(t.path().join("config"), t.path().join("blocked-state"));
        atomic_write(&config.path(), b"# original\n").unwrap();
        std::fs::write(&config.state_dir, b"not a directory").unwrap();
        let drops = Arc::new(AtomicUsize::new(0));
        let mut runtime = None;
        let result = activate(
            &config,
            &mut runtime,
            vec![route("new")],
            "new",
            true,
            |_| async {
                Ok(FakeRuntime {
                    routes: Arc::default(),
                    dropped: drops.clone(),
                })
            },
            |_| panic!("must not commit when catalog cannot be written"),
        )
        .await;
        assert!(result.is_err());
        assert!(runtime.is_none());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(std::fs::read(config.path()).unwrap(), b"# original\n");
    }
    #[tokio::test]
    async fn failed_switch_keeps_previous_runtime_selection_and_baseline() {
        let t = tempfile::tempdir().unwrap();
        let config = ConfigManager::new(t.path().join("config"), t.path().join("state"));
        atomic_write(&config.path(), b"# original\nmodel='original'\n").unwrap();
        let store = Store::memory().unwrap();
        let drops = Arc::new(AtomicUsize::new(0));
        let published = Arc::new(Mutex::new(vec![]));
        let mut runtime = None;
        activate(
            &config,
            &mut runtime,
            vec![route("old")],
            "old",
            true,
            |_| async {
                Ok(FakeRuntime {
                    routes: published.clone(),
                    dropped: drops.clone(),
                })
            },
            |port| store.commit_activation("old", port),
        )
        .await
        .unwrap();
        let before = std::fs::read(config.path()).unwrap();
        let result = activate(
            &config,
            &mut runtime,
            vec![route("new")],
            "new",
            true,
            |_| async { panic!("must reuse existing runtime") },
            |_| Err(message("injected commit failure")),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(std::fs::read(config.path()).unwrap(), before);
        assert_eq!(published.lock().unwrap().as_slice(), ["old"]);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        assert_eq!(store.setting("selected").unwrap().as_deref(), Some("old"));
        config.disable(false).unwrap();
        assert_eq!(
            std::fs::read(config.path()).unwrap(),
            b"# original\nmodel='original'\n"
        );
    }
    #[tokio::test]
    async fn failed_prepare_and_config_conflict_never_commit_or_publish() {
        for fail_start in [true, false] {
            let t = tempfile::tempdir().unwrap();
            let config = ConfigManager::new(t.path().join("config"), t.path().join("state"));
            atomic_write(&config.path(), b"invalid [[").unwrap();
            let drops = Arc::new(AtomicUsize::new(0));
            let published = Arc::new(Mutex::new(vec![]));
            let mut runtime = None;
            let result = activate(
                &config,
                &mut runtime,
                vec![route("new")],
                "new",
                true,
                |_| async {
                    if fail_start {
                        Err(message("injected bind failure"))
                    } else {
                        Ok(FakeRuntime {
                            routes: published.clone(),
                            dropped: drops.clone(),
                        })
                    }
                },
                |_| panic!("must not commit after failure"),
            )
            .await;
            assert!(result.is_err());
            assert!(runtime.is_none());
            assert!(published.lock().unwrap().is_empty());
            assert_eq!(std::fs::read(config.path()).unwrap(), b"invalid [[");
        }
    }
}
