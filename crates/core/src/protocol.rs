//! Local resource/identity guards around the pinned CC Switch implementation.
use crate::{message, Result};
use futures_util::{Stream, StreamExt};
use serde_json::{json, Value};
#[derive(Clone, Default)]
pub struct ToolMap {
    context: cc_switch_codex::ToolContext,
    names: std::collections::HashSet<String>,
}
pub fn chat_request(body: &Value) -> Result<(Value, ToolMap)> {
    chat_request_for(body, None, "")
}
pub fn chat_request_for(
    body: &Value,
    reasoning: Option<&cc_switch_codex::ReasoningConfig>,
    endpoint: &str,
) -> Result<(Value, ToolMap)> {
    if body["input"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["type"] == "compaction_trigger" || item["type"] == "compaction")
    }) {
        return Err(message("此 Chat 连接不支持 Codex 远程压缩协议，请关闭远程上下文压缩，或使用支持该协议的 Responses 服务。"));
    }
    if body
        .get("parallel_tool_calls")
        .is_some_and(|v| !v.is_boolean())
    {
        return Err(message("parallel_tool_calls 必须是布尔值。"));
    }
    // Check before conversion: an upstream bridge may reorder tool messages.
    let mut pending = std::collections::HashSet::new();
    let mut seen = std::collections::HashSet::new();
    for item in body["input"].as_array().into_iter().flatten() {
        match item["type"].as_str().unwrap_or("message") {
            "function_call" | "custom_tool_call" | "tool_search_call" => {
                let id = item["call_id"]
                    .as_str()
                    .or_else(|| item["id"].as_str())
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| message("工具调用缺少标识。"))?;
                if !seen.insert(id) {
                    return Err(message("工具调用标识重复。"));
                }
                pending.insert(id);
            }
            "function_call_output" | "custom_tool_call_output" | "tool_search_output" => {
                let id = item["call_id"].as_str().unwrap_or_default();
                if !pending.remove(id) {
                    return Err(message("工具上下文缺失或结果重复，请重新开始此任务。"));
                }
            }
            "message" if !pending.is_empty() => {
                return Err(message("上一轮工具结果尚未完整返回。"))
            }
            _ => {}
        }
    }
    if !pending.is_empty() {
        return Err(message("工具结果缺失，请等待工具完成。"));
    }
    let (mut out, context) = cc_switch_codex::chat_request(body.clone(), reasoning, endpoint)
        .map_err(|e| message(e.to_string()))?;
    let names = out["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t["function"]["name"].as_str().map(str::to_owned))
        .collect();
    // Preserve the explicit caller policy; omitted parallel policy stays omitted.
    if out["tools"].as_array().is_some_and(|a| !a.is_empty()) {
        if let Some(v) = body.get("parallel_tool_calls") {
            out["parallel_tool_calls"] = v.clone();
        } else {
            out.as_object_mut().unwrap().remove("parallel_tool_calls");
        }
    } else {
        out.as_object_mut().unwrap().remove("parallel_tool_calls");
    }
    Ok((out, ToolMap { context, names }))
}
pub fn chat_response(body: &Value, model: &str, map: ToolMap) -> Result<Value> {
    if body.get("error").is_some() {
        return Err(message("上游返回错误。"));
    }
    if body["choices"][0]["finish_reason"] != "length" {
        for t in body["choices"][0]["message"]["tool_calls"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if !map
                .names
                .contains(t["function"]["name"].as_str().unwrap_or_default())
            {
                return Err(message("上游调用了未声明的工具。"));
            }
            serde_json::from_str::<Value>(t["function"]["arguments"].as_str().unwrap_or(""))
                .map_err(|_| message("工具参数不是完整 JSON。"))?;
        }
    }
    let mut out = cc_switch_codex::chat_response(body.clone(), &map.context)
        .map_err(|e| message(e.to_string()))?;
    out["model"] = json!(model);
    if out["status"] == "incomplete" {
        if let Some(items) = out["output"].as_array_mut() {
            items.retain(|v| !is_tool(v));
        }
    }
    if out["status"] == "completed" && out["output"].as_array().is_none_or(|a| a.is_empty()) {
        return Err(message("上游没有返回文本或工具调用。"));
    }
    Ok(out)
}
fn is_tool(v: &Value) -> bool {
    matches!(
        v["type"].as_str(),
        Some("function_call" | "custom_tool_call" | "tool_search_call")
    )
}
// Validate framing before the upstream parser (which tolerates malformed frames).
// Cancellation and idle timeout are handled by the owning gateway.
pub fn chat_stream<E: std::error::Error + Send + 'static>(
    incoming: impl Stream<Item = std::result::Result<bytes::Bytes, E>> + Send + 'static,
    map: ToolMap,
) -> impl Stream<Item = std::result::Result<bytes::Bytes, std::io::Error>> + Send {
    let truncated = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let input_truncated = truncated.clone();
    let checked = async_stream::try_stream! {
        let mut incoming=Box::pin(incoming); let mut decoder=SseDecoder::default(); let mut finished=false; let mut size=0usize;
        while let Some(chunk)=incoming.next().await {
            let chunk=chunk.map_err(|_|std::io::Error::other("上游流读取失败。"))?;
            size+=chunk.len(); if size>16*1024*1024 {Err(std::io::Error::other("上游输出超过大小限制。"))?;}
            let frames=decoder.push(&chunk).map_err(|e|std::io::Error::other(e.to_string()))?;
            for frame in frames {
                if frame=="[DONE]" { if !finished {Err(std::io::Error::other("上游流缺少结束标志。"))?;} yield bytes::Bytes::from("data: [DONE]\n\n"); return; }
                let v:Value=serde_json::from_str(&frame).map_err(|_|std::io::Error::other("无效流式 JSON。"))?;
                if v.get("error").is_some(){Err(std::io::Error::other("上游返回流式错误。"))?;}
                if let Some(reason)=v["choices"][0]["finish_reason"].as_str(){
                    if !["stop","tool_calls","length"].contains(&reason){Err(std::io::Error::other("上游未正常完成输出。"))?;}
                    finished=true;
                    input_truncated.store(reason=="length",std::sync::atomic::Ordering::SeqCst);
                }
                yield bytes::Bytes::from(format!("data: {frame}\n\n"));
            }
        }
        if !decoder.complete() || !finished {Err(std::io::Error::other("上游流被截断。"))?;}
    };
    async_stream::stream! {
        let mut converted=Box::pin(cc_switch_codex::chat_stream::<std::io::Error>(checked,map.context.clone()));
        let mut decoder=SseDecoder::default();
        while let Some(chunk)=converted.next().await {
            let frames=match chunk.and_then(|b|decoder.push(&b).map_err(|e|std::io::Error::other(e.to_string()))) {
                Ok(f)=>f,Err(_)=>{yield Ok(bytes::Bytes::from(failure("上游流格式无效。")));return;}
            };
            for frame in frames {
                let mut v:Value=match serde_json::from_str(&frame){Ok(v)=>v,Err(_)=>{yield Ok(bytes::Bytes::from(failure("响应转换失败。")));return;}};
                let kind=v["type"].as_str().unwrap_or("").to_owned();
                let cut=truncated.load(std::sync::atomic::Ordering::SeqCst);
                if cut && ((kind=="response.output_item.done" && is_tool(&v["item"])) || kind=="response.function_call_arguments.done" || kind.starts_with("response.custom_tool_call_input.")) {continue;}
                if matches!(kind.as_str(),"response.output_item.added"|"response.output_item.done") && is_tool(&v["item"]) {
                    let item=&v["item"];
                    if !map.context.contains_response_tool(item) || (kind=="response.output_item.done" && item["type"]=="function_call" && serde_json::from_str::<Value>(item["arguments"].as_str().unwrap_or("")).is_err()) {
                        yield Ok(bytes::Bytes::from(failure("上游工具名称或参数无效。")));return;
                    }
                }
                if kind=="response.function_call_arguments.done" && serde_json::from_str::<Value>(v["arguments"].as_str().unwrap_or("")).is_err() {yield Ok(bytes::Bytes::from(failure("工具参数不是完整 JSON。")));return;}
                if cut && kind=="response.completed" {if let Some(items)=v["response"]["output"].as_array_mut(){items.retain(|v|!is_tool(v));}}
                yield Ok(bytes::Bytes::from(event(&kind,v)));
            }
        }
    }
}
pub fn event(kind: &str, mut data: Value) -> String {
    data["type"] = json!(kind);
    format!("event: {kind}\ndata: {}\n\n", data)
}
pub fn failure(text: &str) -> String {
    event(
        "response.failed",
        json!({"response":{"id":"resp_error","object":"response","status":"failed","error":{"code":"upstream_error","message":text}}}),
    )
}

