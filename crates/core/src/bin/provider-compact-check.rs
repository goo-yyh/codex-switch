//! Opt-in, direct native Responses/compaction checks; never starts Codex or reads its state.
use codex_switch_core::{
    gateway::{bounded_json, http_client},
    protocol::SseDecoder,
    providers::{presets, Connection},
};
use futures_util::StreamExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::Write,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const INSTRUCTIONS: &str = "This is a synthetic project ledger. Preserve the exact latest values of every durable checkpoint across turns. Archived routine observations are disposable. Never invent missing checkpoint values.";
const CAP: usize = 2048;
const SUMMARY_PROMPT: &str = include_str!("../../tests/fixtures/codex-compact/prompt.md");
const SUMMARY_PREFIX: &str = include_str!("../../tests/fixtures/codex-compact/summary_prefix.md");

fn text_output(v: &Value) -> String {
    v["output"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|i| i["type"] == "message")
        .flat_map(|i| i["content"].as_array().into_iter().flatten())
        .filter_map(|p| p["text"].as_str())
        .collect::<Vec<_>>()
        .join("")
}
fn recall(v: &Value, expected: &Value) -> Value {
    let text = text_output(v);
    let trimmed = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let parsed = serde_json::from_str::<Value>(trimmed).unwrap_or(Value::Null);
    let checks: serde_json::Map<String, Value> = expected
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, e)| (k.clone(), json!(parsed.get(k) == Some(e))))
        .collect();
    let passed = checks.values().filter(|v| **v == true).count();
    let null_fields: Vec<_> = expected
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| parsed.get(*k).is_some_and(Value::is_null))
        .cloned()
        .collect();
    let missing_fields: Vec<_> = expected
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| parsed.get(*k).is_none())
        .cloned()
        .collect();
    json!({"passed":passed,"total":checks.len(),"fields":checks,"nullFields":null_fields,"missingFields":missing_fields,"validJson":parsed.is_object(),"complete":v["status"]=="completed"})
}
fn history() -> (Vec<Value>, Value, Value) {
    let names = [
        "launch_code",
        "rollback_code",
        "owner_code",
        "region_code",
        "audit_code",
        "release_code",
    ];
    let expected: serde_json::Map<String, Value> = names
        .iter()
        .map(|n| {
            (
                n.to_string(),
                json!(format!(
                    "C{}",
                    &uuid::Uuid::new_v4().simple().to_string()[..12]
                )),
            )
        })
        .collect();
    let mut input = vec![
        json!({"role":"user","content":"Track six durable project checkpoints: launch_code, rollback_code, owner_code, region_code, audit_code, release_code. Retain their exact latest values. Routine archive observations are not checkpoints."}),
    ];
    for part in 0..48 {
        input.push(json!({"role":"user","content":format!("Continue reviewing archive batch {part}. Preserve durable checkpoints.")}));
        let mut content = String::new();
        for line in 0..18 {
            content.push_str(&format!("Archive observation {part:02}-{line:02}: synthetic routine review completed; staging checks passed, historical note closed, no durable checkpoint changed.\n"));
        }
        for (i, name) in names.iter().enumerate() {
            if part == [0, 8, 19, 25, 37, 47][i] {
                content.push_str(&format!("\nDURABLE CHECKPOINT: {name} = {}. Keep this exact value for later project continuation.\n", expected[*name].as_str().unwrap()));
            }
        }
        if part == 1 {
            content.push_str("DURABLE CHECKPOINT: release_code = OBSOLETE-DO-NOT-USE. A later release update supersedes this value.\n");
        }
        input.push(json!({"role":"assistant","content":content}));
    }
    let query = json!({"role":"user","content":"Return only a JSON object with the latest exact checkpoint values for launch_code, rollback_code, owner_code, region_code, audit_code, release_code. No markdown. If unavailable use null; do not guess."});
    (input, Value::Object(expected), query)
}

