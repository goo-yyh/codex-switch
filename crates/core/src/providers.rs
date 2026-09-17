use crate::{message, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Responses,
    Chat,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConnectionOptions {
    pub full_url: bool,
    pub endpoint_candidates: Vec<String>,
    pub model_overrides: std::collections::BTreeMap<String, ModelOptions>,
    pub chat_reasoning: Option<cc_switch_codex::ReasoningConfig>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModelOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Protocol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_url: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_levels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reasoning_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_modalities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    #[serde(default)]
    pub variants: Vec<EndpointVariant>,
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub model: String,
    pub protocol: Protocol,
    pub key_url: String,
    pub docs_url: String,
    pub env_key: String,
    pub context_window: u64,
    #[serde(default)]
    pub options: ConnectionOptions,
}

pub fn presets() -> Vec<Preset> {
    serde_json::from_str(include_str!(
        "../../../packages/provider-registry/providers.json"
    ))
    .expect("bundled registry")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub preset_id: String,
    pub endpoint: String,
    pub model: String,
    pub protocol: Protocol,
    pub context_window: u64,
    #[serde(default)]
    pub options: ConnectionOptions,
}
impl Connection {
    pub fn validate(&mut self) -> Result<()> {
        if self.name.trim().is_empty()
            || self.name.len() > 120
            || self.model.trim().is_empty()
            || self.model.len() > 200
        {
            return Err(message("请填写连接名称和模型。"));
        }
        if !self.id.is_empty() && uuid::Uuid::parse_str(&self.id).is_err() {
            return Err(message("连接标识无效。"));
        }
        self.endpoint = normalize_address(&self.endpoint, self.options.full_url)?;
        if self.options.endpoint_candidates.len() > 20 || self.options.model_overrides.len() > 100 {
            return Err(message("候选地址或模型能力配置过多。"));
        }
        self.options
            .endpoint_candidates
            .retain(|v| !v.trim().is_empty());
        for endpoint in &mut self.options.endpoint_candidates {
            *endpoint = normalize_address(endpoint, self.options.full_url)?;
        }
        for spec in self.options.model_overrides.values_mut() {
            if let Some(endpoint) = &spec.endpoint {
                spec.endpoint = if endpoint.trim().is_empty() {
                    None
                } else {
                    Some(normalize_address(
                        endpoint,
                        spec.full_url.unwrap_or(self.options.full_url),
                    )?)
                };
            }
            if spec
                .base_instructions
                .as_ref()
                .is_some_and(|s| s.len() > 65536)
            {
                return Err(message("模型说明过长。"));
            }
            if spec
                .context_window
                .is_some_and(|v| !(4096..=2_000_000).contains(&v))
            {
                return Err(message("模型上下文长度无效。"));
            }
            if let Some(levels) = &spec.reasoning_levels {
                if levels.is_empty()
                    || levels.iter().any(|l| {
                        ![
                            "none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
                        ]
                        .contains(&l.as_str())
                    })
                {
                    return Err(message("模型思考档位无效。"));
                }
                if spec
                    .default_reasoning_level
                    .as_ref()
                    .is_some_and(|d| !levels.contains(d))
                {
                    return Err(message("默认思考档位必须属于支持的档位。"));
                }
            } else if spec.default_reasoning_level.is_some() {
                return Err(message("请先设置支持的思考档位。"));
            }
            if spec.input_modalities.as_ref().is_some_and(|v| {
                v.is_empty()
                    || !v.iter().any(|m| m == "text")
                    || v.iter().any(|m| m != "text" && m != "image")
            }) {
                return Err(message("输入模态必须包含 text，可选 image。"));
            }
        }
        if self.context_window < 4096 || self.context_window > 2_000_000 {
            return Err(message("上下文长度应在 4096 到 2000000 之间。"));
        }
        Ok(())
    }
    pub fn alias(&self) -> String {
        format!("cs_{}", self.id.replace('-', ""))
    }
    pub fn upstream_url(&self, path: &str) -> Result<String> {
        if !self.options.full_url {
            return Ok(format!(
                "{}/{}",
                self.endpoint,
                path.trim_start_matches('/')
            ));
        }
        let mut url = url::Url::parse(&self.endpoint).map_err(|_| message("API 地址无效。"))?;
        if path == "responses/compact" {
            let p = url.path().trim_end_matches('/');
            if p.ends_with("/responses") {
                url.set_path(&format!("{p}/compact"));
            } else if !p.ends_with("/responses/compact") {
                return Err(message(
                    "完整 URL 无法推导压缩端点，请使用以 /responses 结尾的地址或基础地址模式。",
                ));
            }
        }
        Ok(url.to_string())
    }
}

pub fn normalize_endpoint(input: &str) -> Result<String> {
    normalize_address(input, false)
}
pub fn normalize_address(input: &str, full_url: bool) -> Result<String> {
    let mut u = url::Url::parse(input.trim())
        .map_err(|_| message("API 地址无效，请填写完整的 https 地址。"))?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || (!full_url && u.query().is_some())
        || u.fragment().is_some()
    {
        return Err(message(
            "服务地址必须使用 HTTPS，且不能包含账号、查询参数或片段。",
        ));
    }
    let host = u.host_str().unwrap_or_default();
    if host == "localhost" || host.ends_with(".localhost") || host == "127.0.0.1" || host == "[::1]"
    {
        return Err(message("不能将本地路由设置为上游服务。"));
    }
    if full_url {
        return Ok(u.to_string());
    }
    let path = u.path().trim_end_matches('/').to_string();
    let path = path
        .strip_suffix("/chat/completions")
        .or_else(|| path.strip_suffix("/responses"))
        .or_else(|| path.strip_suffix("/models"))
        .unwrap_or(&path);
    u.set_path(path);
    Ok(u.to_string().trim_end_matches('/').to_owned())
}

pub fn catalog(connections: &[Connection]) -> Value {
    json!({"models":connections.iter().map(|c| {
        let spec=c.options.model_overrides.get(&c.model).cloned().unwrap_or_default();
        let host=url::Url::parse(&c.endpoint).ok().and_then(|u|u.host_str().map(str::to_owned));
        let mut row=cc_switch_codex::catalog::entry(&c.model,&format!("{}-{}",c.name,c.model),spec.context_window.unwrap_or(c.context_window),&serde_json::to_value(spec).expect("model options"),c.protocol==Protocol::Responses,host.as_deref()==Some("api.deepseek.com"));
        row["slug"]=json!(c.alias()); row
    }).collect::<Vec<_>>()})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn url_preserves_prefix_without_duplicate_v1() {
        assert_eq!(
            normalize_endpoint("https://example.com/prefix/v1/chat/completions/").unwrap(),
            "https://example.com/prefix/v1"
        );
        assert_eq!(
            normalize_endpoint("https://example.com").unwrap(),
            "https://example.com"
        );
        for bad in [
            "http://example.com",
            "https://key@example.com",
            "https://example.com?k=secret",
            "https://localhost:123",
        ] {
            assert!(normalize_endpoint(bad).is_err());
        }
    }
    #[test]
    fn registry_has_exactly_five_unique_presets() {
        let p = presets();
        assert_eq!(p.len(), 5);
        let mut ids = p.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5);
        for p in p {
            assert!(normalize_endpoint(&p.endpoint).is_ok());
            assert_eq!(p.env_key, format!("{}_key", p.id));
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RoutingSettings {
    pub remote_compaction: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointVariant {
    pub name: String,
    pub endpoint: String,
    pub model: String,
    pub protocol: Protocol,
    pub context_window: u64,
    #[serde(default)]
    pub options: ConnectionOptions,
}
