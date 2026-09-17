//! Bounded, route-scoped replay of completed responses and tool calls.
use super::GatewayState;
use crate::{message, Result};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub(super) struct History {
    route: String,
    input: Vec<Value>,
    calls: Vec<Value>,
    pub(super) created: Instant,
}

impl GatewayState {
    pub(super) async fn remember(&self, route: &str, input: &Value, response: &Value) {
        if response["status"] != "completed" {
            return;
        }
        let mut all = input_items(input);
        all.extend(response["output"].as_array().cloned().unwrap_or_default());
        let calls: Vec<_> = response["output"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|v| is_call(v))
            .cloned()
            .collect();
        if serde_json::to_vec(&(&all, &calls))
            .map(|b| b.len() > 1024 * 1024)
            .unwrap_or(true)
        {
            return;
        }
        let Some(id) = response["id"].as_str() else {
            return;
        };
        let mut h = self.history.write().await;
        while h.len() >= 64 {
            h.pop_front();
        }
        h.push_back((
            id.into(),
            History {
                route: route.into(),
                input: all,
                calls,
                created: Instant::now(),
            },
        ));
    }
    pub(super) async fn remember_response_event(&self, alias: &str, input: &Value, frame: &str) {
        if let Ok(event) = serde_json::from_str::<Value>(frame) {
            if event["type"] == "response.completed" {
                self.remember(alias, input, &event["response"]).await;
            }
        }
    }
    pub(super) async fn enrich(&self, route: &str, body: &mut Value) -> Result<()> {
        let h = self.history.read().await;
        let valid: Vec<_> = h
            .iter()
            .filter(|(_, h)| h.route == route && h.created.elapsed() < Duration::from_secs(3600))
            .collect();
        let previous = body["previous_response_id"]
            .as_str()
            .map(|id| {
                valid
                    .iter()
                    .find(|(key, _)| key == id)
                    .map(|(_, h)| h)
                    .ok_or_else(|| message("会话缓存已失效或属于其他连接，请重新开始任务。"))
            })
            .transpose()?;
        let mut input = input_items(&body["input"]);
        // Resolve only within this client alias, including after a provider switch.
        // Repeated identical groups in
        // cached responses are harmless; conflicting call identities are not.
        let lookup = |id: &str| -> Result<Option<Vec<Value>>> {
            if let Some(prev) = previous {
                if let Some(call) = prev.input.iter().find(|v| is_call(v) && v["call_id"] == id) {
                    return Ok(Some(vec![call.clone()]));
                }
            }
            let mut found: Option<Vec<Value>> = None;
            for (_, entry) in &valid {
                if entry.calls.iter().any(|v| v["call_id"] == id) {
                    if found.as_ref().is_some_and(|old| old != &entry.calls) {
                        return Err(message("工具调用标识存在冲突，请重新开始任务。"));
                    }
                    found = Some(entry.calls.clone());
                }
            }
            Ok(found)
        };
        for item in input.iter_mut().filter(|v| is_call(v)) {
            if let Some(group) = item["call_id"].as_str().map(&lookup).transpose()?.flatten() {
                let cached = group
                    .iter()
                    .find(|v| v["call_id"] == item["call_id"])
                    .unwrap();
                for key in ["type", "name", "namespace", "arguments", "input"] {
                    if item.get(key).is_some() && item.get(key) != cached.get(key) {
                        return Err(message("工具调用与缓存不一致，请重新开始任务。"));
                    }
                }
                if let Some(reasoning) = cached.get("reasoning_content") {
                    item["reasoning_content"] = reasoning.clone();
                }
            }
        }
        if let Some(prev) = previous {
            // A client may replay the complete history even with a previous ID.
            if input.len() < prev.input.len()
                || !input
                    .iter()
                    .zip(&prev.input)
                    .all(|(a, b)| same_history_item(a, b))
            {
                let mut full = prev.input.clone();
                full.extend(input);
                input = full;
            }
            body.as_object_mut().unwrap().remove("previous_response_id");
        }
        for item in &input {
            if matches!(
                item["type"].as_str(),
                Some("function_call_output" | "custom_tool_call_output" | "tool_search_output")
            ) {
                if let Some(id) = item["call_id"].as_str() {
                    let cached = lookup(id)?;
                    if cached.is_none() && !input.iter().any(|v| is_call(v) && v["call_id"] == id) {
                        return Err(message("工具上下文缺失或已过期，请重新开始任务。"));
                    }
                }
            }
        }
        let upstream_history = cc_switch_codex::HistoryStore::default();
        for (id, entry) in &valid {
            upstream_history
                .record_response(&json!({"id":id,"output":entry.calls}))
                .await;
        }
        body["input"] = json!(input);
        upstream_history.enrich_request(body).await;
        Ok(())
    }
}
fn same_history_item(a: &Value, b: &Value) -> bool {
    if is_call(a) && is_call(b) {
        return ["type", "call_id", "name", "namespace", "arguments", "input"]
            .iter()
            .all(|key| a.get(*key) == b.get(*key));
    }
    let clean = |v: &Value| {
        let mut v = v.clone();
        if let Some(o) = v.as_object_mut() {
            o.remove("id");
            o.remove("status");
            if o.get("type").is_some_and(|v| v == "message") {
                o.remove("type");
            }
        }
        v
    };
    clean(a) == clean(b)
}
fn is_call(v: &Value) -> bool {
    matches!(
        v["type"].as_str(),
        Some("function_call" | "custom_tool_call" | "tool_search_call")
    )
}
fn input_items(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a.clone(),
        Value::Object(_) => vec![v.clone()],
        Value::String(s) => vec![json!({"role":"user","content":s})],
        _ => vec![],
    }
}
