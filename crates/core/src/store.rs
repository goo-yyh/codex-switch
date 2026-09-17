use crate::{config::private_dir, profiles::Profile, providers::Connection, Result};
use rusqlite::{params, Connection as Db};
use std::path::Path;

pub struct Store {
    db: Db,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        private_dir(path.parent().unwrap())?;
        let db = Db::open(path)?;
        Self::init(db)
    }
    pub fn memory() -> Result<Self> {
        Self::init(Db::open_in_memory()?)
    }
    fn init(db: Db) -> Result<Self> {
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS connections(id TEXT PRIMARY KEY, json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
        let store = Self { db };
        store.db.execute_batch("CREATE TABLE IF NOT EXISTS profiles(id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS route_keys(route_id TEXT PRIMARY KEY, credential_id TEXT NOT NULL, profile_id TEXT NOT NULL);")?;
        store.migrate_profiles()?;
        Ok(store)
    }
    fn migrate_profiles(&self) -> Result<()> {
        if self.setting("profiles_v1")?.is_some() {
            return Ok(());
        }
        let tx = self.db.unchecked_transaction()?;
        let mut profiles = vec![];
        for c in self.list()? {
            let p = Profile {
                id: c.id.clone(),
                name: crate::profiles::unique_name(&c.name, &profiles),
                preset_id: c.preset_id,
                endpoint: c.endpoint,
                models: vec![c.model],
                protocol: c.protocol,
                context_window: c.context_window,
                options: c.options.clone(),
                credential_id: c.id.clone(),
                route_ids: vec![c.id.clone()],
            };
            tx.execute(
                "INSERT INTO profiles(id,name,json) VALUES(?1,?2,?3)",
                params![p.id, p.name.to_lowercase(), serde_json::to_string(&p)?],
            )?;
            tx.execute(
                "INSERT INTO route_keys VALUES(?1,?2,?3)",
                params![c.id, p.credential_id, p.id],
            )?;
            profiles.push(p);
        }
        let selected = self
            .setting("selected")?
            .into_iter()
            .filter(|id| profiles.iter().any(|p| p.id == *id))
            .collect::<Vec<_>>();
        tx.execute(
            "INSERT OR IGNORE INTO settings VALUES('selected_profiles',?1)",
            [serde_json::to_string(&selected)?],
        )?;
        self.set(
            "historical_routes",
            &serde_json::to_string(
                &profiles
                    .iter()
                    .flat_map(|p| p.route_ids.clone())
                    .collect::<Vec<_>>(),
            )?,
        )?;
        tx.execute("INSERT INTO settings VALUES('profiles_v1','1')", [])?;
        tx.commit()?;
        Ok(())
    }
    pub fn profiles(&self) -> Result<Vec<Profile>> {
        let mut stmt = self
            .db
            .prepare("SELECT json FROM profiles ORDER BY rowid")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn save_profile(&self, p: &Profile) -> Result<()> {
        let tx = self.db.unchecked_transaction()?;
        tx.execute("INSERT INTO profiles(id,name,json) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET name=excluded.name,json=excluded.json",params![p.id,p.name.to_lowercase(),serde_json::to_string(p)?])?;
        for c in p.routes() {
            tx.execute("INSERT INTO connections(id,json) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET json=excluded.json",params![c.id,serde_json::to_string(&c)?])?;
            tx.execute(
                "INSERT OR REPLACE INTO route_keys VALUES(?1,?2,?3)",
                params![c.id, p.credential_id, p.id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn selected_profiles(&self) -> Result<Vec<String>> {
        Ok(serde_json::from_str(
            &self
                .setting("selected_profiles")?
                .unwrap_or_else(|| "[]".into()),
        )?)
    }
    pub fn select_profiles(&self, ids: &[String]) -> Result<()> {
        let profiles = self.profiles()?;
        if ids.iter().any(|id| !profiles.iter().any(|p| p.id == *id))
            || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
        {
            return Err(crate::message("配置选择无效。"));
        }
        self.set("selected_profiles", &serde_json::to_string(ids)?)
    }
    pub fn selected_routes(&self) -> Result<Vec<Connection>> {
        self.routes_for_profiles(&self.selected_profiles()?)
    }
    pub fn routes_for_profiles(&self, ids: &[String]) -> Result<Vec<Connection>> {
        if ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len() {
            return Err(crate::message("配置选择无效。"));
        }
        let profiles = self.profiles()?;
        let mut routes = vec![];
        for id in ids {
            routes.extend(
                profiles
                    .iter()
                    .find(|p| p.id == *id)
                    .ok_or_else(|| crate::message("配置不存在。"))?
                    .routes(),
            );
        }
        Ok(routes)
    }
    pub fn credential_for(&self, id: &str) -> Result<String> {
        use rusqlite::OptionalExtension;
        Ok(self
            .db
            .query_row(
                "SELECT credential_id FROM route_keys WHERE route_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or_else(|| id.to_owned()))
    }
    /// Preserve old client identifiers, not old providers or credentials.
    /// A surviving model in the same profile follows its new revision; others use the default.
    pub fn aliases_for_routes(
        &self,
        current: &[Connection],
    ) -> Result<std::collections::HashMap<String, Vec<String>>> {
        let mut aliases = std::collections::HashMap::<String, Vec<String>>::new();
        let Some(default) = current.first() else {
            return Ok(aliases);
        };
        let mut stmt = self
            .db
            .prepare("SELECT route_id, profile_id FROM route_keys")?;
        let owners = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<std::result::Result<std::collections::HashMap<_, _>, _>>()?;
        for old in self.historical_routes()? {
            let target = current
                .iter()
                .find(|c| {
                    c.id == old.id
                        || (c.model == old.model
                            && owners.contains_key(&c.id)
                            && owners.get(&c.id) == owners.get(&old.id))
                })
                .unwrap_or(default);
            aliases
                .entry(target.id.clone())
                .or_default()
                .push(old.alias());
        }
        Ok(aliases)
    }
    /// Commit the activated routes atomically; selection is saved separately while off.
    pub fn commit_profiles(&self, routes: &[Connection], port: u16) -> Result<()> {
        let first = routes
            .first()
            .ok_or_else(|| crate::message("请至少勾选一个配置。"))?;
        let tx = self.db.unchecked_transaction()?;
        self.set("selected", &first.id)?;
        self.set("port", &port.to_string())?;
        self.set("applied_routes", &serde_json::to_string(routes)?)?;
        let mut known: Vec<String> = serde_json::from_str(
            &self
                .setting("historical_routes")?
                .unwrap_or_else(|| "[]".into()),
        )?;
        for c in routes {
            if !known.contains(&c.id) {
                known.push(c.id.clone());
            }
        }
        self.set("historical_routes", &serde_json::to_string(&known)?)?;
        tx.commit()?;
        Ok(())
    }
    pub fn historical_routes(&self) -> Result<Vec<Connection>> {
        let ids: Vec<String> = serde_json::from_str(
            &self
                .setting("historical_routes")?
                .unwrap_or_else(|| "[]".into()),
        )?;
        Ok(self
            .list()?
            .into_iter()
            .filter(|c| ids.contains(&c.id))
            .collect())
    }
    pub fn applied_routes(&self) -> Result<Vec<Connection>> {
        if let Some(value) = self.setting("applied_routes")? {
            Ok(serde_json::from_str(&value)?)
        } else {
            // Legacy catalogs included every saved route; preserve the selected default.
            let mut routes = self.historical_routes()?;
            if let Some(selected) = self.setting("selected")? {
                if let Some(index) = routes.iter().position(|c| c.id == selected) {
                    let first = routes.remove(index);
                    routes.insert(0, first);
                }
            }
            Ok(routes)
        }
    }
    pub fn delete_profile(&self, id: &str) -> Result<()> {
        let tx = self.db.unchecked_transaction()?;
        tx.execute("DELETE FROM profiles WHERE id=?1", [id])?;
        let ids: Vec<_> = self
            .selected_profiles()?
            .into_iter()
            .filter(|v| v != id)
            .collect();
        self.set("selected_profiles", &serde_json::to_string(&ids)?)?;
        // Retain archived routes/credential references for older Codex conversations.
        tx.commit()?;
        Ok(())
    }
    pub fn list(&self) -> Result<Vec<Connection>> {
        let mut s = self
            .db
            .prepare("SELECT json FROM connections ORDER BY rowid")?;
        let rows = s.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = vec![];
        for r in rows {
            out.push(serde_json::from_str(&r?)?);
        }
        Ok(out)
    }
    pub fn save(&self, c: &Connection) -> Result<()> {
        self.db.execute("INSERT INTO connections(id,json) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET json=excluded.json",params![c.id,serde_json::to_string(c)?])?;
        Ok(())
    }
    pub fn save_batch(&self, connections: &[Connection]) -> Result<()> {
        let tx = self.db.unchecked_transaction()?;
        for c in connections {
            // Batch creation must never overwrite a previously saved identity.
            tx.execute(
                "INSERT INTO connections(id,json) VALUES(?1,?2)",
                params![c.id, serde_json::to_string(c)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn delete(&self, id: &str) -> Result<()> {
        self.db
            .execute("DELETE FROM connections WHERE id=?1", [id])?;
        self.db.execute(
            "DELETE FROM settings WHERE key='selected' AND value=?1",
            [id],
        )?;
        Ok(())
    }
    pub fn setting(&self, k: &str) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        Ok(self
            .db
            .query_row("SELECT value FROM settings WHERE key=?1", [k], |r| r.get(0))
            .optional()?)
    }
    pub fn commit_activation(&self, id: &str, port: u16) -> Result<()> {
        let tx = self.db.unchecked_transaction()?;
        for (key, value) in [("port", port.to_string()), ("selected", id.to_owned())] {
            tx.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn set(&self, k: &str, v: &str) -> Result<()> {
        self.db.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![k,v])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Protocol;
    #[test]
    fn activation_metadata_rolls_back_both_settings_on_second_write_failure() {
        let s = Store::memory().unwrap();
        s.commit_activation("old", 1234).unwrap();
        s.db.execute_batch("CREATE TRIGGER fail_selection BEFORE INSERT ON settings WHEN NEW.key='selected' BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(s.commit_activation("new", 5678).is_err());
        assert_eq!(s.setting("port").unwrap().as_deref(), Some("1234"));
        assert_eq!(s.setting("selected").unwrap().as_deref(), Some("old"));
    }
    #[test]
    fn stores_metadata_and_selection() {
        let s = Store::memory().unwrap();
        let c = Connection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "service".into(),
            preset_id: "custom".into(),
            endpoint: "https://example.com".into(),
            model: "m".into(),
            protocol: Protocol::Chat,
            context_window: 32768,
            options: Default::default(),
        };
        s.save(&c).unwrap();
        s.set("selected", &c.id).unwrap();
        assert_eq!(s.list().unwrap().len(), 1);
        assert_eq!(s.setting("selected").unwrap(), Some(c.id.clone()));
        s.delete(&c.id).unwrap();
        assert!(s.list().unwrap().is_empty());
    }
    #[test]
    fn batch_insert_failure_rolls_back_earlier_inserts() {
        let s = Store::memory().unwrap();
        let c = Connection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "mock".into(),
            preset_id: "custom".into(),
            endpoint: "https://example.com".into(),
            model: "m".into(),
            protocol: Protocol::Chat,
            context_window: 32768,
            options: Default::default(),
        };
        s.set("selected", "existing").unwrap();
        // Force the second INSERT to violate the primary key, after the first succeeded.
        assert!(s.save_batch(&[c.clone(), c]).is_err());
        assert!(s.list().unwrap().is_empty());
        assert_eq!(s.setting("selected").unwrap().as_deref(), Some("existing"));
    }
    fn profile(id: &str, name: &str, models: &[&str]) -> Profile {
        Profile {
            id: id.into(),
            name: name.into(),
            preset_id: "kimi".into(),
            endpoint: "https://example.com/v1".into(),
            models: models.iter().map(|m| m.to_string()).collect(),
            protocol: Protocol::Chat,
            context_window: 32768,
            options: Default::default(),
            credential_id: format!("key-{id}"),
            route_ids: models.iter().map(|m| format!("{id}-{m}")).collect(),
        }
    }
    #[test]
    fn selected_profiles_expand_to_only_their_models_and_keep_distinct_key_mapping() {
        let s = Store::memory().unwrap();
        let a = profile("a", "Work", &["m1", "m2"]);
        let b = profile("b", "Personal", &["m1"]);
        let c = profile("c", "Unselected", &["m3"]);
        for p in [&a, &b, &c] {
            s.save_profile(p).unwrap();
        }
        s.select_profiles(&["a".into(), "b".into()]).unwrap();
        let routes = s.selected_routes().unwrap();
        assert_eq!(routes.len(), 3);
        assert!(routes.iter().all(|r| r.name != "Unselected"));
        assert_eq!(s.credential_for("a-m1").unwrap(), "key-a");
        assert_eq!(s.credential_for("a-m2").unwrap(), "key-a");
        assert_eq!(s.credential_for("b-m1").unwrap(), "key-b");
        assert!(s.select_profiles(&["missing".into()]).is_err());
        assert!(s.select_profiles(&["a".into(), "a".into()]).is_err());
        s.select_profiles(&[]).unwrap();
        assert!(s.selected_routes().unwrap().is_empty());
    }
    #[test]
    fn selected_edits_wait_for_apply_and_deletion_preserves_applied_snapshot() {
        let s = Store::memory().unwrap();
        let mut p = profile("a", "Work", &["m1"]);
        s.save_profile(&p).unwrap();
        s.select_profiles(&["a".into()]).unwrap();
        s.commit_profiles(&p.routes(), 1234).unwrap();
        p.name = "Renamed".into();
        p.models = vec!["m2".into()];
        p.route_ids = vec!["a-m2".into()];
        p.credential_id = "key-new".into();
        s.save_profile(&p).unwrap();
        assert_eq!(s.profiles().unwrap().len(), 1);
        assert_eq!(s.applied_routes().unwrap()[0].model, "m1");
        assert_eq!(s.selected_routes().unwrap()[0].model, "m2");
        assert_eq!(s.credential_for("a-m1").unwrap(), "key-a");
        s.delete_profile("a").unwrap();
        assert!(s.profiles().unwrap().is_empty());
        assert!(s.selected_profiles().unwrap().is_empty());
        assert_eq!(s.applied_routes().unwrap()[0].model, "m1");
    }
    #[test]
    fn profile_save_rolls_back_metadata_and_routes_on_failure() {
        let s = Store::memory().unwrap();
        let p = profile("a", "Work", &["m1", "m2"]);
        s.db.execute_batch("CREATE TRIGGER fail_route BEFORE INSERT ON route_keys WHEN NEW.route_id='a-m2' BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
        assert!(s.save_profile(&p).is_err());
        assert!(s.profiles().unwrap().is_empty());
        assert!(s.list().unwrap().is_empty());
    }
    #[test]
    fn activation_commit_rolls_back_all_metadata_on_failure() {
        let s = Store::memory().unwrap();
        let a = profile("a", "Work", &["m1"]);
        let b = profile("b", "Personal", &["m2"]);
        s.commit_profiles(&a.routes(), 1234).unwrap();
        s.db.execute_batch("CREATE TRIGGER fail_applied BEFORE INSERT ON settings WHEN NEW.key='applied_routes' BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
        assert!(s.commit_profiles(&b.routes(), 5678).is_err());
        assert_eq!(s.setting("port").unwrap().as_deref(), Some("1234"));
        assert_eq!(s.setting("selected").unwrap().as_deref(), Some("a-m1"));
        assert_eq!(s.applied_routes().unwrap()[0].model, "m1");
    }
    #[test]
    fn migration_is_idempotent_preserves_old_aliases_keys_and_uniquifies_names() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("legacy.db");
        let db = Db::open(&path).unwrap();
        db.execute_batch("CREATE TABLE connections(id TEXT PRIMARY KEY,json TEXT NOT NULL); CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);").unwrap();
        for id in ["first", "second"] {
            let c = Connection {
                id: id.into(),
                name: "Kimi".into(),
                preset_id: "kimi".into(),
                endpoint: "https://example.com/v1".into(),
                model: "model".into(),
                protocol: Protocol::Chat,
                context_window: 32768,
                options: Default::default(),
            };
            db.execute(
                "INSERT INTO connections VALUES(?1,?2)",
                params![id, serde_json::to_string(&c).unwrap()],
            )
            .unwrap();
        }
        db.execute("INSERT INTO settings VALUES('selected','second')", [])
            .unwrap();
        drop(db);
        for _ in 0..2 {
            let s = Store::open(&path).unwrap();
            let profiles = s.profiles().unwrap();
            assert_eq!(profiles.len(), 2);
            assert_eq!(profiles[0].name, "Kimi");
            assert_eq!(profiles[1].name, "Kimi-2");
            assert_eq!(profiles[1].route_ids, ["second"]);
            assert_eq!(s.credential_for("second").unwrap(), "second");
            assert_eq!(s.selected_profiles().unwrap(), ["second"]);
        }
    }
    #[test]
    fn never_applied_profiles_are_not_loaded_as_historical_routes() {
        let s = Store::memory().unwrap();
        let a = profile("a", "Work", &["m1"]);
        let b = profile("b", "Private", &["m2"]);
        s.save_profile(&a).unwrap();
        s.save_profile(&b).unwrap();
        assert!(s.historical_routes().unwrap().is_empty());
        s.commit_profiles(&a.routes(), 1234).unwrap();
        let history = s.historical_routes().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "a-m1");
        assert!(s.commit_profiles(&[], 1234).is_err());
        assert_eq!(s.applied_routes().unwrap()[0].id, "a-m1");
    }
    #[test]
    fn invalid_selection_and_empty_activation_do_not_change_saved_state() {
        let s = Store::memory().unwrap();
        let p = profile("a", "Work", &["m1"]);
        s.save_profile(&p).unwrap();
        s.select_profiles(&["a".into()]).unwrap();
        for ids in [vec!["deleted".into()], vec!["a".into(), "a".into()]] {
            assert!(s.routes_for_profiles(&ids).is_err());
            assert!(s.select_profiles(&ids).is_err());
        }
        assert!(s.commit_profiles(&[], 1234).is_err());
        assert_eq!(s.selected_profiles().unwrap(), ["a"]);
        assert!(s.applied_routes().unwrap().is_empty());
        assert!(s.setting("port").unwrap().is_none());
    }
    #[test]
    fn old_aliases_follow_current_revision_of_same_model_or_current_default() {
        let s = Store::memory().unwrap();
        let old = profile("a", "Work", &["m1", "m2", "removed"]);
        s.save_profile(&old).unwrap();
        s.commit_profiles(&old.routes(), 1234).unwrap();
        let mut edited = profile("a", "Renamed", &["m1", "m2"]);
        edited.credential_id = "new-key".into();
        edited.route_ids = vec!["new-1".into(), "new-2".into()];
        s.save_profile(&edited).unwrap();
        let aliases = s.aliases_for_routes(&edited.routes()).unwrap();
        assert!(aliases["new-1"].contains(&old.routes()[0].alias()));
        assert!(aliases["new-2"].contains(&old.routes()[1].alias()));
        assert!(aliases["new-1"].contains(&old.routes()[2].alias()));
        let other = profile("b", "Other account", &["m2"]);
        s.save_profile(&other).unwrap();
        let aliases = s.aliases_for_routes(&other.routes()).unwrap();
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases["b-m2"].len(), 3);
        assert_eq!(s.credential_for("b-m2").unwrap(), "key-b");
        assert!(s.aliases_for_routes(&[]).unwrap().is_empty());
    }
}
