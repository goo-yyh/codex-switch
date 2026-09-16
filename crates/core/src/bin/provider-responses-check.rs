//! Explicit, low-output Responses checks against catalog models. Never invoked by the app or CI.
use codex_switch_core::{
    gateway::{bounded_json, http_client},
    providers::presets,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn clean(value: &Value, secrets: &[String]) -> Value {
    if value.is_null() {
        return Value::Null;
    }
    let mut text = value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string());
    for secret in secrets {
        if !secret.is_empty() {
            text = text.replace(secret, "[redacted]");
        }
    }
    Value::String(text.chars().take(500).collect())
}
#[tokio::main]
async fn main() {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let mut max_output_tokens = 256u64;
    if args.len() >= 2 && args[args.len() - 2] == "--max-output-tokens" {
        max_output_tokens = args
            .pop()
            .and_then(|v| v.parse().ok())
            .filter(|v| (1..=4096).contains(v))
            .unwrap_or_else(|| {
                eprintln!("Token limit must be between 1 and 4096; no requests sent.");
                std::process::exit(2);
            });
        args.pop();
    }
    let selected_model = if args.len() == 2 && args[0] == "--model" {
        Some(args[1].clone())
    } else {
        None
    };
    if args != ["--all"] && selected_model.is_none() {
        eprintln!("Usage: cargo run -p codex-switch-core --bin provider-responses-check -- --all | --model MODEL");
        std::process::exit(2);
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    let values: HashMap<String, String> =
        match dotenvy::from_path_iter(path).and_then(|v| v.collect()) {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Cannot parse .env; no requests sent.");
                std::process::exit(2);
            }
        };
    let secrets: Vec<String> = values.values().filter(|v| !v.is_empty()).cloned().collect();
    let catalog: HashMap<String, Vec<String>> = serde_json::from_str(include_str!(
        "../../../../packages/provider-registry/models.json"
    ))
    .expect("catalog");
    if selected_model
        .as_ref()
        .is_some_and(|model| !catalog.values().any(|models| models.contains(model)))
    {
        eprintln!("Unknown catalog model; no requests sent.");
        std::process::exit(2);
    }
    let client = http_client().expect("HTTP client");
    let mut tasks = tokio::task::JoinSet::new();
    for preset in presets() {
        let models: Vec<String> = catalog
            .get(&preset.id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|model| {
                selected_model
                    .as_ref()
                    .is_none_or(|selected| selected == model)
            })
            .collect();
        if models.is_empty() {
            continue;
        }
        let key = values
            .get(&preset.env_key)
            .filter(|v| !v.trim().is_empty())
            .cloned();
        let client = client.clone();
        let secrets = secrets.clone();
        tasks.spawn(async move {
            let endpoint = format!("{}/responses", preset.endpoint.trim_end_matches('/'));
            let mut blocked = if key.is_none() { Some("missing_key") } else { None };
            let mut all_ok = true;
            for model in models {
                let started = Instant::now();
                let mut row = json!({"provider":preset.id,"model":model,"endpoint":endpoint,"protocol":"responses","startedAt":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),"maxOutputTokens":max_output_tokens});
                if let Some(reason) = blocked { row["result"]=json!("skipped"); row["reason"]=json!(reason); }
                else {
                    let request = async {
                        let response = client.post(&endpoint).bearer_auth(key.as_ref().unwrap().trim())
                            .json(&json!({"model":model,"input":"Reply with exactly OK.","stream":false,"max_output_tokens":max_output_tokens}))
                            .send().await.map_err(|_| "network_error")?;
                        let status = response.status().as_u16();
                        let body = bounded_json(response).await;
                        Ok::<_, &str>((status, body))
                    };
                    match tokio::time::timeout(Duration::from_secs(60), request).await {
                        Err(_) => {row["result"]=json!("timeout");}
                        Ok(Err(reason)) => { row["result"]=json!(reason); }
                        Ok(Ok((status, body))) => {
                            row["httpStatus"]=json!(status);
                            if status == 401 { blocked=Some("provider_authentication_failed"); }
                            match body {
                                Err(_) => { row["result"]=json!("invalid_json"); }
                                Ok(body) => {
                                    let text: String = body["output"].as_array().into_iter().flatten()
                                        .filter(|v| v["type"]=="message")
                                        .flat_map(|v| v["content"].as_array().into_iter().flatten())
                                        .filter_map(|v| v["text"].as_str()).collect();
                                    row["responseStatus"]=clean(&body["status"], &secrets);
                                    row["returnedModel"]=clean(&body["model"], &secrets);
                                    row["errorCode"]=clean(body["error"].get("code").unwrap_or(&body["code"]), &secrets);
                                    row["errorMessage"]=clean(body["error"].get("message").unwrap_or(&body["message"]), &secrets);
                                    row["incompleteReason"]=clean(&body["incomplete_details"]["reason"], &secrets);
                                    row["inputTokens"]=body["usage"]["input_tokens"].clone();
                                    row["outputTokens"]=body["usage"]["output_tokens"].clone();
                                    row["textNonempty"]=json!(!text.trim().is_empty());
                                    row["exactMatch"]=json!(text.trim()=="OK");
                                    if text.trim() != "OK" && !text.is_empty() {
                                        row["textSample"] = clean(&json!(text), &secrets);
                                    }
                                    row["result"]=json!(if !(200..300).contains(&status) {"http_error"}
                                        else if body["status"]=="completed" && text.trim()=="OK" {"passed"}
                                        else if body["status"]=="completed" && !text.trim().is_empty() {"completed_text_mismatch"}
                                        else if body["status"]=="incomplete" {"incomplete"} else {"unexpected_response"});
                                }
                            }
                        }
                    }
                }
                row["elapsedMs"]=json!(started.elapsed().as_millis());
                all_ok &= row["result"] == "passed";
                println!("{row}");
                if blocked.is_none() {tokio::time::sleep(Duration::from_millis(500)).await;}
            }
            all_ok
        });
    }
    let mut all_ok = true;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(passed) => all_ok &= passed,
            Err(_) => {
                eprintln!("A provider task failed; report is incomplete.");
                all_ok = false;
            }
        }
    }
    if !all_ok {
        std::process::exit(1);
    }
}
