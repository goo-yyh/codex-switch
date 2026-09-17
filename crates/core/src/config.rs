//! File effects are rooted in explicitly supplied directories. No home discovery here.
use crate::{message, Error, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use toml_edit::{value, DocumentMut, Item, Table};

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| message("文件路径无效。"))?;
    fs::create_dir_all(parent)?;
    if fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(message("拒绝写入符号链接配置文件。"));
    }
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| Error::Io(e.error))?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}
fn restore_optional(path: &Path, bytes: Option<&[u8]>) -> Result<()> {
    if read_optional(path)?.as_deref() == bytes {
        return Ok(());
    }
    match bytes {
        Some(bytes) => atomic_write(path, bytes),
        None => {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(e.into()),
            }
            #[cfg(unix)]
            File::open(path.parent().ok_or_else(|| message("文件路径无效。"))?)?.sync_all()?;
            Ok(())
        }
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    if fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(message("配置文件是符号链接，请使用独立的普通配置文件。"));
    }
    match fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Journal {
    version: u32,
    phase: String,
    config_path: PathBuf,
    before: Option<Vec<u8>>,
    applied_hash: String,
    #[serde(default)]
    previous_hash: Option<String>,
}

impl Journal {
    fn matches(&self, bytes: &[u8]) -> bool {
        let h = hash(bytes);
        h == self.applied_hash
            || (self.phase == "prepared" && self.previous_hash.as_ref() == Some(&h))
    }
}

