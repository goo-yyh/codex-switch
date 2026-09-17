//! A named configuration owns one credential revision and several model routes.
use crate::{
    connections::{same_credential_scope, Credentials},
    message,
    providers::Connection,
    Result,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub preset_id: String,
    pub endpoint: String,
    pub models: Vec<String>,
    pub protocol: crate::providers::Protocol,
    pub context_window: u64,
    #[serde(default)]
    pub options: crate::providers::ConnectionOptions,
    #[serde(default)]
    pub credential_id: String,
    #[serde(default)]
    pub route_ids: Vec<String>,
}
impl Profile {
    fn base_route(&self, model: &str, id: &str) -> Connection {
        Connection {
            id: id.to_owned(),
            name: self.name.clone(),
            preset_id: self.preset_id.clone(),
            endpoint: self.endpoint.clone(),
            model: model.to_owned(),
            protocol: self.protocol,
            context_window: self.context_window,
            options: self.options.clone(),
        }
    }
    fn model_route(&self, model: &str, id: &str) -> Connection {
        let mut route = self.base_route(model, id);
        if let Some(spec) = self.options.model_overrides.get(model) {
            route.protocol = spec.protocol.unwrap_or(self.protocol);
            route.options.full_url = spec.full_url.unwrap_or(self.options.full_url);
            if let Some(endpoint) = &spec.endpoint {
                route.endpoint = endpoint.clone();
            }
        }
        // A model may interpret an inherited full URL as a base address, or vice versa.
        // prepare_profile validates this effective address before any key is read.
        if let Ok(endpoint) =
            crate::providers::normalize_address(&route.endpoint, route.options.full_url)
        {
            route.endpoint = endpoint;
        }
        route
    }

