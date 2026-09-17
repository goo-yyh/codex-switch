//! Explicit connectivity probes; saving a profile never calls these functions.
use super::{bounded_json, http_client};
use crate::{
    message, protocol,
    providers::{Connection, Protocol},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProbeReceipt {
    pub ok: bool,
    pub status: Option<u16>,
    pub message: String,
    pub elapsed_ms: u128,
}
pub fn status_message(status: u16) -> &'static str {
    match status {
        401 | 403 => "密钥无效，或当前套餐没有此权限。",
        404 => "服务地址或模型不存在。",
        429 => "服务限流或可用额度不足。",
        400 | 422 => "模型或请求参数不兼容，请检查套餐与接口格式。",
        _ => "供应商暂时无法处理请求。",
    }
}
pub async fn probe(c: &Connection, key: &str) -> ProbeReceipt {
    let start = Instant::now();
    let inner = async {
        let response_body = json!({"model":c.model,"input":"Reply with exactly OK.","max_output_tokens":256,"stream":false});
        let body = if c.protocol == Protocol::Chat {
            protocol::chat_request_for(
                &response_body,
                c.options.chat_reasoning.as_ref(),
                &c.endpoint,
            )?
            .0
        } else {
            response_body
        };
        let path = if c.protocol == Protocol::Chat {
            "chat/completions"
        } else {
            "responses"
        };
        let client = http_client()?;
        let r = client
            .post(c.upstream_url(path)?)
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|_| message("连接失败，请检查网络或 API 地址。"))?;
        let status = r.status().as_u16();
        if !(200..300).contains(&status) {
            return Ok((false, Some(status), status_message(status).to_owned()));
        }
        let v = bounded_json(r).await?;
        let valid = if c.protocol == Protocol::Chat {
            v["choices"][0]["message"]["content"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
                && v["choices"][0]["finish_reason"] == "stop"
        } else {
            v["status"] == "completed" && v["output"].as_array().is_some_and(|a| !a.is_empty())
        };
        Ok::<_, crate::Error>((
            valid,
            Some(status),
            if valid {
                "连接验证通过"
            } else {
                "接口已响应，但没有完整文本结果"
            }
            .into(),
        ))
    };
    let result = tokio::time::timeout(Duration::from_secs(60), inner).await;
    let (ok, status, text) = match result {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => (false, None, e.to_string()),
        Err(_) => (false, None, "连接测试超时".into()),
    };
    ProbeReceipt {
        ok,
        status,
        message: text,
        elapsed_ms: start.elapsed().as_millis(),
    }
}