#[derive(Clone)]
pub struct ConfigManager {
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
}
impl ConfigManager {
    pub fn new(config_dir: PathBuf, state_dir: PathBuf) -> Self {
        Self {
            config_dir,
            state_dir,
        }
    }
    pub fn path(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }
    fn journal_path(&self) -> PathBuf {
        self.state_dir.join("activation.json")
    }
    fn journal(&self) -> Result<Option<Journal>> {
        read_optional(&self.journal_path())?
            .map(|b| serde_json::from_slice(&b).map_err(Into::into))
            .transpose()
    }
    fn lock(&self) -> Result<File> {
        private_dir(&self.state_dir)?;
        let f = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.state_dir.join("config.lock"))?;
        f.try_lock_exclusive()
            .map_err(|_| message("另一个配置操作正在进行，请稍后重试。"))?;
        Ok(f)
    }
    fn save(&self, j: &Journal) -> Result<()> {
        atomic_write(&self.journal_path(), &serde_json::to_vec(j)?)
    }
    pub fn enabled(&self) -> Result<bool> {
        Ok(self.journal()?.is_some_and(|j| j.phase != "restored"))
    }
    pub fn enable(
        &self,
        endpoint: &str,
        token: &str,
        model: &str,
        catalog_path: &Path,
    ) -> Result<()> {
        self.enable_with_commit(endpoint, token, model, catalog_path, || Ok(()))
    }
    /// Apply config and commit associated metadata while retaining a rollback
    /// snapshot of THIS operation, separate from the activation baseline.
    pub fn enable_with_commit(
        &self,
        endpoint: &str,
        token: &str,
        model: &str,
        catalog_path: &Path,
        commit: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        self.enable_with_options(endpoint, token, model, catalog_path, false, commit)
    }
    pub fn enable_with_options(
        &self,
        endpoint: &str,
        token: &str,
        model: &str,
        catalog_path: &Path,
        remote_compaction: bool,
        commit: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let _lock = self.lock()?;
        let before = read_optional(&self.path())?;
        let journal = read_optional(&self.journal_path())?;
        let original = String::from_utf8(before.clone().unwrap_or_default())
            .map_err(|_| message("配置不是有效的 UTF-8 文本。"))?;
        let target = project_with_options(
            &original,
            endpoint,
            token,
            model,
            catalog_path,
            remote_compaction,
        )?;
        let result = self
            .enable_locked(before.clone(), &target)
            .and_then(|_| commit());
        if let Err(error) = result {
            let current = read_optional(&self.path())?;
            // Never compensate over a concurrent external edit. The prepared
            // journal still retains the original baseline for explicit recovery.
            if current != before && current.as_deref() != Some(target.as_bytes()) {
                return Err(Error::Conflict);
            }
            restore_optional(&self.path(), before.as_deref())?;
            restore_optional(&self.journal_path(), journal.as_deref())?;
            return Err(error);
        }
        Ok(())
    }
    fn enable_locked(&self, current: Option<Vec<u8>>, target: &str) -> Result<()> {
        let old = self.journal()?.filter(|j| j.phase != "restored");
        if let Some(j) = &old {
            if j.config_path != self.path() {
                return Err(message("配置位置与备份不一致。"));
            }
            let cur = current.as_deref().unwrap_or_default();
            if !j.matches(cur) && current != j.before {
                return Err(Error::Conflict);
            }
        }
        let mut j = Journal {
            version: 1,
            phase: "prepared".into(),
            config_path: self.path(),
            before: old
                .as_ref()
                .map(|j| j.before.clone())
                .unwrap_or(current.clone()),
            applied_hash: hash(target.as_bytes()),
            previous_hash: Some(hash(current.as_deref().unwrap_or_default())),
        };
        self.save(&j)?; // baseline reaches disk before any live write
        if read_optional(&self.path())? != current {
            return Err(Error::Conflict);
        }
        atomic_write(&self.path(), target.as_bytes())?;
        if read_optional(&self.path())?.as_deref() != Some(target.as_bytes()) {
            return Err(Error::Conflict);
        }
        j.phase = "enabled".into();
        j.previous_hash = None;
        self.save(&j)
    }
    /// Caller must ensure Codex has exited and no gateway requests remain.
    pub fn prune_catalogs(&self) -> Result<()> {
        let _lock = self.lock()?;
        let mut retained = std::collections::HashSet::new();
        for bytes in [
            read_optional(&self.path())?,
            self.journal()?.and_then(|j| j.before),
        ]
        .into_iter()
        .flatten()
        {
            let doc = String::from_utf8_lossy(&bytes)
                .parse::<DocumentMut>()
                .map_err(|_| message("配置无法解析，保留所有模型目录。"))?;
            if let Some(path) = doc.get("model_catalog_json").and_then(Item::as_str) {
                retained.insert(PathBuf::from(path));
            }
        }
        for entry in fs::read_dir(&self.state_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let owned = name
                .strip_prefix("codex-switch-catalog-")
                .and_then(|n| n.strip_suffix(".json"))
                .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok());
            if owned && entry.file_type()?.is_file() && !retained.contains(&entry.path()) {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }
    pub fn disable(&self, force: bool) -> Result<bool> {
        let _lock = self.lock()?;
        let Some(mut j) = self.journal()?.filter(|j| j.phase != "restored") else {
            return Ok(false);
        };
        if j.config_path != self.path() {
            return Err(message("配置位置与备份不一致，未执行恢复。"));
        }
        let current = read_optional(&self.path())?;
        if current != j.before && !j.matches(current.as_deref().unwrap_or_default()) {
            if !force {
                return Err(Error::Conflict);
            }
            if let Some(b) = &current {
                atomic_write(
                    &self
                        .state_dir
                        .join(format!("before-restore-{}.toml", uuid::Uuid::new_v4())),
                    b,
                )?;
            }
        }
        if read_optional(&self.path())? != current {
            return Err(Error::Conflict);
        }
        match &j.before {
            Some(b) => atomic_write(&self.path(), b)?,
            None => {
                restore_optional(&self.path(), None)?;
            }
        }
        j.phase = "restored".into();
        self.save(&j)?;
        Ok(true)
    }
}

