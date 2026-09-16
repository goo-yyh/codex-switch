//! Credential policy is shared by the native coordinator and isolated tests.
use crate::{
    gateway::Route,
    message,
    providers::{normalize_address, Connection},
    Result,
};
use serde::Serialize;

pub trait Credentials {
    fn read(&self, id: &str) -> Result<String>;
    fn write(&self, id: &str, key: &str) -> Result<()>;
    fn delete(&self, id: &str) -> Result<()>;
}

pub fn same_credential_scope(a: &Connection, b: &Connection) -> bool {
    a.preset_id == b.preset_id
        && a.options.full_url == b.options.full_url
        && matches!((normalize_address(&a.endpoint,a.options.full_url), normalize_address(&b.endpoint,b.options.full_url)), (Ok(a), Ok(b)) if a == b)
}

pub fn prepare_save(
    mut connection: Connection,
    previous: Option<&Connection>,
    supplied: &str,
    vault: &impl Credentials,
) -> Result<(Connection, String)> {
    connection.validate()?;
    if !connection.id.is_empty() && previous.is_none() {
        return Err(message("原连接不存在，请重新添加。"));
    }
    let key = if supplied.trim().is_empty() {
        let old = previous.ok_or_else(|| message("请填写 API Key。"))?;
        if !same_credential_scope(old, &connection) {
            return Err(message(
                "服务商或 API 地址已改变，请填写该服务的新 API Key。",
            ));
        }
        vault.read(&old.id)?
    } else {
        supplied.trim().to_owned()
    };
    if key.trim().is_empty() {
        return Err(message("此连接的密钥为空，请重新填写。"));
    }
    // Both routing and credential revisions get a new identity. Existing tasks
    // retain their original endpoint, provider options and credential.
    if previous.is_none()
        || previous.is_some_and(|old| {
            !supplied.trim().is_empty()
                || old.endpoint != connection.endpoint
                || old.model != connection.model
                || old.protocol != connection.protocol
                || old.context_window != connection.context_window
                || old.options != connection.options
                || old.preset_id != connection.preset_id
        })
    {
        connection.id = uuid::Uuid::new_v4().to_string();
    }
    Ok((connection, key))
}

pub fn persist_connection(
    connection: &Connection,
    key: &str,
    new_identity: bool,
    vault: &impl Credentials,
    save: impl FnOnce() -> Result<()>,
) -> Result<()> {
    // Existing identities only change metadata; their key is already stored.
    if new_identity {
        vault.write(&connection.id, key)?;
    }
    if let Err(error) = save() {
        if new_identity && vault.delete(&connection.id).is_err() {
            return Err(message(format!(
                "{error} 新凭据清理失败，请在系统凭据库中移除未保存的连接 {}。",
                connection.id
            )));
        }
        return Err(error);
    }
    Ok(())
}

