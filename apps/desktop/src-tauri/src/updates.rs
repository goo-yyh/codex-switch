//! Read public release metadata only. Installation stays under the user's control.
use serde::{Deserialize, Serialize};
use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const REPOSITORY: &str = "goo-yyh/codex-switch";
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const RETRY_INTERVAL: Duration = Duration::from_secs(15 * 60);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AvailableUpdate {
    version: String,
    url: String,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    state: String,
    size: u64,
}

type UpdateResult = Result<Option<AvailableUpdate>, String>;
struct CachedCheck {
    checked_at: Instant,
    result: UpdateResult,
}

// Tauri installers include architecture tokens in their names. Never fall back
// to the first asset: it may be a different architecture or a detached signature.
fn installer_rank(name: &str, os: &str, arch: &str) -> Option<u8> {
    let extension = match os {
        "macos" => ".dmg",
        "windows" => ".exe",
        _ => return None,
    };
    let name = name.to_ascii_lowercase().replace("x86_64", "x64");
    if !name.ends_with(extension) {
        return None;
    }
    let tokens: Vec<_> = name.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    let architectures: Vec<_> = tokens
        .iter()
        .filter_map(|token| match *token {
            "aarch64" | "arm64" => Some("aarch64"),
            "x64" | "amd64" => Some("x86_64"),
            "x86" | "i686" | "i386" => Some("x86"),
            _ => None,
        })
        .collect();
    if !architectures.is_empty() {
        return architectures
            .iter()
            .all(|candidate| *candidate == arch)
            .then_some(0);
    }
    (os == "macos" && matches!(arch, "aarch64" | "x86_64") && tokens.contains(&"universal"))
        .then_some(1)
}

fn available_update(release: Release, current: &str, os: &str, arch: &str) -> UpdateResult {
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let tag = release.tag_name.trim();
    let Ok(mut latest) = semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag)) else {
        return Ok(None);
    };
    let mut current = semver::Version::parse(current).map_err(|e| e.to_string())?;
    // Build metadata does not identify a newer release. Never offer prereleases,
    // even if a release was accidentally published without GitHub's prerelease flag.
    if !latest.pre.is_empty() {
        return Ok(None);
    }
    latest.build = semver::BuildMetadata::EMPTY;
    current.build = semver::BuildMetadata::EMPTY;
    if latest <= current {
        return Ok(None);
    }
    let asset = release
        .assets
        .iter()
        .filter(|asset| asset.state == "uploaded" && asset.size > 0)
        .filter_map(|asset| installer_rank(&asset.name, os, arch).map(|rank| (rank, asset)))
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, asset)| asset);
    let Some(asset) = asset else { return Ok(None) };
    // Open the installer directly on our repository, never a payload-provided host.
    let mut url = url::Url::parse(&format!(
        "https://github.com/{REPOSITORY}/releases/download/"
    ))
    .map_err(|e| e.to_string())?;
    url.path_segments_mut()
        .map_err(|_| "Invalid download URL")?
        .pop_if_empty()
        .push(tag)
        .push(&asset.name);
    Ok(Some(AvailableUpdate {
        version: latest.to_string(),
        url: url.to_string(),
    }))
}