struct Run {
    file: std::fs::File,
    secrets: Vec<String>,
    failures: usize,
}
impl Run {
    async fn summary_smoke(&mut self, c: &Connection, key: &str) {
        let (mut input, _, _) = history();
        let history_bytes = serde_json::to_vec(&input).unwrap().len();
        self.emit(
            c,
            "summary_smoke_fixture",
            json!({
                "historyBytes": history_bytes, "historyItems": input.len(),
                "promptSha256": format!("{:x}", Sha256::digest(SUMMARY_PROMPT.as_bytes())),
                "remoteCompaction": false, "maxRequests": 1,
            }),
        );
        input.push(json!({"role":"user","content":SUMMARY_PROMPT}));
        let mut request = body(c, json!(input));
        request["stream"] = json!(true);
        let Some(summary) = self
            .request(c, key, "summary_generate", false, request)
            .await
        else {
            return;
        };
        let text = text_output(&summary);
        let shorter = text.len() < history_bytes;
        let ok = summary["_valid"] == true
            && !text.trim().is_empty()
            && summary["_stream_check"]["deltaMatchesFinal"] == true
            && shorter;
        self.assert(
            c,
            "summary_smoke_result",
            ok,
            json!({
                "completed": summary["status"] == "completed",
                "nonempty": !text.trim().is_empty(), "summaryBytes": text.len(),
                "historyBytes": history_bytes, "shorterThanHistory": shorter,
                "summarySha256": format!("{:x}", Sha256::digest(text.as_bytes())),
                "usedRemoteEndpoint": false, "usedCompactionTrigger": false,
            }),
        );
    }
    async fn summary_chain(&mut self, c: &Connection, key: &str) {
        let (input, expected, query) = history();
        let bytes = serde_json::to_vec(&input).unwrap();
        self.emit(c,"summary_fixture",json!({"historyBytes":bytes.len(),"historyItems":input.len(),"historySha256":format!("{:x}",Sha256::digest(&bytes)),"checkpoints":6,"promptSource":"openai/codex rust-v0.149.0","promptSha256":format!("{:x}",Sha256::digest(SUMMARY_PROMPT.as_bytes())),"remoteCompaction":false,"maxRequests":3,"stream":true}));
        let mut baseline_input = input.clone();
        baseline_input.push(query.clone());
        let mut b = body(c, json!(baseline_input));
        b["stream"] = json!(true);
        let Some(baseline) = self.request(c, key, "summary_baseline", false, b).await else {
            return;
        };
        let base_check = recall(&baseline, &expected);
        let base_ok = baseline["_valid"] == true
            && base_check["complete"] == true
            && base_check["passed"] == base_check["total"];
        self.assert(c, "summary_baseline_recall", base_ok, base_check);
        if baseline["_valid"] != true {
            return;
        }
        let mut summary_input = input;
        summary_input.push(json!({"role":"user","content":SUMMARY_PROMPT}));
        let mut b = body(c, json!(summary_input));
        b["stream"] = json!(true);
        let Some(summary) = self.request(c, key, "summary_generate", false, b).await else {
            return;
        };
        let summary_text = text_output(&summary);
        let summary_ok = summary["_valid"] == true
            && !summary_text.trim().is_empty()
            && summary["_stream_check"]["deltaMatchesFinal"] == true;
        self.assert(c,"summary_text_valid",summary_ok,json!({"nonempty":!summary_text.trim().is_empty(),"completed":summary["status"]=="completed","textBytes":summary_text.len(),"textSha256":format!("{:x}",Sha256::digest(summary_text.as_bytes()))}));
        if !summary_ok {
            return;
        }
        // Stricter than Codex's retained-user-history policy: only the summary and
        // the new question survive. Never pass baseline output or original facts.
        let next =
            json!([{"role":"user","content":format!("{SUMMARY_PREFIX}\n{summary_text}")},query]);
        let mut b = body(c, next);
        b["stream"] = json!(true);
        let Some(resumed) = self.request(c, key, "summary_resume", false, b).await else {
            return;
        };
        let check = recall(&resumed, &expected);
        let before = baseline["usage"]["input_tokens"].as_u64();
        let after = resumed["usage"]["input_tokens"].as_u64();
        let reduction = before
            .zip(after)
            .filter(|(b, _)| *b > 0)
            .map(|(b, a)| 1.0 - a as f64 / b as f64);
        let ok = base_ok
            && resumed["_valid"] == true
            && check["passed"] == check["total"]
            && reduction.is_some_and(|r| r > 0.0);
        self.assert(c,"summary_chain_result",ok,json!({"baselinePassed":base_ok,"recall":check,"inputTokensBefore":before,"inputTokensAfter":after,"inputTokenReduction":reduction,"replayedOriginalHistory":false,"usedPreviousResponseId":false,"usedRemoteEndpoint":false,"usedCompactionTrigger":false,"clientHistoryReplacementSimulated":true}));
    }
    fn emit(&mut self, c: &Connection, kind: &str, details: Value) {
        let row = json!({"timestamp":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),"provider":c.preset_id,"model":c.model,"endpoint":c.endpoint,"protocol":"responses","test":kind,"details":details});
        let mut line = serde_json::to_string(&row).unwrap();
        for key in &self.secrets {
            if !key.is_empty() {
                line = line.replace(key, "[REDACTED]");
            }
        }
        writeln!(self.file, "{line}").unwrap();
        self.file.flush().unwrap();
        println!("{line}");
    }
    async fn request(
        &mut self,
        c: &Connection,
        key: &str,
        kind: &str,
        compact: bool,
        body: Value,
    ) -> Option<Value> {
        let start = Instant::now();
        let path = if compact {
            "responses/compact"
        } else {
            "responses"
        };
        let url = c.upstream_url(path).unwrap();
        eprintln!("Starting {} / {}", c.preset_id, kind);
        let result = tokio::time::timeout(Duration::from_secs(if compact { 180 } else { 120 }), async {
            let response = http_client().map_err(|_| "client_error")?.post(&url).bearer_auth(key).json(&body).send().await.map_err(|_| "network_error")?;
            let status = response.status().as_u16();
            let is_stream = response.headers().get("content-type").and_then(|h| h.to_str().ok()).is_some_and(|h| h.contains("text/event-stream"));
            if body["stream"] == true && (200..300).contains(&status) && is_stream {
                let mut stream = response.bytes_stream(); let mut decoder = SseDecoder::default();
                let mut delta = String::new();
                let mut count = 0;
                while let Some(chunk) = stream.next().await {
                    for frame in decoder.push(&chunk.map_err(|_| "stream_read_failed")?).map_err(|_| "sse_invalid")? {
                        if frame == "[DONE]" { continue; }
                        let event: Value = serde_json::from_str(&frame).map_err(|_| "event_invalid")?;
                        count += 1;
                        if event["type"] == "response.output_text.delta" { delta.push_str(event["delta"].as_str().unwrap_or("")); }
                        if ["response.completed", "response.incomplete", "response.failed"].iter().any(|t| event["type"] == *t) {
                            let mut v = event["response"].clone();
                            v["_stream_check"] = json!({"events":count,"deltaMatchesFinal":!delta.is_empty() && delta==text_output(&v)});
                            return Ok((status, v));
                        }
                    }
                }
                Err("missing_final_event")
            } else {
                let v = bounded_json(response).await.ok().filter(Value::is_object).unwrap_or_else(|| json!({"error":{"code":"non_json_response","message":"Response was not a bounded JSON object"}}));
                Ok((status, v))
            }
        }).await;
        match result {
            Ok(Ok((status, v))) => {
                let output = v["output"].as_array();
                let encrypted = output
                    .into_iter()
                    .flatten()
                    .filter(|i| {
                        i["type"] == "compaction"
                            && i["encrypted_content"]
                                .as_str()
                                .is_some_and(|s| !s.is_empty())
                    })
                    .count();
                let valid = (200..300).contains(&status)
                    && if compact {
                        v["object"] == "response.compaction" && encrypted > 0
                    } else {
                        v["status"] == "completed" && output.is_some_and(|a| !a.is_empty())
                    };
                let error_message = v
                    .pointer("/error/message")
                    .or_else(|| v.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let mut safe_message = error_message.to_string();
                for secret in &self.secrets {
                    safe_message = safe_message.replace(secret, "[REDACTED]");
                }
                let raw_output = serde_json::to_vec(&v["output"]).unwrap();
                self.emit(c,kind,json!({"ok":valid,"httpStatus":status,"path":path,"elapsedMs":start.elapsed().as_millis(),"requestBytes":serde_json::to_vec(&body).unwrap().len(),"maxOutputTokens":body.get("max_output_tokens"),"usage":v["usage"],"object":v["object"],"responseStatus":v["status"],"incompleteDetails":v["incomplete_details"],"errorCode":v.pointer("/error/code"),"errorMessage":safe_message.chars().take(500).collect::<String>(),"outputTypes":output.into_iter().flatten().map(|i| i["type"].clone()).collect::<Vec<_>>(),"encryptedCompactionItems":encrypted,"outputBytes":raw_output.len(),"outputSha256":format!("{:x}",Sha256::digest(&raw_output)),"streamCheck":v["_stream_check"]}));
                if !valid {
                    self.failures += 1;
                }
                // Preserve error status for stop decisions, without emitting the raw response.
                let mut v = v;
                v["_http_status"] = json!(status);
                v["_valid"] = json!(valid);
                Some(v)
            }
            other => {
                self.failures += 1;
                self.emit(c,kind,json!({"ok":false,"elapsedMs":start.elapsed().as_millis(),"error":match other {Err(_)=>"timeout",Ok(Err(e))=>e,_=>unreachable!()}}));
                None
            }
        }
    }
    fn assert(&mut self, c: &Connection, kind: &str, ok: bool, detail: Value) {
        if !ok {
            self.failures += 1;
        }
        self.emit(c, kind, json!({"ok":ok,"check":detail}));
    }
}
fn body(c: &Connection, input: Value) -> Value {
    json!({"model":c.model,"input":input,"instructions":INSTRUCTIONS,"max_output_tokens":CAP,"store":false})
}