/// Prepare new independent routes sharing the supplied provider credentials.
/// Model order is preserved: the first choice is the initial active route.
pub fn prepare_batch(base: &Connection, models: &[String], key: &str) -> Result<Vec<Connection>> {
    if !base.id.is_empty() {
        return Err(message("批量添加不能修改已有连接。"));
    }
    if key.trim().is_empty() {
        return Err(message("请填写 API Key。"));
    }
    if models.is_empty() || models.len() > 20 {
        return Err(message("请选择 1 到 20 个模型。"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut connections = vec![];
    for model in models {
        let mut c = base.clone();
        c.model = model.trim().to_owned();
        c.id = uuid::Uuid::new_v4().to_string();
        c.validate()?;
        if !seen.insert(c.model.clone()) {
            return Err(message("模型不能重复。"));
        }
        connections.push(c);
    }
    Ok(connections)
}

pub async fn validate_batch<F, Fut>(
    connections: &[Connection],
    cancel: &tokio_util::sync::CancellationToken,
    mut check: F,
) -> Result<()>
where
    F: FnMut(Connection) -> Fut,
    Fut: std::future::Future<Output = crate::gateway::ProbeReceipt>,
{
    for c in connections {
        let receipt = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(message("验证已取消，整批连接未保存。")),
            result = check(c.clone()) => result,
        };
        if !receipt.ok {
            return Err(message(format!(
                "{}：{} 整批连接未保存。",
                c.model, receipt.message
            )));
        }
    }
    if cancel.is_cancelled() {
        return Err(message("验证已取消，整批连接未保存。"));
    }
    Ok(())
}

/// All probes must succeed before this is called. Metadata commits in one
/// transaction; compensate any new credential writes if persistence fails.
pub fn persist_batch(
    connections: &[Connection],
    key: &str,
    vault: &impl Credentials,
    save: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let mut written = vec![];
    let result = (|| {
        for c in connections {
            // Include even a failed write: a credentials backend may fail after writing.
            written.push(c.id.as_str());
            vault.write(&c.id, key.trim())?;
        }
        save()
    })();
    if let Err(error) = result {
        let leftovers: Vec<_> = written
            .into_iter()
            .filter(|id| vault.delete(id).is_err())
            .collect();
        if !leftovers.is_empty() {
            return Err(message(format!(
                "{error} 新凭据清理失败，请在系统凭据库移除未保存的连接：{}。",
                leftovers.join(", ")
            )));
        }
        return Err(error);
    }
    Ok(())
}

#[derive(Clone, Serialize, Debug)]
pub struct UnavailableConnection {
    pub id: String,
    pub message: String,
}
pub fn load_routes(
    connections: Vec<Connection>,
    vault: &impl Credentials,
) -> (Vec<Route>, Vec<UnavailableConnection>) {
    let mut routes = vec![];
    let mut unavailable = vec![];
    for connection in connections {
        match vault.read(&connection.id) {
            Ok(key) if !key.trim().is_empty() => routes.push(Route {
                connection,
                key,
                aliases: vec![],
            }),
            _ => unavailable.push(UnavailableConnection {
                id: connection.id,
                message: "密钥不可用，请编辑连接并重新填写。".into(),
            }),
        }
    }
    (routes, unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Protocol;
    use std::{cell::RefCell, collections::HashMap};
    #[derive(Default)]
    struct Vault(RefCell<HashMap<String, String>>, RefCell<usize>);
    impl Credentials for Vault {
        fn read(&self, id: &str) -> Result<String> {
            *self.1.borrow_mut() += 1;
            self.0
                .borrow()
                .get(id)
                .cloned()
                .ok_or_else(|| message("missing"))
        }
        fn write(&self, id: &str, key: &str) -> Result<()> {
            self.0.borrow_mut().insert(id.into(), key.into());
            Ok(())
        }
        fn delete(&self, id: &str) -> Result<()> {
            self.0.borrow_mut().remove(id);
            Ok(())
        }
    }
    fn connection() -> Connection {
        Connection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "mock".into(),
            preset_id: "custom".into(),
            endpoint: "https://example.com/v1".into(),
            model: "m".into(),
            protocol: Protocol::Chat,
            context_window: 32768,
            options: Default::default(),
        }
    }
    #[test]
    fn scope_changes_reject_before_reading_credentials() {
        let old = connection();
        let vault = Vault::default();
        for (endpoint, preset) in [
            ("https://other.example/v1", "custom"),
            ("https://example.com/coding/v1", "custom"),
            ("https://example.com/v1", "other"),
        ] {
            let mut new = old.clone();
            new.endpoint = endpoint.into();
            new.preset_id = preset.into();
            assert!(prepare_save(new, Some(&old), "", &vault).is_err());
        }
        assert_eq!(*vault.1.borrow(), 0);
    }
    #[test]
    fn model_edits_reuse_keys_but_explicit_keys_create_immutable_revisions() {
        let old = connection();
        let vault = Vault::default();
        vault.write(&old.id, "synthetic-old").unwrap();
        let mut new = old.clone();
        new.model = "new-model".into();
        let (new, key) = prepare_save(new, Some(&old), "", &vault).unwrap();
        assert_eq!(key, "synthetic-old");
        assert_ne!(new.id, old.id);
        let mut changed = old.clone();
        changed.endpoint = "https://other.example/v1".into();
        let (changed, key) = prepare_save(changed, Some(&old), "synthetic-new", &vault).unwrap();
        assert_eq!(key, "synthetic-new");
        assert_ne!(changed.id, old.id);
        assert_eq!(vault.read(&old.id).unwrap(), "synthetic-old");
    }
    #[test]
    fn failed_metadata_save_removes_only_new_credential() {
        let old = connection();
        let vault = Vault::default();
        vault.write(&old.id, "old").unwrap();
        let (new, key) = prepare_save(old.clone(), Some(&old), "new", &vault).unwrap();
        assert!(persist_connection(&new, &key, true, &vault, || Err(message(
            "database unavailable"
        )))
        .is_err());
        assert!(vault.read(&new.id).is_err());
        assert_eq!(vault.read(&old.id).unwrap(), "old");
    }
    #[test]
    fn missing_unselected_key_does_not_hide_healthy_route() {
        let a = connection();
        let b = connection();
        let vault = Vault::default();
        vault.write(&a.id, "synthetic").unwrap();
        let (routes, unavailable) = load_routes(vec![a.clone(), b.clone()], &vault);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].connection.id, a.id);
        assert_eq!(unavailable.len(), 1);
        assert_eq!(unavailable[0].id, b.id);
    }
    fn batch() -> Vec<Connection> {
        let mut base = connection();
        base.id.clear();
        prepare_batch(&base, &["model-a".into(), "model-b".into()], "synthetic").unwrap()
    }
    #[test]
    fn batch_rejects_empty_duplicate_and_edit_requests() {
        let mut base = connection();
        assert!(prepare_batch(&base, &["m".into()], "synthetic").is_err());
        base.id.clear();
        for models in [
            vec![],
            vec![" ".into()],
            vec!["a".into(), " a ".into()],
            vec!["a".into(); 21],
        ] {
            assert!(prepare_batch(&base, &models, "synthetic").is_err());
        }
        assert!(prepare_batch(&base, &["a".into()], " ").is_err());
        let connections = batch();
        assert_eq!(
            connections
                .iter()
                .map(|c| c.model.as_str())
                .collect::<Vec<_>>(),
            ["model-a", "model-b"]
        );
        assert_ne!(connections[0].id, connections[1].id);
    }
    #[test]
    fn batch_database_failure_cleans_all_new_keys_and_preserves_old_keys() {
        let vault = Vault::default();
        vault.write("existing", "old").unwrap();
        let connections = batch();
        assert!(
            persist_batch(&connections, "synthetic", &vault, || Err(message(
                "database failure"
            )))
            .is_err()
        );
        assert_eq!(vault.0.borrow().len(), 1);
        assert_eq!(vault.read("existing").unwrap(), "old");
    }
    #[test]
    fn batch_partial_credential_failure_never_commits_metadata() {
        struct FailingVault(Vault);
        impl Credentials for FailingVault {
            fn read(&self, id: &str) -> Result<String> {
                self.0.read(id)
            }
            fn write(&self, id: &str, key: &str) -> Result<()> {
                self.0.write(id, key)?;
                if self.0 .0.borrow().len() == 2 {
                    Err(message("vault failure"))
                } else {
                    Ok(())
                }
            }
            fn delete(&self, id: &str) -> Result<()> {
                self.0.delete(id)
            }
        }
        let vault = FailingVault(Vault::default());
        let result = persist_batch(&batch(), "synthetic", &vault, || {
            panic!("must not save metadata")
        });
        assert!(result.is_err());
        assert!(vault.0 .0.borrow().is_empty());
    }
    #[test]
    fn batch_persists_separate_routes_with_the_same_key() {
        let vault = Vault::default();
        let store = crate::store::Store::memory().unwrap();
        let connections = batch();
        persist_batch(&connections, " synthetic ", &vault, || {
            store.save_batch(&connections)
        })
        .unwrap();
        assert_eq!(store.list().unwrap().len(), 2);
        for c in connections {
            assert_eq!(vault.read(&c.id).unwrap(), "synthetic");
        }
    }
    fn receipt(ok: bool) -> crate::gateway::ProbeReceipt {
        crate::gateway::ProbeReceipt {
            ok,
            status: Some(if ok { 200 } else { 403 }),
            message: "mock result".into(),
            elapsed_ms: 0,
        }
    }
    #[tokio::test]
    async fn batch_validation_names_failed_model_and_stops() {
        let mut connections = batch();
        connections.push(connection());
        let mut checked = vec![];
        let cancel = tokio_util::sync::CancellationToken::new();
        let result = validate_batch(&connections, &cancel, |c| {
            checked.push(c.model.clone());
            async move { receipt(c.model != "model-b") }
        })
        .await;
        assert!(result.unwrap_err().to_string().contains("model-b"));
        assert_eq!(checked, ["model-a", "model-b"]);
    }
    #[tokio::test]
    async fn batch_cancel_during_probe_stops_subsequent_models() {
        let cancel = tokio_util::sync::CancellationToken::new();
        let mut checked = 0;
        let result = validate_batch(&batch(), &cancel, |_| {
            checked += 1;
            cancel.cancel();
            std::future::pending()
        })
        .await;
        assert!(result.unwrap_err().to_string().contains("取消"));
        assert_eq!(checked, 1);
    }
    #[tokio::test]
    async fn batch_final_probe_cancel_race_is_not_success() {
        let cancel = tokio_util::sync::CancellationToken::new();
        let result = validate_batch(&batch()[..1], &cancel, |_| {
            cancel.cancel();
            async { receipt(true) }
        })
        .await;
        assert!(result.is_err());
    }
}
