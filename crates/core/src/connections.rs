//! Credential policy is shared by the native coordinator and isolated tests.
use crate::{
    message,
    providers::{normalize_address, Connection},
    Result,
};

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

pub fn persist_connection(
    credential_id: &str,
    key: &str,
    new_identity: bool,
    vault: &impl Credentials,
    save: impl FnOnce() -> Result<()>,
) -> Result<()> {
    // Existing identities only change metadata; their key is already stored.
    // A vault can fail after writing, so compensate both vault and database failures.
    // Never delete an existing credential when a metadata-only update fails.
    let result = if new_identity {
        vault.write(credential_id, key).and_then(|()| save())
    } else {
        save()
    };
    if let Err(error) = result {
        if new_identity && vault.delete(credential_id).is_err() {
            return Err(message(format!(
                "{error} 新凭据清理失败，请在系统凭据库中移除未保存的连接 {}。",
                credential_id
            )));
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::HashMap};
    #[derive(Default)]
    struct Vault(RefCell<HashMap<String, String>>);
    impl Credentials for Vault {
        fn read(&self, id: &str) -> Result<String> {
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
    #[test]
    fn failed_metadata_save_removes_only_new_credential() {
        let vault = Vault::default();
        vault.write("old", "original").unwrap();
        assert!(
            persist_connection("new", "replacement", true, &vault, || Err(message(
                "database unavailable"
            )))
            .is_err()
        );
        assert!(vault.read("new").is_err());
        assert_eq!(vault.read("old").unwrap(), "original");
        assert!(
            persist_connection("old", "ignored", false, &vault, || Err(message(
                "database unavailable"
            )))
            .is_err()
        );
        assert_eq!(vault.read("old").unwrap(), "original");
    }
    #[test]
    fn failed_vault_write_is_compensated_without_saving_metadata() {
        struct FailingVault(Vault);
        impl Credentials for FailingVault {
            fn read(&self, id: &str) -> Result<String> {
                self.0.read(id)
            }
            fn write(&self, id: &str, key: &str) -> Result<()> {
                self.0.write(id, key)?;
                Err(message("vault write failed after persistence"))
            }
            fn delete(&self, id: &str) -> Result<()> {
                self.0.delete(id)
            }
        }
        let vault = FailingVault(Vault::default());
        assert!(
            persist_connection("new", "synthetic", true, &vault, || panic!(
                "metadata must not be saved"
            ))
            .is_err()
        );
        assert!(vault.read("new").is_err());
    }
    #[test]
    fn existing_credential_is_not_overwritten_on_metadata_save() {
        let vault = Vault::default();
        vault.write("existing", "original").unwrap();
        persist_connection("existing", "ignored", false, &vault, || Ok(())).unwrap();
        assert_eq!(vault.read("existing").unwrap(), "original");
    }
}
