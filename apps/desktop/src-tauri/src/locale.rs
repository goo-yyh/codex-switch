//! UI language is an app preference, independent of providers and Codex configuration.
use codex_switch_core::store::Store;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum Locale {
    #[default]
    #[serde(rename = "zh-CN")]
    Chinese,
    #[serde(rename = "en")]
    English,
}
impl Locale {
    pub(crate) fn read(store: &Store) -> codex_switch_core::Result<Self> {
        Ok(match store.setting("locale")?.as_deref() {
            Some("en") => Self::English,
            _ => Self::Chinese,
        })
    }
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Chinese => "zh-CN",
            Self::English => "en",
        }
    }
    pub(crate) fn text<'a>(self, zh: &'a str, en: &'a str) -> &'a str {
        match self {
            Self::Chinese => zh,
            Self::English => en,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_defaults_to_chinese_and_survives_reopening() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preferences.db");
        let store = Store::open(&path).unwrap();
        assert_eq!(Locale::read(&store).unwrap(), Locale::Chinese);
        store.set("locale", Locale::English.code()).unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_eq!(Locale::read(&store).unwrap(), Locale::English);
        store.set("locale", Locale::Chinese.code()).unwrap();
        assert_eq!(Locale::read(&store).unwrap(), Locale::Chinese);
        assert!(serde_json::from_str::<Locale>("\"invalid\"").is_err());
    }
}
