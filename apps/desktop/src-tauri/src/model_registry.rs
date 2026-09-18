//! Public model metadata is additive. It never writes profiles, credentials or Codex files.
use crate::state::{err, AppState, CommandResult};
use codex_switch_core::{
    providers::{presets, Connection, ConnectionOptions, ModelOptions, Preset},
    store::Store,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex, time::Duration};
use tauri::{Emitter, Manager};

const URL: &str = "https://www.codex-switch.com/registry/models-v1.json";
const SCHEMA_VERSION: u32 = 1;
const MAX_MODELS_PER_SCOPE: usize = 100;
const CACHE_KEY: &str = "model_registry_v1";
const MAX_BYTES: usize = 1024 * 1024;
const MAX_CANDIDATES: usize = 5;
// These IDs need new successful endpoint evidence before they can be reintroduced.
const EXCLUDED_QIANWEN: &[&str] = &[
    "qwen3.6-27b",
    "qwen3-235b-a22b-instruct-2507",
    "qwen3-14b",
    "qwen-max",
    "qwen-coder-plus",
    "qwen-coder-turbo",
    "qwq-plus",
    "qwen-long",
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Catalog {
    schema_version: u32,
    version: u32,
    pub(crate) candidates: BTreeMap<String, Vec<String>>,
    pub(crate) presets: Vec<Preset>,
}
pub(crate) struct RegistryState(pub(crate) Mutex<Catalog>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteCatalog {
    schema_version: u32,
    version: u32,
    models: BTreeMap<String, Vec<String>>,
    providers: Vec<Preset>,
}

impl RemoteCatalog {
    fn is_newer_than(&self, version: u32) -> CommandResult<bool> {
        if self.schema_version != SCHEMA_VERSION || self.version == 0 || self.providers.len() > 20 {
            return Err("Unsupported model registry".into());
        }
        // Compare before cloning or merging; the common same-version check allocates no cache.
        Ok(self.version > version)
    }
}

#[derive(Deserialize)]
struct RegistryVersion {
    version: u32,
}

impl Catalog {
    fn bundled() -> Self {
        let presets = presets();
        let mut candidates: BTreeMap<String, Vec<String>> = serde_json::from_str(include_str!(
            "../../../../packages/provider-registry/models.json"
        ))
        .expect("bundled model candidates");
        for preset in &presets {
            for (index, variant) in preset.variants.iter().enumerate().skip(1) {
                candidates.insert(
                    candidate_key(&preset.id, index),
                    variant
                        .options
                        .model_overrides
                        .keys()
                        .take(MAX_CANDIDATES)
                        .cloned()
                        .collect(),
                );
            }
        }
        Self {
            schema_version: SCHEMA_VERSION,
            version: serde_json::from_str::<RegistryVersion>(include_str!(
                "../../../../packages/provider-registry/version.json"
            ))
            .expect("bundled registry version")
            .version,
            candidates,
            presets,
        }
    }

    pub(crate) fn load(store: &Store) -> Self {
        let mut catalog = Self::bundled();
        // A corrupt/obsolete cache must never prevent the application from starting.
        let cached = store
            .setting(CACHE_KEY)
            .ok()
            .flatten()
            .filter(|raw| raw.len() <= MAX_BYTES)
            .and_then(|raw| serde_json::from_str::<Self>(&raw).ok())
            .filter(|cached| {
                cached.schema_version == SCHEMA_VERSION && cached.version >= catalog.version
            });
        let Some(cached) = cached else { return catalog };
        for local in &mut catalog.presets {
            let Some(source) = cached.presets.iter().find(|p| p.id == local.id) else {
                continue;
            };
            if source.endpoint == local.endpoint && source.protocol == local.protocol {
                restore_scope(
                    &local.id,
                    &local.endpoint,
                    local.protocol,
                    &mut local.options,
                    catalog.candidates.get_mut(&local.id).unwrap(),
                    cached.candidates.get(&local.id),
                    &source.options,
                );
            }
            for (index, variant) in local.variants.iter_mut().enumerate() {
                let Some(source_variant) = source
                    .variants
                    .iter()
                    .find(|v| v.endpoint == variant.endpoint && v.protocol == variant.protocol)
                else {
                    continue;
                };
                let key = candidate_key(&local.id, index);
                restore_scope(
                    &local.id,
                    &variant.endpoint,
                    variant.protocol,
                    &mut variant.options,
                    catalog.candidates.get_mut(&key).unwrap(),
                    cached.candidates.get(&key),
                    &source_variant.options,
                );
            }
        }
        catalog.version = cached.version;
        catalog
    }

    fn merge(&mut self, remote: RemoteCatalog) -> CommandResult<()> {
        for local in &mut self.presets {
            let Some(source) = remote.providers.iter().find(|p| p.id == local.id) else {
                continue;
            };
            // Only existing service scopes are supported. Remote data cannot redirect API keys.
            if source.endpoint == local.endpoint && source.protocol == local.protocol {
                if let Some(ids) = remote.models.get(&local.id) {
                    merge_models(
                        &local.id,
                        &local.endpoint,
                        local.protocol,
                        &mut local.options,
                        self.candidates.entry(local.id.clone()).or_default(),
                        ids,
                        &source.options,
                    )?;
                }
            }
            for (index, variant) in local.variants.iter_mut().enumerate() {
                let Some(source_variant) = source
                    .variants
                    .iter()
                    .find(|v| v.endpoint == variant.endpoint && v.protocol == variant.protocol)
                else {
                    continue;
                };
                let ids = if index == 0 {
                    remote.models.get(&local.id).cloned().unwrap_or_default()
                } else {
                    source_variant
                        .options
                        .model_overrides
                        .keys()
                        .cloned()
                        .collect()
                };
                let mut defaults = source_variant.options.clone();
                if source.endpoint == variant.endpoint && source.protocol == variant.protocol {
                    for (id, spec) in &source.options.model_overrides {
                        defaults
                            .model_overrides
                            .entry(id.clone())
                            .or_insert_with(|| spec.clone());
                    }
                }
                let key = candidate_key(&local.id, index);
                merge_models(
                    &local.id,
                    &variant.endpoint,
                    variant.protocol,
                    &mut variant.options,
                    self.candidates.entry(key).or_default(),
                    &ids,
                    &defaults,
                )?;
            }
        }
        self.version = remote.version;
        Ok(())
    }
}

// The first variant shares the standard list; additional variants are separate plans.
fn candidate_key(provider: &str, variant: usize) -> String {
    if variant == 0 {
        provider.into()
    } else {
        format!("{provider}:{variant}")
    }
}

fn allowed_id(provider: &str, id: &str) -> bool {
    id.trim() == id
        && !id.is_empty()
        && id.len() <= 200
        && !(provider == "qianwen" && EXCLUDED_QIANWEN.contains(&id))
}

fn restore_scope(
    provider: &str,
    endpoint: &str,
    protocol: codex_switch_core::providers::Protocol,
    local: &mut ConnectionOptions,
    candidates: &mut Vec<String>,
    order: Option<&Vec<String>>,
    cached: &ConnectionOptions,
) {
    // Keep previously downloaded defaults even after a model leaves the five recommendations.
    for (id, spec) in cached.model_overrides.iter().take(MAX_MODELS_PER_SCOPE) {
        if local.model_overrides.len() >= MAX_MODELS_PER_SCOPE {
            break;
        }
        if allowed_id(provider, id) && valid_spec(id, endpoint, protocol, spec) {
            local
                .model_overrides
                .entry(id.clone())
                .or_insert_with(|| spec.clone());
        }
    }
    if let Some(order) = order {
        let mut sorted = Vec::new();
        for id in order.iter().chain(candidates.iter()) {
            if local.model_overrides.contains_key(id)
                && allowed_id(provider, id)
                && !sorted.contains(id)
            {
                sorted.push(id.clone());
            }
        }
        sorted.truncate(MAX_CANDIDATES);
        *candidates = sorted;
    }
}

fn valid_spec(
    id: &str,
    endpoint: &str,
    protocol: codex_switch_core::providers::Protocol,
    spec: &ModelOptions,
) -> bool {
    // Context values are already the published 80% preset; do not scale them again.
    if spec.context_window.is_none()
        || spec.endpoint.is_some()
        || spec.protocol.is_some()
        || spec.full_url.is_some()
        || spec.base_instructions.is_some()
    {
        return false;
    }
    let mut connection = Connection {
        id: String::new(),
        name: "registry".into(),
        preset_id: String::new(),
        endpoint: endpoint.into(),
        model: id.into(),
        protocol,
        context_window: 32768,
        options: ConnectionOptions {
            model_overrides: BTreeMap::from([(id.into(), spec.clone())]),
            ..Default::default()
        },
    };
    connection.validate().is_ok()
}

fn merge_models(
    provider: &str,
    endpoint: &str,
    protocol: codex_switch_core::providers::Protocol,
    local: &mut ConnectionOptions,
    candidates: &mut Vec<String>,
    ids: &[String],
    remote: &ConnectionOptions,
) -> CommandResult<()> {
    if ids.len() > MAX_MODELS_PER_SCOPE || remote.model_overrides.len() > MAX_MODELS_PER_SCOPE {
        return Err("Model registry is too large".into());
    }
    let mut added = Vec::new();
    for id in ids.iter().take(MAX_CANDIDATES) {
        if !allowed_id(provider, id) {
            continue;
        }
        let Some(spec) = remote.model_overrides.get(id) else {
            continue;
        };
        if !valid_spec(id, endpoint, protocol, spec) {
            continue;
        }
        if !local.model_overrides.contains_key(id) {
            if local.model_overrides.len() >= MAX_MODELS_PER_SCOPE {
                continue;
            }
            local.model_overrides.insert(id.clone(), spec.clone());
            if !candidates.contains(id) && !added.contains(id) {
                added.push(id.clone());
            }
        }
    }
    // Only new IDs move into the recommendation list. Existing metadata is never overwritten.
    added.extend(candidates.iter().cloned());
    added.truncate(MAX_CANDIDATES);
    *candidates = added;
    Ok(())
}

async fn fetch() -> CommandResult<RemoteCatalog> {
    fetch_from(URL).await
}

async fn fetch_from(url: &str) -> CommandResult<RemoteCatalog> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(err)?;
    let mut response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?;
    if response.status() != reqwest::StatusCode::OK {
        return Err("Model registry unavailable".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(err)? {
        if bytes.len() + chunk.len() > MAX_BYTES {
            return Err("Model registry is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(err)
}

fn apply(store: &Store, current: &mut Catalog, remote: RemoteCatalog) -> CommandResult<bool> {
    // Commit cache and in-memory state together; failed validation/storage preserves the old catalog.
    if !remote.is_newer_than(current.version)? {
        return Ok(false);
    }
    let mut next = current.clone();
    next.merge(remote)?;
    let json = serde_json::to_string(&next).map_err(err)?;
    if json.len() > MAX_BYTES {
        return Err("Model registry cache is too large".into());
    }
    store.set(CACHE_KEY, &json).map_err(err)?;
    *current = next;
    Ok(true)
}

pub(crate) fn refresh_on_startup(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let result: CommandResult<bool> = async {
            // No service-operation lock or Store guard is held during the network request.
            let remote = fetch().await?;
            let state = app.state::<AppState>();
            let store = state.store.lock().map_err(err)?;
            let registry = app.state::<RegistryState>();
            let mut current = registry.0.lock().map_err(err)?;
            apply(&store, &mut current, remote)
        }
        .await;
        if matches!(result, Ok(true)) {
            let _ = app.emit("model-registry-updated", ());
        }
        // Offline checks are quiet: the bundled/cached list is already available.
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_switch_core::providers::Protocol;

    fn remote(version_increment: u32) -> RemoteCatalog {
        RemoteCatalog {
            schema_version: SCHEMA_VERSION,
            version: Catalog::bundled().version + version_increment,
            models: Catalog::bundled().candidates,
            providers: presets(),
        }
    }
    fn spec(context: u64) -> ModelOptions {
        ModelOptions {
            context_window: Some(context),
            reasoning_levels: Some(vec!["high".into()]),
            default_reasoning_level: Some("high".into()),
            input_modalities: Some(vec!["text".into()]),
            ..Default::default()
        }
    }
    fn add(source: &mut RemoteCatalog, id: &str) {
        source.models.get_mut("zhipu").unwrap().insert(0, id.into());
        source.models.get_mut("zhipu").unwrap().truncate(5);
        let provider = source
            .providers
            .iter_mut()
            .find(|p| p.id == "zhipu")
            .unwrap();
        provider
            .options
            .model_overrides
            .insert(id.into(), spec(131072));
        provider.variants[0]
            .options
            .model_overrides
            .insert(id.into(), spec(131072));
    }
    fn temporary_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("connections.db")).unwrap();
        (dir, store)
    }

    #[test]
    fn bundled_version_skips_first_sync_and_cached_versions_never_downgrade() {
        let (_dir, store) = temporary_store();
        let mut catalog = Catalog::load(&store);
        let bundled_version = Catalog::bundled().version;
        assert_eq!(catalog.version, bundled_version);
        assert!(!apply(&store, &mut catalog, remote(0)).unwrap());
        assert_eq!(store.setting(CACHE_KEY).unwrap(), None);

        let mut newer = remote(2);
        add(&mut newer, "test-new-version");
        assert!(apply(&store, &mut catalog, newer).unwrap());
        let before = store.setting(CACHE_KEY).unwrap();
        let mut catalog = Catalog::load(&store);
        assert_eq!(catalog.version, bundled_version + 2);
        for version in [0, 2] {
            let mut same_or_older = remote(version);
            add(&mut same_or_older, "test-skip");
            assert!(!apply(&store, &mut catalog, same_or_older).unwrap());
            assert_eq!(store.setting(CACHE_KEY).unwrap(), before);
            assert!(!catalog.candidates["zhipu"].contains(&"test-skip".into()));
            assert_eq!(catalog.candidates["zhipu"][0], "test-new-version");
        }
    }

    #[test]
    fn missing_or_zero_data_version_cannot_be_confused_with_schema_version() {
        let legacy = serde_json::json!({
            "schemaVersion": 1,
            "revision": "a".repeat(64),
            "models": {},
            "providers": []
        });
        assert!(serde_json::from_value::<RemoteCatalog>(legacy).is_err());
        let (_dir, store) = temporary_store();
        let mut catalog = Catalog::load(&store);
        let mut invalid = remote(0);
        invalid.version = 0;
        assert!(apply(&store, &mut catalog, invalid).is_err());
        assert_eq!(catalog.version, Catalog::bundled().version);
        assert_eq!(store.setting(CACHE_KEY).unwrap(), None);
    }

    #[test]
    fn adds_new_models_without_changing_existing_defaults_or_user_profiles() {
        let (_dir, store) = temporary_store();
        let profile = codex_switch_core::profiles::Profile {
            id: "user".into(),
            name: "User settings".into(),
            preset_id: "zhipu".into(),
            endpoint: "https://custom.example/v1".into(),
            models: vec!["test-new".into()],
            protocol: Protocol::Responses,
            context_window: 65536,
            options: ConnectionOptions {
                model_overrides: BTreeMap::from([("test-new".into(), spec(65536))]),
                ..Default::default()
            },
            credential_id: "saved-key-reference".into(),
            route_ids: vec![],
        };
        store.save_profile(&profile).unwrap();
        store.select_profiles(&["user".into()]).unwrap();
        let before = serde_json::to_value(store.profiles().unwrap()).unwrap();
        let mut catalog = Catalog::load(&store);
        let previous = serde_json::to_value(&catalog.presets[0]).unwrap();
        let mut source = remote(2);
        add(&mut source, "test-new");
        source.providers[0]
            .options
            .model_overrides
            .insert("glm-5.3".into(), spec(4096));
        assert!(apply(&store, &mut catalog, source).unwrap());
        assert_eq!(catalog.candidates["zhipu"][0], "test-new");
        assert_eq!(catalog.candidates["zhipu"].len(), 5);
        assert_eq!(
            serde_json::to_value(&catalog.presets[0]).unwrap()["options"]["modelOverrides"]
                ["glm-5.3"],
            previous["options"]["modelOverrides"]["glm-5.3"]
        );
        assert_eq!(catalog.presets[0].model, "glm-5.3");
        assert_eq!(
            serde_json::to_value(store.profiles().unwrap()).unwrap(),
            before
        );
        assert_eq!(store.selected_profiles().unwrap(), ["user"]);
    }

    #[test]
    fn normal_and_subscription_models_keep_independent_defaults() {
        let (_dir, store) = temporary_store();
        let mut catalog = Catalog::load(&store);
        let mut source = remote(3);
        add(&mut source, "test-shared-id");
        source.providers[0].variants[1]
            .options
            .model_overrides
            .insert("test-shared-id".into(), spec(65536));
        source.providers[0].variants[1]
            .options
            .model_overrides
            .insert("test-plan-only".into(), spec(262144));
        apply(&store, &mut catalog, source).unwrap();
        assert!(!catalog.candidates["zhipu"].contains(&"test-plan-only".into()));
        assert!(catalog.candidates["zhipu:1"].contains(&"test-plan-only".into()));
        assert_eq!(
            catalog.presets[0].options.model_overrides["test-shared-id"].context_window,
            Some(131072)
        );
        assert_eq!(
            catalog.presets[0].variants[1].options.model_overrides["test-shared-id"].context_window,
            Some(65536)
        );
    }

    #[test]
    fn cache_survives_reopening_and_keeps_defaults_outside_recommendations() {
        let (dir, store) = temporary_store();
        let mut catalog = Catalog::load(&store);
        for i in 1..=6 {
            let mut source = remote(i + 1);
            add(&mut source, &format!("test-new-{i}"));
            apply(&store, &mut catalog, source).unwrap();
        }
        assert!(!catalog.candidates["zhipu"].contains(&"test-new-1".into()));
        drop(store);
        let store = Store::open(&dir.path().join("connections.db")).unwrap();
        let mut restored = Catalog::load(&store);
        assert_eq!(restored.candidates, catalog.candidates);
        assert_eq!(
            restored.presets[0].options.model_overrides["test-new-1"],
            spec(131072)
        );
        let mut source = remote(8);
        add(&mut source, "test-new-6");
        source.providers[0]
            .options
            .model_overrides
            .insert("test-new-6".into(), spec(4096));
        apply(&store, &mut restored, source).unwrap();
        assert_eq!(
            restored.presets[0].options.model_overrides["test-new-6"],
            spec(131072)
        );
    }

    #[test]
    fn rejects_incompatible_updates_atomically_and_ignores_corrupt_cache() {
        let (_dir, store) = temporary_store();
        let mut catalog = Catalog::load(&store);
        apply(&store, &mut catalog, remote(2)).unwrap();
        let before = store.setting(CACHE_KEY).unwrap();
        let mut source = remote(3);
        add(&mut source, "test-not-committed");
        source
            .models
            .insert("qianwen".into(), vec!["oversized".into(); 101]);
        assert!(apply(&store, &mut catalog, source).is_err());
        assert_eq!(store.setting(CACHE_KEY).unwrap(), before);
        assert!(!catalog.candidates["zhipu"].contains(&"test-not-committed".into()));
        let mut source = remote(3);
        source.schema_version = 2;
        assert!(apply(&store, &mut catalog, source).is_err());
        store.set(CACHE_KEY, "invalid JSON").unwrap();
        assert_eq!(
            Catalog::load(&store).candidates,
            Catalog::bundled().candidates
        );
    }

    #[test]
    fn ignores_invalid_capabilities_routing_changes_and_excluded_models() {
        let (_dir, store) = temporary_store();
        let mut catalog = Catalog::load(&store);
        let mut source = remote(9);
        add(&mut source, "test-new");
        for provider in &mut source.providers {
            if provider.id == "zhipu" {
                provider
                    .options
                    .model_overrides
                    .get_mut("test-new")
                    .unwrap()
                    .endpoint = Some("https://different.example/v1".into());
                provider.variants[0]
                    .options
                    .model_overrides
                    .get_mut("test-new")
                    .unwrap()
                    .context_window = Some(1);
                provider.endpoint = "https://different.example/v1".into();
            }
            if provider.id == "qianwen" {
                provider
                    .options
                    .model_overrides
                    .insert("qwen-max".into(), spec(131072));
                source
                    .models
                    .insert("qianwen".into(), vec!["qwen-max".into()]);
            }
        }
        apply(&store, &mut catalog, source).unwrap();
        assert!(!catalog.candidates["zhipu"].contains(&"test-new".into()));
        assert!(!catalog.candidates["qianwen"].contains(&"qwen-max".into()));
        assert_eq!(catalog.presets[0].endpoint, presets()[0].endpoint);
    }

    #[test]
    fn same_scope_variant_inherits_new_primary_defaults() {
        let (_dir, store) = temporary_store();
        let mut catalog = Catalog::load(&store);
        let mut source = remote(10);
        add(&mut source, "test-primary-only");
        source.providers[0].variants[0]
            .options
            .model_overrides
            .remove("test-primary-only");
        apply(&store, &mut catalog, source).unwrap();
        assert_eq!(
            catalog.presets[0].variants[0].options.model_overrides["test-primary-only"],
            spec(131072)
        );
    }

    #[tokio::test]
    async fn failed_http_invalid_json_and_oversized_payload_leave_cache_usable() {
        use std::io::{Read, Write};
        let (_dir, store) = temporary_store();
        let mut catalog = Catalog::load(&store);
        let mut good = remote(2);
        add(&mut good, "test-offline");
        apply(&store, &mut catalog, good).unwrap();
        let before = store.setting(CACHE_KEY).unwrap();
        for (status, body) in [
            ("503 Service Unavailable", "offline".to_owned()),
            ("200 OK", "<html>not json</html>".to_owned()),
            ("200 OK", " ".repeat(MAX_BYTES + 1)),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/models.json", listener.local_addr().unwrap());
            let worker = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let _ = stream.read(&mut [0; 4096]);
                let header = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(body.as_bytes());
            });
            assert!(fetch_from(&url).await.is_err());
            worker.join().unwrap();
            assert_eq!(store.setting(CACHE_KEY).unwrap(), before);
            assert_eq!(Catalog::load(&store).candidates["zhipu"][0], "test-offline");
        }
    }

    #[tokio::test]
    #[ignore = "Explicit read-only check of the public model CDN; no provider API calls"]
    async fn live_public_registry_can_be_downloaded_and_merged() {
        let (_dir, store) = temporary_store();
        let mut catalog = Catalog::load(&store);
        let remote = fetch().await.unwrap();
        let expected_version = remote.version.max(catalog.version);
        apply(&store, &mut catalog, remote).unwrap();
        assert_eq!(catalog.version, expected_version);
        assert_eq!(catalog.presets.len(), 5);
    }
}
