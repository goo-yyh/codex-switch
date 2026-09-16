//! Explicit provider-only checks. No Codex process, config directory or credential vault access.
use codex_switch_core::{
    gateway::{bounded_json, http_client, probe, status_message},
    protocol::{chat_request_for, chat_response, chat_stream, SseDecoder, ToolMap},
    providers::{presets, Connection, Protocol},
};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn receipt(
    c: &Connection,
    kind: &str,
    ok: bool,
    status: Option<u16>,
    message: &str,
    start: Instant,
) {
    println!(
        "{}",
        json!({"provider":c.preset_id,"model":c.model,"protocol":c.protocol,"timestamp":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),"test":kind,"ok":ok,"status":status,"message":message,"elapsedMs":start.elapsed().as_millis()})
    );
}
async fn request(
    c: &Connection,
    key: &str,
    body: Value,
) -> Result<(Value, u16), (&'static str, Option<u16>)> {
    let (upstream, map) = if c.protocol == Protocol::Chat {
        chat_request_for(&body, c.options.chat_reasoning.as_ref(), &c.endpoint)
            .map_err(|_| ("request_conversion_failed", None))?
    } else {
        (body.clone(), ToolMap::default())
    };
    let client = http_client().map_err(|_| ("client_failed", None))?;
    let response = client
        .post(
            c.upstream_url(if c.protocol == Protocol::Chat {
                "chat/completions"
            } else {
                "responses"
            })
            .map_err(|_| ("invalid_endpoint", None))?,
        )
        .bearer_auth(key)
        .json(&upstream)
        .send()
        .await
        .map_err(|_| ("network_error", None))?;
    let status = response.status().as_u16();
    if !response.status().is_success() {
        return Err((status_message(status), Some(status)));
    }
    if body["stream"] != true {
        let raw = bounded_json(response)
            .await
            .map_err(|_| ("invalid_json_response", Some(status)))?;
        let converted = if c.protocol == Protocol::Chat {
            chat_response(&raw, &c.alias(), map)
                .map_err(|_| ("response_conversion_failed", Some(status)))?
        } else {
            raw
        };
        return Ok((converted, status));
    }
    if !response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.contains("text/event-stream"))
    {
        return Err(("expected_event_stream", Some(status)));
    }
    let mut stream: std::pin::Pin<
        Box<dyn futures_util::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send>,
    > = if c.protocol == Protocol::Chat {
        Box::pin(chat_stream(response.bytes_stream(), map))
    } else {
        Box::pin(
            response
                .bytes_stream()
                .map(|v| v.map_err(|_| std::io::Error::other("read"))),
        )
    };
    let mut decoder = SseDecoder::default();
    while let Some(chunk) = stream.next().await {
        for frame in decoder
            .push(&chunk.map_err(|_| ("stream_interrupted", Some(status)))?)
            .map_err(|_| ("invalid_sse", Some(status)))?
        {
            if frame == "[DONE]" {
                continue;
            }
            let v: Value =
                serde_json::from_str(&frame).map_err(|_| ("invalid_event", Some(status)))?;
            if v["type"] == "response.completed" {
                return Ok((v["response"].clone(), status));
            }
            if v["type"] == "response.failed" {
                return Err(("upstream_stream_failed", Some(status)));
            }
        }
    }
    Err(("missing_final_response", Some(status)))
}
async fn checked(
    c: &Connection,
    key: &str,
    kind: &str,
    body: Value,
    want_tool: bool,
) -> Option<Value> {
    let start = Instant::now();
    match tokio::time::timeout(Duration::from_secs(60), request(c, key, body)).await {
        Ok(Ok((v, status))) => {
            let complete = v["status"] == "completed";
            let output = v["output"].as_array();
            let valid = complete
                && output.is_some_and(|a| {
                    if want_tool {
                        a.iter()
                            .any(|i| i["type"] == "function_call" && i["name"] == "report_probe")
                    } else {
                        a.iter().any(|i| {
                            i["type"] == "message"
                                && i["content"][0]["text"]
                                    .as_str()
                                    .is_some_and(|s| !s.is_empty())
                        })
                    }
                });
            receipt(
                c,
                kind,
                valid,
                Some(status),
                if valid {
                    "passed"
                } else {
                    "incomplete_or_unexpected_output"
                },
                start,
            );
            valid.then_some(v)
        }
        Ok(Err((m, status))) => {
            receipt(c, kind, false, status, m, start);
            None
        }
        Err(_) => {
            receipt(c, kind, false, None, "timeout", start);
            None
        }
    }
}
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).filter(|a| a != "--").collect();
    let registry = presets();
    if args.is_empty()
        || args
            .iter()
            .any(|a| a != "--all" && !registry.iter().any(|p| p.id == *a))
    {
        eprintln!("Usage: pnpm test:providers --all | qianwen minimax zhipu kimi deepseek");
        std::process::exit(2)
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    let values: HashMap<String, String> =
        match dotenvy::from_path_iter(path).and_then(|i| i.collect()) {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Cannot parse repository .env; no requests sent.");
                std::process::exit(2)
            }
        };
    let mut all_ok = true;
    for p in registry
        .into_iter()
        .filter(|p| args.iter().any(|a| a == "--all" || *a == p.id))
    {
        let c = Connection {
            id: uuid::Uuid::new_v4().to_string(),
            name: p.name,
            preset_id: p.id,
            endpoint: p.endpoint,
            model: p.model,
            protocol: p.protocol,
            context_window: p.context_window,
            options: p.options.clone(),
        };
        let Some(key) = values.get(&p.env_key).filter(|v| !v.trim().is_empty()) else {
            receipt(
                &c,
                "credentials",
                false,
                None,
                "missing_key",
                Instant::now(),
            );
            all_ok = false;
            continue;
        };
        let start = Instant::now();
        let r = probe(&c, key).await;
        receipt(&c, "text", r.ok, r.status, &r.message, start);
        if !r.ok {
            all_ok = false;
            continue;
        }
        if checked(&c,key,"stream",json!({"model":c.model,"input":"Reply with exactly OK.","stream":true,"max_output_tokens":256}),false).await.is_none(){all_ok=false;continue}
        let user = json!({"role":"user","content":"Call report_probe with value OK. This is a synthetic protocol test; no external action is performed."});
        let tools = json!([{"type":"function","name":"report_probe","description":"Return a test marker without performing any action.","parameters":{"type":"object","properties":{"value":{"type":"string","enum":["OK"]}},"required":["value"],"additionalProperties":false}}]);
        let Some(response)=checked(&c,key,"tool_call",json!({"model":c.model,"input":[user],"tools":tools,"tool_choice":"required","max_output_tokens":256}),true).await else{all_ok=false;continue};
        let mut input = vec![user];
        let output = response["output"].as_array().unwrap();
        input.extend(output.clone());
        for item in output {
            if item["type"] == "function_call" {
                input.push(
                    json!({"type":"function_call_output","call_id":item["call_id"],"output":"OK"}),
                );
            }
        }
        input.push(json!({"role":"user","content":"The tool returned OK. Reply with exactly OK and do not call tools again."}));
        if checked(&c,key,"tool_result",json!({"model":c.model,"input":input,"tools":tools,"tool_choice":"none","max_output_tokens":256}),false).await.is_none(){all_ok=false;}
    }
    if !all_ok {
        std::process::exit(1)
    }
}