pub fn project(
    original: &str,
    endpoint: &str,
    token: &str,
    model: &str,
    catalog: &Path,
) -> Result<String> {
    project_with_options(original, endpoint, token, model, catalog, false)
}
pub fn project_with_options(
    original: &str,
    endpoint: &str,
    token: &str,
    model: &str,
    catalog: &Path,
    remote_compaction: bool,
) -> Result<String> {
    let mut d = original
        .parse::<DocumentMut>()
        .map_err(|_| message("现有 config.toml 无法解析，原文件未修改。"))?;
    if let Some(existing) = d.get("model_catalog_json").and_then(Item::as_str) {
        if !existing.contains("codex-switch-catalog-") {
            return Err(message(
                "检测到用户自定义模型目录，请先在 Codex 中处理该目录配置；原配置未修改。",
            ));
        }
    }
    d["model_provider"] = value("codex_switch");
    d["model"] = value(model);
    d["model_catalog_json"] = value(catalog.to_string_lossy().as_ref());
    d["web_search"] = value("disabled");
    // Avoid carrying model-specific official defaults into an unknown provider.
    for k in [
        "model_reasoning_effort",
        "model_reasoning_summary",
        "model_verbosity",
        "model_context_window",
        "model_auto_compact_token_limit",
        "service_tier",
    ] {
        d.remove(k);
    }
    if d.get("model_providers").is_none() {
        d["model_providers"] = Item::Table(Table::new());
    }
    if !d["model_providers"].is_table() {
        return Err(message("model_providers 必须是 TOML 表，原配置未修改。"));
    }
    let mut p = Table::new();
    p["name"] = value(if remote_compaction {
        "OpenAI"
    } else {
        "Codex Switch"
    });
    p["base_url"] = value(endpoint);
    p["wire_api"] = value("responses");
    p["requires_openai_auth"] = value(false);
    p["supports_websockets"] = value(false);
    p["experimental_bearer_token"] = value(token);
    d["model_providers"]["codex_switch"] = Item::Table(p);
    Ok(d.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (tempfile::TempDir, ConfigManager) {
        let t = tempfile::tempdir().unwrap();
        let m = ConfigManager::new(t.path().join("codex"), t.path().join("switch"));
        (t, m)
    }
    fn on(m: &ConfigManager, model: &str) {
        m.enable(
            "http://127.0.0.1:1234/v1",
            "test-token",
            model,
            &m.state_dir.join("codex-switch-catalog-1.json"),
        )
        .unwrap();
    }
    #[test]
    fn exact_restore_including_comments_and_no_auth_changes() {
        let (_t, m) = setup();
        let b = b"# user\r\nmodel = 'original'\r\n[mcp_servers.demo]\r\ncommand = 'test'\r\n";
        atomic_write(&m.path(), b).unwrap();
        atomic_write(&m.config_dir.join("auth.json"), b"leave me").unwrap();
        on(&m, "a");
        on(&m, "b");
        assert!(m.enabled().unwrap());
        m.disable(false).unwrap();
        assert_eq!(fs::read(m.path()).unwrap(), b);
        assert_eq!(
            fs::read(m.config_dir.join("auth.json")).unwrap(),
            b"leave me"
        );
        assert!(!m.enabled().unwrap());
    }
    #[test]
    fn absent_file_is_removed_on_disable() {
        let (_t, m) = setup();
        on(&m, "a");
        m.disable(false).unwrap();
        assert!(!m.path().exists());
        assert!(!m.disable(false).unwrap());
    }
    #[test]
    fn rollback_does_not_overwrite_a_concurrent_external_edit() {
        // An edit after the operation snapshot must not become a new baseline.
        {
            let (_t, m) = setup();
            atomic_write(&m.path(), b"# original snapshot\n").unwrap();
            {
                let _lock = m.lock().unwrap();
                let before = read_optional(&m.path()).unwrap();
                let target = project(
                    "# original snapshot\n",
                    "http://127.0.0.1",
                    "synthetic",
                    "new",
                    Path::new("catalog"),
                )
                .unwrap();
                atomic_write(&m.path(), b"# edit after snapshot\n").unwrap();
                assert!(matches!(
                    m.enable_locked(before, &target),
                    Err(Error::Conflict)
                ));
                assert_eq!(fs::read(m.path()).unwrap(), b"# edit after snapshot\n");
            }
            m.disable(true).unwrap();
            assert_eq!(fs::read(m.path()).unwrap(), b"# original snapshot\n");
        }
        let (_t, m) = setup();
        atomic_write(&m.path(), b"# original\n").unwrap();
        let result = m.enable_with_commit(
            "http://127.0.0.1",
            "synthetic",
            "new",
            &m.state_dir.join("codex-switch-catalog-new.json"),
            || {
                atomic_write(&m.path(), b"# external edit during commit\n")?;
                Err(message("injected metadata failure"))
            },
        );
        assert!(matches!(result, Err(Error::Conflict)));
        assert_eq!(
            fs::read(m.path()).unwrap(),
            b"# external edit during commit\n"
        );
        m.disable(true).unwrap();
        assert_eq!(fs::read(m.path()).unwrap(), b"# original\n");
    }
    #[test]
    fn catalog_cleanup_retains_live_and_baseline_references_and_unowned_files() {
        let (_t, m) = setup();
        let baseline = m.state_dir.join(format!(
            "codex-switch-catalog-{}.json",
            uuid::Uuid::new_v4()
        ));
        let live = m.state_dir.join(format!(
            "codex-switch-catalog-{}.json",
            uuid::Uuid::new_v4()
        ));
        let old = m.state_dir.join(format!(
            "codex-switch-catalog-{}.json",
            uuid::Uuid::new_v4()
        ));
        let user = m.state_dir.join("codex-switch-catalog-user.json");
        for path in [&baseline, &live, &old, &user] {
            atomic_write(path, b"{}").unwrap();
        }
        atomic_write(
            &m.path(),
            format!("model_catalog_json = '{}'\n", baseline.display()).as_bytes(),
        )
        .unwrap();
        m.enable("http://127.0.0.1", "synthetic", "model", &live)
            .unwrap();
        m.prune_catalogs().unwrap();
        assert!(baseline.exists());
        assert!(live.exists());
        assert!(user.exists());
        assert!(!old.exists());
        m.disable(false).unwrap();
        m.prune_catalogs().unwrap();
        assert!(baseline.exists());
        assert!(!live.exists());
    }
    #[test]
    fn each_cycle_captures_a_fresh_baseline() {
        let (_t, m) = setup();
        on(&m, "a");
        m.disable(false).unwrap();
        atomic_write(&m.path(), b"model = 'new-user-model'\n").unwrap();
        on(&m, "b");
        m.disable(false).unwrap();
        assert_eq!(
            fs::read_to_string(m.path()).unwrap(),
            "model = 'new-user-model'\n"
        );
    }
    #[test]
    fn conflict_does_not_clobber_and_forced_restore_backs_up() {
        let (_t, m) = setup();
        on(&m, "a");
        atomic_write(&m.path(), b"model='external'\n").unwrap();
        assert!(matches!(m.disable(false), Err(Error::Conflict)));
        assert!(m.enabled().unwrap());
        m.disable(true).unwrap();
        assert!(!m.path().exists());
        assert!(fs::read_dir(&m.state_dir).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("before-restore-")));
    }
    #[test]
    fn automatic_restore_preserves_external_edits_and_original_bytes() {
        let (_t, m) = setup();
        let original = b"# original settings\r\nmodel = 'original'\r\n";
        atomic_write(&m.path(), original).unwrap();
        on(&m, "a");
        let external = b"# later edits\nmodel = 'external'\n";
        atomic_write(&m.path(), external).unwrap();
        m.disable(true).unwrap();
        assert_eq!(fs::read(m.path()).unwrap(), original);
        assert!(!m.enabled().unwrap());
        let backup = fs::read_dir(&m.state_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("before-restore-")
            })
            .unwrap();
        assert_eq!(fs::read(backup).unwrap(), external);
        assert!(!m.disable(true).unwrap());
    }
    #[test]
    fn corrupted_input_never_changes() {
        let (_t, m) = setup();
        atomic_write(&m.path(), b"invalid [[").unwrap();
        assert!(m
            .enable("http://127.0.0.1", "t", "m", Path::new("catalog"))
            .is_err());
        assert_eq!(fs::read(m.path()).unwrap(), b"invalid [[");
        assert!(!m.enabled().unwrap());
    }
    #[test]
    fn restart_can_restore_without_runtime() {
        let (t, m) = setup();
        atomic_write(&m.path(), b"# baseline\n").unwrap();
        on(&m, "a");
        let restarted = ConfigManager::new(t.path().join("codex"), t.path().join("switch"));
        restarted.disable(false).unwrap();
        assert_eq!(fs::read(m.path()).unwrap(), b"# baseline\n");
    }
    #[test]
    fn interrupted_reenable_restores_original_baseline() {
        let (_t, m) = setup();
        atomic_write(&m.path(), b"# original\n").unwrap();
        on(&m, "a");
        let mut j = m.journal().unwrap().unwrap();
        j.phase = "prepared".into();
        j.previous_hash = Some(j.applied_hash.clone());
        j.applied_hash = hash(b"not yet written");
        m.save(&j).unwrap();
        m.disable(false).unwrap();
        assert_eq!(fs::read(m.path()).unwrap(), b"# original\n");
    }
    #[test]
    fn invalid_provider_table_returns_error() {
        assert!(project("model_providers=1", "x", "t", "m", Path::new("x")).is_err());
    }
    #[test]
    fn preserves_unrelated_fields_and_rejects_external_catalog() {
        let result = project(
            "# hi\n[mcp_servers.test]\ncommand='ok'\n",
            "http://127.0.0.1",
            "t",
            "m",
            Path::new("codex-switch-catalog-1.json"),
        )
        .unwrap();
        assert!(result.contains("# hi"));
        assert!(result.contains("command='ok'"));
        assert!(project(
            "model_catalog_json='mine.json'",
            "x",
            "t",
            "m",
            Path::new("x")
        )
        .is_err());
    }
    #[test]
    fn exclusive_lock_blocks_a_second_writer() {
        let (_t, m) = setup();
        let lock = m.lock().unwrap();
        assert!(m
            .enable("http://127.0.0.1", "t", "m", Path::new("catalog"))
            .is_err());
        assert!(!m.path().exists());
        drop(lock);
        on(&m, "a");
    }
    #[test]
    fn failed_reenable_never_replaces_original_backup() {
        let (_t, m) = setup();
        atomic_write(&m.path(), b"# baseline\n").unwrap();
        on(&m, "a");
        atomic_write(&m.path(), b"model='user-edit'\n").unwrap();
        assert!(m
            .enable("http://127.0.0.1", "t", "b", Path::new("catalog"))
            .is_err());
        m.disable(true).unwrap();
        assert_eq!(fs::read(m.path()).unwrap(), b"# baseline\n");
    }
    #[cfg(unix)]
    #[test]
    fn symlink_is_not_followed() {
        use std::os::unix::fs::symlink;
        let (t, m) = setup();
        fs::create_dir_all(&m.config_dir).unwrap();
        let dest = t.path().join("private-config");
        fs::write(&dest, b"model='untouched'").unwrap();
        symlink(&dest, m.path()).unwrap();
        assert!(m
            .enable("http://127.0.0.1", "t", "m", Path::new("catalog"))
            .is_err());
        assert_eq!(fs::read(dest).unwrap(), b"model='untouched'");
    }
}