#[tokio::main]
async fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).filter(|a| a != "--").collect();
    let tools_only = args.iter().any(|a| a == "--tools-auto");
    let compact_control = args.iter().any(|a| a == "--compact-control");
    let summary_only = args.iter().any(|a| a == "--summary");
    let summary_smoke = args.iter().any(|a| a == "--summary-smoke");
    args.retain(|a| {
        ![
            "--tools-auto",
            "--compact-control",
            "--summary",
            "--summary-smoke",
        ]
        .contains(&a.as_str())
    });
    let registry = presets();
    if [tools_only, compact_control, summary_only, summary_smoke]
        .into_iter()
        .filter(|v| *v)
        .count()
        > 1
        || args.is_empty()
        || args
            .iter()
            .any(|a| a != "--all" && !registry.iter().any(|p| p.id == *a))
    {
        eprintln!("Usage: provider-compact-check [--summary | --summary-smoke | --tools-auto | --compact-control] --all | qianwen minimax zhipu kimi deepseek");
        std::process::exit(2);
    }
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let values: HashMap<String, String> =
        match dotenvy::from_path_iter(root.join(".env")).and_then(|v| v.collect()) {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Cannot parse .env; no requests sent");
                std::process::exit(2)
            }
        };
    let dir = root.join("reports/local");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!(
        "provider-compact-{}.jsonl",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    ));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    eprintln!("Sanitized receipts: {}", path.display());
    let mut run = Run {
        file,
        secrets: values.values().filter(|s| !s.is_empty()).cloned().collect(),
        failures: 0,
    };
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
            options: p.options,
        };
        let Some(key) = values.get(&p.env_key).filter(|s| !s.is_empty()) else {
            run.assert(&c, "credentials", false, json!("missing_key"));
            continue;
        };
        if summary_smoke {
            run.summary_smoke(&c, key).await;
            continue;
        }
        if summary_only {
            run.summary_chain(&c, key).await;
            continue;
        }
        if compact_control {
            let code = format!("C{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
            let expected = json!({"probe_code":code});
            let user = json!({"role":"user","content":format!("The durable checkpoint probe_code is {code}. Remember it. Return only JSON with probe_code.")});
            let mut b = body(&c, json!([user]));
            b["reasoning"] = json!({"effort":"high"});
            run.emit(&c,"control_parameters",json!({"reasoningEffort":"high","history":"user message plus actual provider output","maxRequests":3}));
            let Some(seed) = run.request(&c, key, "control_seed", false, b).await else {
                continue;
            };
            let r = recall(&seed, &expected);
            let ok = r["complete"] == true && r["passed"] == r["total"];
            run.assert(&c, "control_seed_recall", ok, r);
            if !ok {
                continue;
            }
            let mut input = vec![user];
            input.extend(seed["output"].as_array().unwrap().clone());
            let Some(compact) = run
                .request(
                    &c,
                    key,
                    "control_compact",
                    true,
                    json!({"model":c.model,"input":input,"instructions":INSTRUCTIONS}),
                )
                .await
            else {
                continue;
            };
            if compact["_valid"] != true {
                continue;
            }
            let mut next = compact["output"].as_array().unwrap().clone();
            next.push(json!({"role":"user","content":"Return only JSON containing the exact probe_code checkpoint. If unavailable use null."}));
            let mut b = body(&c, json!(next));
            b["reasoning"] = json!({"effort":"high"});
            if let Some(v) = run.request(&c, key, "control_resume", false, b).await {
                let r = recall(&v, &expected);
                run.assert(
                    &c,
                    "control_resume_recall",
                    r["complete"] == true && r["passed"] == r["total"],
                    r,
                );
            }
            continue;
        }
        let basic = if tools_only {
            None
        } else {
            run.request(
                &c,
                key,
                "text",
                false,
                body(&c, json!("Reply with exactly OK.")),
            )
            .await
        };
        let basic_ok = tools_only
            || basic
                .as_ref()
                .is_some_and(|v| v["_valid"] == true && text_output(v).trim() == "OK");
        if !tools_only {
            run.assert(&c, "text_assertion", basic_ok, json!({"exactOK":basic_ok}));
        }
        // Authentication, permission, quota, invalid model/parameters stop this provider.
        if basic.as_ref().is_some_and(|v| {
            matches!(
                v["_http_status"].as_u64(),
                Some(400 | 401 | 402 | 403 | 429)
            )
        }) {
            continue;
        }
        if basic_ok {
            if !tools_only {
                let mut b = body(&c, json!("Reply with exactly OK."));
                b["stream"] = json!(true);
                if let Some(v) = run.request(&c, key, "stream", false, b).await {
                    let ok = v["_valid"] == true
                        && text_output(&v).trim() == "OK"
                        && v["_stream_check"]["deltaMatchesFinal"] == true;
                    run.assert(&c, "stream_assertion", ok, v["_stream_check"].clone());
                }
            }
            let user = json!({"role":"user","content":"Call report_probe with value OK. This synthetic tool has no external side effects."});
            let tools = json!([{"type":"function","name":"report_probe","description":"Return a synthetic marker.","parameters":{"type":"object","properties":{"value":{"type":"string","enum":["OK"]}},"required":["value"],"additionalProperties":false}}]);
            let mut b = body(&c, json!([user]));
            b["tools"] = tools.clone();
            b["tool_choice"] = json!(if tools_only { "auto" } else { "required" });
            run.emit(&c, "tool_mode", json!({"toolChoice":b["tool_choice"]}));
            if let Some(v) = run.request(&c, key, "tool_call", false, b).await {
                let calls: Vec<_> = v["output"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|i| i["type"] == "function_call")
                    .collect();
                let valid = v["_valid"] == true
                    && calls.len() == 1
                    && calls[0]["name"] == "report_probe"
                    && calls[0]["call_id"].as_str().is_some_and(|s| !s.is_empty())
                    && serde_json::from_str::<Value>(calls[0]["arguments"].as_str().unwrap_or(""))
                        .ok()
                        == Some(json!({"value":"OK"}));
                run.assert(
                    &c,
                    "tool_assertion",
                    valid,
                    json!({"exactNameAndArguments":valid,"callCount":calls.len()}),
                );
                if valid {
                    let mut input = vec![user];
                    input.extend(v["output"].as_array().unwrap().clone());
                    input.push(json!({"type":"function_call_output","call_id":calls[0]["call_id"],"output":"OK"}));
                    input.push(json!({"role":"user","content":"Reply with exactly the marker returned by the tool. Do not call tools."}));
                    let mut b = body(&c, json!(input));
                    b["tools"] = tools;
                    b["tool_choice"] = json!(if tools_only { "auto" } else { "none" });
                    run.emit(
                        &c,
                        "tool_result_mode",
                        json!({"toolChoice":b["tool_choice"]}),
                    );
                    if let Some(v) = run.request(&c, key, "tool_result", false, b).await {
                        let ok = v["_valid"] == true && text_output(&v).trim() == "OK";
                        run.assert(&c, "tool_result_assertion", ok, json!({"exactOK":ok}));
                    }
                }
            }
        }
        if tools_only {
            continue;
        }
        let small = json!([{"role":"user","content":"Remember the durable checkpoint probe_code exactly."},{"role":"assistant","content":"DURABLE CHECKPOINT: probe_code = PROBE-7K4M9. Keep this exact value for continuation."}]);
        let Some(compact) = run
            .request(
                &c,
                key,
                "compact_small",
                true,
                json!({"model":c.model,"input":small,"instructions":INSTRUCTIONS}),
            )
            .await
        else {
            continue;
        };
        if compact["_valid"] != true || !basic_ok {
            continue;
        }
        let mut next = compact["output"].as_array().unwrap().clone();
        next.push(json!({"role":"user","content":"Return only JSON containing the exact probe_code checkpoint. If unavailable use null."}));
        if let Some(v) = run
            .request(
                &c,
                key,
                "compact_small_resume",
                false,
                body(&c, json!(next)),
            )
            .await
        {
            let r = recall(&v, &json!({"probe_code":"PROBE-7K4M9"}));
            run.assert(
                &c,
                "small_recall",
                r["passed"] == r["total"] && r["complete"] == true,
                r,
            );
        }
        let (input, expected, query) = history();
        let bytes = serde_json::to_vec(&input).unwrap();
        run.emit(&c,"fixture",json!({"historyBytes":bytes.len(),"historyItems":input.len(),"sha256":format!("{:x}",Sha256::digest(&bytes)),"checkpoints":6,"obsoleteValueSuperseded":true,"factsOnlyInAssistantHistory":true}));
        let mut baseline_input = input.clone();
        baseline_input.push(query.clone());
        let baseline = run
            .request(
                &c,
                key,
                "long_baseline",
                false,
                body(&c, json!(baseline_input)),
            )
            .await;
        if let Some(v) = &baseline {
            let r = recall(v, &expected);
            run.assert(
                &c,
                "baseline_recall",
                r["passed"] == r["total"] && r["complete"] == true,
                r,
            );
        }
        let Some(compact) = run
            .request(
                &c,
                key,
                "compact_long",
                true,
                json!({"model":c.model,"input":input,"instructions":INSTRUCTIONS}),
            )
            .await
        else {
            continue;
        };
        if compact["_valid"] != true {
            continue;
        }
        let mut next = compact["output"].as_array().unwrap().clone();
        next.push(query);
        if let Some(v) = run
            .request(&c, key, "compact_long_resume", false, body(&c, json!(next)))
            .await
        {
            let r = recall(&v, &expected);
            let before = baseline
                .as_ref()
                .and_then(|v| v["usage"]["input_tokens"].as_u64());
            let after = v["usage"]["input_tokens"].as_u64();
            let reduction = before
                .zip(after)
                .filter(|(b, _)| *b > 0)
                .map(|(b, a)| 1.0 - a as f64 / b as f64);
            let base_ok = baseline.as_ref().is_some_and(|v| {
                let r = recall(v, &expected);
                r["complete"] == true && r["passed"] == r["total"]
            });
            run.assert(&c,"long_compaction_result",base_ok && r["complete"]==true && r["passed"]==r["total"] && reduction.is_some_and(|r|r>0.0),json!({"baselinePassed":base_ok,"recall":r,"inputTokensBefore":before,"inputTokensAfter":after,"inputTokenReduction":reduction,"replayedOriginalHistory":false,"usedPreviousResponseId":false,"returnedWindowPassedUnmodified":true}));
        }
    }
    eprintln!(
        "Finished with {} failed checks. Receipts: {}",
        run.failures,
        path.display()
    );
    if run.failures > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recall_requires_exact_values_and_a_complete_response() {
        let expected = json!({"code":"C123"});
        let response = |text: &str, status: &str| json!({"status":status,"output":[{"type":"message","content":[{"type":"output_text","text":text}]}]});
        assert_eq!(
            recall(&response("{\"code\":\"C123\"}", "completed"), &expected)["passed"],
            1
        );
        let missing = recall(&response("{\"code\":null}", "completed"), &expected);
        assert_eq!(missing["passed"], 0);
        assert_eq!(missing["nullFields"], json!(["code"]));
        assert_eq!(
            recall(&response("{\"code\":\"C123\"}", "incomplete"), &expected)["complete"],
            false
        );
        assert_eq!(
            recall(&response("C123", "completed"), &expected)["validJson"],
            false
        );
    }

    #[test]
    fn fixture_is_long_and_answers_do_not_leak_into_query_or_user_messages() {
        let (input, expected, query) = history();
        assert!(serde_json::to_vec(&input).unwrap().len() > 128_000);
        let user_text = input
            .iter()
            .filter(|v| v["role"] == "user")
            .map(Value::to_string)
            .collect::<String>();
        let assistant_text = input
            .iter()
            .filter(|v| v["role"] == "assistant")
            .map(Value::to_string)
            .collect::<String>();
        for value in expected.as_object().unwrap().values() {
            let value = value.as_str().unwrap();
            assert!(!user_text.contains(value));
            assert!(!query.to_string().contains(value));
            assert_eq!(assistant_text.matches(value).count(), 1);
        }
        assert!(
            assistant_text.find("OBSOLETE-DO-NOT-USE").unwrap()
                < assistant_text
                    .find(expected["release_code"].as_str().unwrap())
                    .unwrap()
        );
        assert_ne!(history().1, expected);
    }
}