#[derive(Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
}
impl SseDecoder {
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<String>> {
        self.buffer.extend_from_slice(chunk);
        if self.buffer.len() > 1024 * 1024 {
            return Err(message("流式事件超过大小限制。"));
        }
        let mut frames = vec![];
        loop {
            let lf = self
                .buffer
                .windows(2)
                .position(|x| x == b"\n\n")
                .map(|i| (i, 2));
            let cr = self
                .buffer
                .windows(4)
                .position(|x| x == b"\r\n\r\n")
                .map(|i| (i, 4));
            let Some((i, n)) = lf.into_iter().chain(cr).min_by_key(|x| x.0) else {
                break;
            };
            let bytes: Vec<u8> = self.buffer.drain(..i + n).collect();
            let frame = std::str::from_utf8(&bytes).map_err(|_| message("流包含无效 UTF-8。"))?;
            let data = frame
                .lines()
                .filter_map(|l| l.strip_prefix("data:").map(str::trim_start))
                .collect::<Vec<_>>()
                .join("\n");
            if !data.is_empty() {
                frames.push(data);
            }
        }
        Ok(frames)
    }
    pub fn complete(&self) -> bool {
        self.buffer.iter().all(u8::is_ascii_whitespace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf8_and_crlf_frames() {
        let s = "data: {\"text\":\"中文\"}\r\n\r\ndata: [DONE]\n\n";
        let mut d = SseDecoder::default();
        let mut out = vec![];
        for b in s.as_bytes() {
            out.extend(d.push(&[*b]).unwrap());
        }
        assert_eq!(out.len(), 2);
        assert!(out[0].contains("中文"));
        assert!(d.complete());
    }
    #[test]
    fn tool_result_requires_context() {
        assert!(chat_request(
            &json!({"input":[{"type":"function_call_output","call_id":"lost","output":"x"}]})
        )
        .is_err());
    }
    #[test]
    fn text_request_and_response() {
        let (r, m) =
            chat_request(&json!({"model":"m","instructions":"hello","input":"hi"})).unwrap();
        assert_eq!(r["messages"][1]["content"], "hi");
        let v = chat_response(
            &json!({"choices":[{"message":{"content":"你好"},"finish_reason":"stop"}]}),
            "alias",
            m,
        )
        .unwrap();
        assert_eq!(v["output"][0]["content"][0]["text"], "你好");
    }
    #[test]
    fn custom_tool_roundtrip() {
        let (_, map) =
            chat_request(&json!({"tools":[{"type":"custom","name":"apply_patch"}],"input":"go"}))
                .unwrap();
        let r=chat_response(&json!({"choices":[{"message":{"tool_calls":[{"id":"c","function":{"name":"apply_patch","arguments":"{\"input\":\"patch\"}"}}]},"finish_reason":"tool_calls"}]}),"m",map).unwrap();
        assert_eq!(r["output"][0]["type"], "custom_tool_call");
        assert_eq!(r["output"][0]["input"], "patch");
    }
    #[test]
    fn token_limit_is_incomplete() {
        let r = chat_response(
            &json!({"choices":[{"message":{"content":"partial"},"finish_reason":"length"}]}),
            "m",
            ToolMap::default(),
        )
        .unwrap();
        assert_eq!(r["status"], "incomplete");
    }
    #[test]
    fn tools_cannot_be_executed_from_truncated_arguments() {
        let (_, map) = chat_request(&json!({"tools":[{"type":"function","name":"one"}]})).unwrap();
        let v=chat_response(&json!({"choices":[{"message":{"tool_calls":[{"id":"a","function":{"name":"one","arguments":"{"}}]},"finish_reason":"length"}]}),"m",map).unwrap();
        assert_eq!(v["status"], "incomplete");
        assert!(v["output"].as_array().unwrap().is_empty());
    }
    #[test]
    fn tool_results_require_preceding_unique_calls() {
        let call = json!({"type":"function_call","call_id":"c","name":"one","arguments":"{}"});
        let result = json!({"type":"function_call_output","call_id":"c","output":"OK"});
        assert!(chat_request(&json!({"input":[result,call]})).is_err());
        assert!(chat_request(&json!({"input":[call,result,result]})).is_err());
        assert!(chat_request(&json!({"input":[call,result]})).is_ok());
    }
    #[test]
    fn explicit_tool_choice_is_respected() {
        let tools = json!([{"type":"function","name":"one"}]);
        let (r, _) = chat_request(&json!({"tools":tools,"tool_choice":"required"})).unwrap();
        assert_eq!(r["tool_choice"], "required");
        let (r, _) =
            chat_request(&json!({"tools":tools,"tool_choice":{"type":"function","name":"one"}}))
                .unwrap();
        assert_eq!(r["tool_choice"]["function"]["name"], "one");
    }
}

#[cfg(test)]
mod review_regressions {
    use super::*;
    #[test]
    fn review_keeps_explicit_parallel_tool_policy() {
        let (out, _) = chat_request(&json!({"model":"mock","input":"hi","parallel_tool_calls":false,"tools":[{"type":"function","name":"read_file","parameters":{"type":"object","properties":{}}}]})).unwrap();
        assert_eq!(
            out.get("parallel_tool_calls"),
            Some(&json!(false)),
            "explicit false must survive conversion"
        );
    }
    #[test]
    fn review_long_namespace_gets_bounded_reversible_name() {
        let result = chat_request(
            &json!({"model":"mock","input":"hi","tools":[{"type":"namespace","name":"a".repeat(40),"tools":[{"type":"function","name":"b".repeat(40),"parameters":{"type":"object","properties":{}}}]}]}),
        );
        assert!(
            result.is_ok(),
            "individually valid names must be encoded instead of rejecting the request"
        );
    }
    #[test]
    fn bounded_names_roundtrip_choices_history_and_common_prefixes() {
        let ns = "namespace".repeat(6);
        let a = "tool".repeat(12);
        let b = format!("{a}b");
        let tools = json!([{"type":"namespace","name":ns,"tools":[{"type":"function","name":a},{"type":"function","name":b}]}]);
        let request = json!({"tools":tools,"tool_choice":{"type":"function","namespace":ns,"name":a},"input":[{"type":"function_call","namespace":ns,"name":a,"call_id":"a","arguments":"{}"},{"type":"function_call_output","call_id":"a","output":"OK"}]});
        let (chat, map) = chat_request(&request).unwrap();
        let name = chat["tools"][0]["function"]["name"].as_str().unwrap();
        let other = chat["tools"][1]["function"]["name"].as_str().unwrap();
        assert!(name.len() <= 64 && other.len() <= 64);
        assert_ne!(name, other);
        assert_eq!(chat["tool_choice"]["function"]["name"], name);
        assert_eq!(
            chat["messages"][0]["tool_calls"][0]["function"]["name"],
            name
        );
        let response = chat_response(&json!({"choices":[{"message":{"tool_calls":[{"id":"new","function":{"name":name,"arguments":"{}"}}]},"finish_reason":"tool_calls"}]}), "mock", map).unwrap();
        assert_eq!(response["output"][0]["name"], a);
        assert_eq!(response["output"][0]["namespace"], ns);
    }
    #[test]
    fn tool_policy_is_omitted_without_tools_and_sampling_values_survive() {
        let (out, _) =
            chat_request(&json!({"parallel_tool_calls":false,"temperature":0,"top_p":0.7}))
                .unwrap();
        assert!(out.get("parallel_tool_calls").is_none());
        assert_eq!(out["temperature"], 0);
        assert_eq!(out["top_p"], 0.7);
        let tools = json!([{"type":"function","name":"read"}]);
        let (out, _) = chat_request(&json!({"tools":tools})).unwrap();
        assert!(out.get("parallel_tool_calls").is_none());
        assert!(chat_request(&json!({"tools":tools,"parallel_tool_calls":"false"})).is_err());
    }
}