    pub fn routes(&self) -> Vec<Connection> {
        self.models
            .iter()
            .zip(&self.route_ids)
            .map(|(model, id)| self.model_route(model, id))
            .collect()
    }
}
pub fn ensure_editable(enabled: bool) -> Result<()> {
    if enabled {
        return Err(message(
            "Codex Switch 已开启，请先关闭服务，再修改配置或设置。",
        ));
    }
    Ok(())
}
pub fn unique_name(base: &str, profiles: &[Profile]) -> String {
    let mut name = base.to_owned();
    let mut n = 2;
    while profiles
        .iter()
        .any(|p| p.name.to_lowercase() == name.to_lowercase())
    {
        name = format!("{base}-{n}");
        n += 1;
    }
    name
}
pub fn prepare_profile(
    mut p: Profile,
    existing: &[Profile],
    supplied: &str,
    vault: &impl Credentials,
) -> Result<(Profile, String)> {
    let old = existing.iter().find(|old| old.id == p.id);
    if !p.id.is_empty() && old.is_none() {
        return Err(message("原配置不存在，请重新添加。"));
    }
    if old.is_some_and(|old| old.preset_id != p.preset_id) {
        return Err(message("已有配置不能更换厂商，请新增配置。"));
    }
    p.name = p.name.trim().to_owned();
    if existing
        .iter()
        .any(|other| other.id != p.id && other.name.to_lowercase() == p.name.to_lowercase())
    {
        return Err(message("配置名称已存在，请换一个名称。"));
    }
    if p.models.is_empty() || p.models.len() > 20 {
        return Err(message("请选择 1 到 20 个模型。"));
    }
    let mut seen = std::collections::HashSet::new();
    for m in &mut p.models {
        *m = m.trim().to_owned();
        if !seen.insert(m.clone()) {
            return Err(message("模型不能重复。"));
        }
    }
    // Never accept credential or route identities from the frontend.
    p.credential_id.clear();
    p.route_ids.clear();
    let mut candidate = Connection {
        id: String::new(),
        name: p.name.clone(),
        preset_id: p.preset_id.clone(),
        endpoint: p.endpoint.clone(),
        model: p.models[0].clone(),
        protocol: p.protocol,
        context_window: p.context_window,
        options: p.options.clone(),
    };
    candidate.validate()?;
    p.endpoint = candidate.endpoint.clone();
    p.options = candidate.options.clone();
    for m in &p.models {
        if m.is_empty() || m.len() > 200 {
            return Err(message("请填写有效的模型名称。"));
        }
        let route = p.model_route(m, "");
        crate::providers::normalize_address(&route.endpoint, route.options.full_url)?;
    }
    let key = if supplied.trim().is_empty() {
        let old = old.ok_or_else(|| message("请填写 API Key。"))?;
        if !same_credential_scope(&old.base_route("", ""), &candidate)
            || p.models.iter().any(|model| {
                !same_credential_scope(&old.model_route(model, ""), &p.model_route(model, ""))
            })
        {
            return Err(message("厂商或地址已改变，请重新填写 API Key。"));
        }
        p.credential_id = old.credential_id.clone();
        vault.read(&p.credential_id)?
    } else {
        p.credential_id = uuid::Uuid::new_v4().to_string();
        supplied.trim().to_owned()
    };
    if key.trim().is_empty() {
        return Err(message("密钥为空，请重新填写。"));
    }
    if p.id.is_empty() {
        p.id = uuid::Uuid::new_v4().to_string();
    }
    for model in &p.models {
        let reuse = old
            .filter(|o| {
                o.credential_id == p.credential_id
                    && o.endpoint == p.endpoint
                    && o.protocol == p.protocol
                    && o.context_window == p.context_window
                    && o.options == p.options
            })
            .and_then(|o| {
                o.models
                    .iter()
                    .position(|m| m == model)
                    .map(|i| o.route_ids[i].clone())
            });
        p.route_ids
            .push(reuse.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()));
    }
    Ok((p, key))
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
    fn draft() -> Profile {
        Profile {
            id: String::new(),
            name: "Kimi".into(),
            preset_id: "kimi".into(),
            endpoint: "https://example.com/v1/".into(),
            models: vec!["a".into(), "b".into()],
            protocol: crate::providers::Protocol::Chat,
            context_window: 32768,
            options: Default::default(),
            credential_id: "untrusted".into(),
            route_ids: vec!["untrusted".into()],
        }
    }
    #[test]
    fn model_routes_inherit_defaults_or_override_address_protocol_and_url_mode() {
        use crate::providers::{ModelOptions, Protocol};
        let mut p = draft();
        p.options.model_overrides.insert(
            "b".into(),
            ModelOptions {
                endpoint: Some("https://other.example/custom/invoke?version=2".into()),
                protocol: Some(Protocol::Responses),
                full_url: Some(true),
                ..Default::default()
            },
        );
        let (p, _) = prepare_profile(p, &[], "synthetic", &Vault::default()).unwrap();
        let persisted: Profile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        let routes = persisted.routes();
        assert_eq!(routes[0].protocol, Protocol::Chat);
        assert_eq!(
            routes[0].upstream_url("chat/completions").unwrap(),
            "https://example.com/v1/chat/completions"
        );
        assert_eq!(routes[1].protocol, Protocol::Responses);
        assert_eq!(
            routes[1].upstream_url("responses").unwrap(),
            "https://other.example/custom/invoke?version=2"
        );
        assert_eq!(p.endpoint, "https://example.com/v1");
        assert_eq!(p.protocol, Protocol::Chat);
    }
    #[test]
    fn model_address_changes_require_a_new_key_but_protocol_changes_do_not() {
        use crate::providers::{ModelOptions, Protocol};
        let vault = Vault::default();
        let (old, key) = prepare_profile(draft(), &[], "synthetic", &vault).unwrap();
        vault.write(&old.credential_id, &key).unwrap();
        let mut p = old.clone();
        p.options.model_overrides.insert(
            "a".into(),
            ModelOptions {
                protocol: Some(Protocol::Responses),
                ..Default::default()
            },
        );
        assert!(prepare_profile(p.clone(), std::slice::from_ref(&old), "", &vault).is_ok());
        p.options.model_overrides.get_mut("a").unwrap().endpoint =
            Some("https://other.example/v1/".into());
        assert!(prepare_profile(p.clone(), std::slice::from_ref(&old), "", &vault).is_err());
        let (saved, key) = prepare_profile(p, std::slice::from_ref(&old), "fresh", &vault).unwrap();
        assert_eq!(saved.routes()[0].endpoint, "https://other.example/v1");
        vault.write(&saved.credential_id, &key).unwrap();
        assert!(prepare_profile(saved.clone(), std::slice::from_ref(&saved), "", &vault).is_ok());
    }
    #[test]
    fn full_url_inheritance_and_override_validation_apply_before_requests() {
        use crate::providers::ModelOptions;
        let vault = Vault::default();
        let mut p = draft();
        p.endpoint = "https://example.com/v1/responses".into();
        p.options.full_url = true;
        p.options.model_overrides.insert(
            "b".into(),
            ModelOptions {
                full_url: Some(false),
                ..Default::default()
            },
        );
        let (saved, _) = prepare_profile(p.clone(), &[], "synthetic", &vault).unwrap();
        let routes = saved.routes();
        assert_eq!(routes[0].upstream_url("responses").unwrap(), p.endpoint);
        assert_eq!(
            routes[1].upstream_url("chat/completions").unwrap(),
            "https://example.com/v1/chat/completions"
        );
        for invalid in [
            "http://example.com/v1",
            "https://localhost/v1",
            "https://key@example.com/v1",
            "https://example.com/v1?key=secret",
        ] {
            p.options.model_overrides.get_mut("b").unwrap().endpoint = Some(invalid.into());
            assert!(prepare_profile(p.clone(), &[], "synthetic", &vault).is_err());
        }
    }
    #[test]
    fn one_profile_owns_one_key_and_multiple_unique_model_routes() {
        let vault = Vault::default();
        let (p, key) = prepare_profile(draft(), &[], " synthetic ", &vault).unwrap();
        assert_eq!(key, "synthetic");
        assert_ne!(p.credential_id, "untrusted");
        assert_eq!(p.routes().len(), 2);
        assert_ne!(p.route_ids[0], p.route_ids[1]);
        assert_eq!(p.routes()[0].endpoint, "https://example.com/v1");
        let catalog = crate::providers::catalog(&p.routes());
        assert_eq!(catalog["models"][0]["display_name"], "Kimi-a");
        assert_eq!(catalog["models"][1]["display_name"], "Kimi-b");
    }
    #[test]
    fn rename_reuses_key_and_routes_while_model_edits_only_add_new_routes() {
        let vault = Vault::default();
        let (old, key) = prepare_profile(draft(), &[], "synthetic", &vault).unwrap();
        vault.write(&old.credential_id, &key).unwrap();
        let mut edited = old.clone();
        edited.name = "Work".into();
        edited.models = vec!["b".into(), "c".into()];
        let (new, _) = prepare_profile(edited, std::slice::from_ref(&old), "", &vault).unwrap();
        assert_eq!(new.id, old.id);
        assert_eq!(new.credential_id, old.credential_id);
        assert_eq!(new.route_ids[0], old.route_ids[1]);
        assert!(!old.route_ids.contains(&new.route_ids[1]));
    }
    #[test]
    fn new_key_creates_revision_without_changing_profile_or_old_credentials() {
        let vault = Vault::default();
        let (old, key) = prepare_profile(draft(), &[], "old", &vault).unwrap();
        vault.write(&old.credential_id, &key).unwrap();
        let (new, key) =
            prepare_profile(old.clone(), std::slice::from_ref(&old), "new", &vault).unwrap();
        assert_eq!(new.id, old.id);
        assert_ne!(new.credential_id, old.credential_id);
        assert!(new.route_ids.iter().all(|id| !old.route_ids.contains(id)));
        assert_eq!(key, "new");
        assert_eq!(vault.read(&old.credential_id).unwrap(), "old");
    }
    #[test]
    fn names_are_trimmed_unique_case_insensitively_and_self_edit_is_allowed() {
        let vault = Vault::default();
        let (old, key) = prepare_profile(draft(), &[], "synthetic", &vault).unwrap();
        vault.write(&old.credential_id, &key).unwrap();
        let mut duplicate = draft();
        duplicate.name = " kIMI ".into();
        assert!(prepare_profile(duplicate, std::slice::from_ref(&old), "new", &vault).is_err());
        assert!(prepare_profile(old.clone(), std::slice::from_ref(&old), "", &vault).is_ok());
        assert_eq!(unique_name("Kimi", &[old]), "Kimi-2");
    }
    #[test]
    fn models_and_credential_scope_are_validated() {
        let vault = Vault::default();
        let (old, key) = prepare_profile(draft(), &[], "synthetic", &vault).unwrap();
        vault.write(&old.credential_id, &key).unwrap();
        for models in [
            vec![],
            vec![" ".into()],
            vec!["a".into(), " a ".into()],
            vec!["a".into(); 21],
        ] {
            let mut p = draft();
            p.models = models;
            assert!(prepare_profile(p, &[], "synthetic", &vault).is_err());
        }
        for field in ["provider", "endpoint"] {
            let mut p = old.clone();
            if field == "provider" {
                p.preset_id = "other".into();
            } else {
                p.endpoint = "https://other.example".into();
            }
            assert!(prepare_profile(p, std::slice::from_ref(&old), "", &vault).is_err());
        }
    }
    #[test]
    fn editing_cannot_change_vendor_even_with_a_fresh_key() {
        let vault = Vault::default();
        let (old, _) = prepare_profile(draft(), &[], "old", &vault).unwrap();
        for vendor in ["zhipu", "custom"] {
            let mut edited = old.clone();
            edited.preset_id = vendor.into();
            let error =
                prepare_profile(edited, std::slice::from_ref(&old), "new", &vault).unwrap_err();
            assert_eq!(error.to_string(), "已有配置不能更换厂商，请新增配置。");
        }
        let mut edited = old.clone();
        edited.name = "Work".into();
        edited.endpoint = "https://new.example/v1".into();
        edited.models = vec!["new-model".into()];
        let (saved, key) = prepare_profile(edited, &[old], "new", &vault).unwrap();
        assert_eq!(saved.preset_id, "kimi");
        assert_eq!(saved.models, ["new-model"]);
        assert_eq!(key, "new");
    }
    #[test]
    fn two_profiles_for_same_vendor_have_distinct_credentials_and_routes() {
        let vault = Vault::default();
        let (a, _) = prepare_profile(draft(), &[], "first", &vault).unwrap();
        let mut p = draft();
        p.name = "Kimi-2".into();
        let (b, _) = prepare_profile(p, std::slice::from_ref(&a), "second", &vault).unwrap();
        assert_ne!(a.credential_id, b.credential_id);
        assert!(a.route_ids.iter().all(|id| !b.route_ids.contains(id)));
    }
    #[test]
    fn enabled_mode_locks_configuration_changes() {
        assert!(ensure_editable(true)
            .unwrap_err()
            .to_string()
            .contains("请先关闭"));
        assert!(ensure_editable(false).is_ok());
    }
}
