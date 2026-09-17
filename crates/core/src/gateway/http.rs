//! Shared limits for gateway traffic and explicit provider probes.
use crate::{message, Result};
use futures_util::StreamExt;
use serde_json::Value;
use std::time::Duration;

// Never follow redirects with provider credentials. Bound bytes before JSON decoding.
pub fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|_| message("无法创建网络连接。"))
}
pub async fn bounded_json(response: reqwest::Response) -> Result<Value> {
    let mut stream = response.bytes_stream();
    let mut bytes = vec![];
    while let Some(chunk) = stream.next().await {
        let c = chunk.map_err(|_| message("上游响应读取失败。"))?;
        if bytes.len() + c.len() > 16 * 1024 * 1024 {
            return Err(message("上游响应过大。"));
        }
        bytes.extend_from_slice(&c);
    }
    serde_json::from_slice(&bytes).map_err(|_| message("上游返回了无效 JSON。"))
}