async fn fetch_update(current: &str) -> UpdateResult {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(format!("Codex-Switch/{current}"))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(format!(
            "https://api.github.com/repos/{REPOSITORY}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    // A repository with no public release is expected during initial development.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err("Release check unavailable".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err("Release metadata too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let release = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    available_update(
        release,
        current,
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

#[tauri::command]
pub(crate) async fn check_update(app: tauri::AppHandle) -> UpdateResult {
    static CACHE: OnceLock<Mutex<Option<CachedCheck>>> = OnceLock::new();
    // Independent of the service-operation lock: an offline update check must not
    // delay opening the panel, editing configurations or stopping the service.
    let mut cache = CACHE.get_or_init(|| Mutex::new(None)).lock().await;
    if let Some(entry) = cache.as_ref() {
        let ttl = if entry.result.is_ok() {
            CHECK_INTERVAL
        } else {
            RETRY_INTERVAL
        };
        if entry.checked_at.elapsed() < ttl {
            return entry.result.clone();
        }
    }
    let result = fetch_update(&app.package_info().version.to_string()).await;
    *cache = Some(CachedCheck {
        checked_at: Instant::now(),
        result: result.clone(),
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release(tag: &str, name: &str) -> Release {
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
            assets: vec![Asset {
                name: name.into(),
                state: "uploaded".into(),
                size: 100,
            }],
        }
    }
    #[test]
    fn compares_versions_numerically_and_links_directly_to_the_installer() {
        let update = available_update(
            release("v0.10.0", "Codex_0.2.0_aarch64.dmg"),
            "0.9.0",
            "macos",
            "aarch64",
        )
        .unwrap()
        .unwrap();
        assert_eq!(update.version, "0.10.0");
        assert_eq!(
            update.url,
            "https://github.com/goo-yyh/codex-switch/releases/download/v0.10.0/Codex_0.2.0_aarch64.dmg"
        );
        assert!(available_update(
            release("0.2.0", "Codex_0.2.0_x64-setup.exe"),
            "0.1.0",
            "windows",
            "x86_64"
        )
        .unwrap()
        .is_some());
    }
    #[test]
    fn skips_old_invalid_prerelease_and_build_only_versions() {
        for tag in [
            "v0.1.0",
            "v0.0.9",
            "v0.1.0+build2",
            "v0.2.0-beta.1",
            "nightly",
        ] {
            assert!(
                available_update(
                    release(tag, "Codex_0.2.0_aarch64.dmg"),
                    "0.1.0",
                    "macos",
                    "aarch64"
                )
                .unwrap()
                .is_none(),
                "{tag}"
            );
        }
        for draft in [true, false] {
            let mut item = release("v0.2.0", "Codex_0.2.0_aarch64.dmg");
            item.draft = draft;
            item.prerelease = !draft;
            assert!(available_update(item, "0.1.0", "macos", "aarch64")
                .unwrap()
                .is_none());
        }
    }
    #[test]
    fn requires_an_uploaded_nonempty_installer_for_the_current_platform() {
        for name in ["source.zip", "Codex_0.2.0_x64-setup.exe", "Codex.dmg.sig"] {
            assert!(
                available_update(release("v0.2.0", name), "0.1.0", "macos", "aarch64")
                    .unwrap()
                    .is_none()
            );
        }
        for (state, size) in [("starter", 100), ("uploaded", 0)] {
            let mut item = release("v0.2.0", "Codex_0.2.0_aarch64.dmg");
            item.assets[0].state = state.into();
            item.assets[0].size = size;
            assert!(available_update(item, "0.1.0", "macos", "aarch64")
                .unwrap()
                .is_none());
        }
        let product: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../packages/product-info/product.json"
        ))
        .unwrap();
        assert_eq!(
            product["repository"],
            format!("https://github.com/{REPOSITORY}")
        );
    }
    #[test]
    fn matches_architecture_tokens_and_prefers_native_over_universal() {
        for (file, os, arch, rank) in [
            ("Codex_0.2.0_aarch64.dmg", "macos", "aarch64", Some(0)),
            ("Codex_0.2.0_x64.dmg", "macos", "x86_64", Some(0)),
            ("Codex_0.2.0_x86_64.dmg", "macos", "x86_64", Some(0)),
            ("Codex_0.2.0_arm64-setup.exe", "windows", "aarch64", Some(0)),
            ("Codex_0.2.0_x64-setup.exe", "windows", "x86_64", Some(0)),
            ("Codex_0.2.0_x86-setup.exe", "windows", "x86", Some(0)),
            ("Codex_0.2.0_universal.dmg", "macos", "aarch64", Some(1)),
            ("Codex_0.2.0_x64.dmg", "macos", "aarch64", None),
            ("Codex_0.2.0_x86_64-setup.exe", "windows", "x86", None),
            ("Codex_0.2.0_arm64_x64.dmg", "macos", "aarch64", None),
            ("Codex_0.2.0.dmg", "macos", "aarch64", None),
            ("Codex_0.2.0_aarch64.dmg.sig", "macos", "aarch64", None),
        ] {
            assert_eq!(installer_rank(file, os, arch), rank, "{file}");
        }
        let mut item = release("v0.2.0", "Codex_0.2.0_universal.dmg");
        item.assets
            .extend(release("v0.2.0", "Codex_0.2.0_x64.dmg").assets);
        item.assets
            .extend(release("v0.2.0", "Codex_0.2.0_aarch64.dmg").assets);
        let result = available_update(item, "0.1.0", "macos", "aarch64")
            .unwrap()
            .unwrap();
        assert!(result.url.ends_with("/Codex_0.2.0_aarch64.dmg"));
    }
}
