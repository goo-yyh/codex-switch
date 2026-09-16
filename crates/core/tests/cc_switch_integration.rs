use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use codex_switch_core::{
    config::{project_with_options, ConfigManager},
    gateway::{router, GatewayState, Route},
    protocol,
    providers::{catalog, normalize_address, presets, Connection, Protocol},
};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
struct Mock {
    endpoint: String,
    seen: Arc<Mutex<Vec<Value>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn mock(status: StatusCode, body: Value, sse: Option<&'static str>) -> Mock {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let seen = Arc::new(Mutex::new(vec![]));
    let capture = seen.clone();
    let app=Router::new().fallback(post(move |uri:Uri,headers:HeaderMap,Json(v):Json<Value>|{
        let capture=capture.clone();let body=body.clone();async move{
            capture.lock().unwrap().push(json!({"uri":uri.to_string(),"authorization":headers["authorization"].to_str().unwrap(),"body":v}));
            if let Some(sse)=sse{(status,[("content-type","text/event-stream")],sse).into_response()}else{(status,Json(body)).into_response()}
        }
    }));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Mock {
        endpoint,
        seen,
        task,
    }
}
fn connection(endpoint: &str, protocol: Protocol) -> Connection {
    serde_json::from_value(json!({"id":uuid::Uuid::new_v4().to_string(),"name":"test","presetId":"custom","endpoint":endpoint,"model":"upstream-model","protocol":protocol,"contextWindow":32768})).unwrap()
}
fn route(mock: &Mock, protocol: Protocol, key: &str) -> Route {
    Route {
        connection: connection(&mock.endpoint, protocol),
        key: key.into(),
        aliases: vec![],
    }
}
async fn send(state: GatewayState, alias: &str, path: &str, stream: bool) -> Response {
    router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("authorization", "Bearer local")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"model":alias,"input":"synthetic only","stream":stream}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
}
async fn json_body(response: Response) -> Value {
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 1000000)
            .await
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn remote_v2_preserves_trigger_on_normal_responses_and_opaque_output() {
    let payload = json!({"object":"response","status":"completed","output":[{"type":"compaction","encrypted_content":"synthetic-opaque"}]});
    let mock = mock(StatusCode::OK, payload.clone(), None).await;
    let r = route(&mock, Protocol::Responses, "key-native");
    let alias = r.connection.alias();
    let state = GatewayState::new(vec![r], "local".into()).unwrap();
    let input = json!([{"role":"user","content":"synthetic history"},{"type":"compaction","encrypted_content":"previous-opaque"},{"type":"compaction_trigger"}]);
    let response = router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer local")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"model":alias,"input":input,"store":false}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(json_body(response).await, payload);
    let seen = mock.seen.lock().unwrap();
    assert_eq!(seen[0]["uri"], "/responses");
    assert_eq!(seen[0]["body"]["input"], input);
    assert_eq!(seen[0]["body"]["store"], false);
}

#[test]
fn local_summary_stays_a_normal_chat_request_but_remote_items_cannot_be_silently_dropped() {
    let (chat,_)=protocol::chat_request(&json!({"model":"test","input":[{"role":"user","content":"Summarize this synthetic history."}],"metadata":{"request_kind":"compaction"}})).unwrap();
    assert_eq!(
        chat["messages"][0]["content"],
        "Summarize this synthetic history."
    );
    for kind in ["compaction_trigger", "compaction"] {
        let err = protocol::chat_request(
            &json!({"input":[{"type":kind,"encrypted_content":"synthetic"}]}),
        )
        .err()
        .unwrap();
        assert!(err.to_string().contains("关闭远程上下文压缩"));
    }
}

#[tokio::test]
async fn remote_v2_sse_compaction_item_is_passed_through_without_rewriting() {
    let sse="data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"compaction\",\"encrypted_content\":\"opaque-test\"}}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[{\"type\":\"compaction\",\"encrypted_content\":\"opaque-test\"}]}}\n\n";
    let mock = mock(StatusCode::OK, Value::Null, Some(sse)).await;
    let r = route(&mock, Protocol::Responses, "native-key");
    let alias = r.connection.alias();
    let state = GatewayState::new(vec![r], "local".into()).unwrap();
    let response = router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer local")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"model":alias,"input":[{"type":"compaction_trigger"}],"stream":true})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        axum::body::to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .as_ref(),
        sse.as_bytes()
    );
    let seen = mock.seen.lock().unwrap();
    assert_eq!(seen[0]["uri"], "/responses");
    assert_eq!(seen[0]["body"]["input"][0]["type"], "compaction_trigger");
}

#[tokio::test]
async fn native_compact_is_exact_passthrough_at_full_url_with_query() {
    let payload = json!({"id":"cmp_mock","object":"response.compaction","output":[{"type":"compaction","encrypted_content":"synthetic-opaque"}],"usage":{"input_tokens":100,"output_tokens":10}});
    let mock = mock(StatusCode::OK, payload.clone(), None).await;
    let mut r = route(&mock, Protocol::Responses, "key-native");
    r.connection
        .endpoint
        .push_str("/custom/v2/responses?api-version=demo");
    r.connection.options.full_url = true;
    let alias = r.connection.alias();
    let s = GatewayState::new(vec![r], "local".into()).unwrap();
    let response = send(s.clone(), &alias, "/v1/responses/compact", false).await;
    assert_eq!(response.status(), 200);
    assert_eq!(json_body(response).await, payload);
    let seen = mock.seen.lock().unwrap();
    assert_eq!(
        seen[0]["uri"],
        "/custom/v2/responses/compact?api-version=demo"
    );
    assert_eq!(seen[0]["authorization"], "Bearer key-native");
    assert_eq!(seen[0]["body"]["model"], "upstream-model");
    assert_eq!(s.active.load(std::sync::atomic::Ordering::SeqCst), 0);
}
#[tokio::test]
async fn explicit_queue_uses_each_routes_protocol_model_and_credential() {
    let first = mock(
        StatusCode::METHOD_NOT_ALLOWED,
        json!({"error":"unsupported"}),
        None,
    )
    .await;
    let next = mock(
        StatusCode::OK,
        json!({"id":"chat_mock","choices":[{"message":{"content":"OK"},"finish_reason":"stop"}]}),
        None,
    )
    .await;
    let a = route(&first, Protocol::Responses, "key-A");
    let alias = a.connection.alias();
    let mut b = route(&next, Protocol::Chat, "key-B");
    b.connection.model = "different-model".into();
    let state = GatewayState::new(vec![a.clone()], "local".into()).unwrap();
    state.set_failover(vec![a, b]).await;
    let response = send(state, &alias, "/v1/responses", false).await;
    assert_eq!(response.status(), 200);
    let out = json_body(response).await;
    assert_eq!(out["output"][0]["content"][0]["text"], "OK");
    let first = first.seen.lock().unwrap();
    let next = next.seen.lock().unwrap();
    assert_eq!(first[0]["uri"], "/responses");
    assert_eq!(first[0]["authorization"], "Bearer key-A");
    assert_eq!(next[0]["uri"], "/chat/completions");
    assert_eq!(next[0]["authorization"], "Bearer key-B");
    assert_eq!(next[0]["body"]["model"], "different-model");
    assert!(next[0]["body"]["messages"].is_array());
}
#[tokio::test]
async fn failures_do_not_guess_a_protocol_without_an_explicit_queue() {
    let mock = mock(StatusCode::NOT_FOUND, json!({"error":"not_found"}), None).await;
    let r = route(&mock, Protocol::Responses, "key-A");
    let alias = r.connection.alias();
    let state = GatewayState::new(vec![r], "local".into()).unwrap();
    assert_eq!(
        send(state, &alias, "/v1/responses", false).await.status(),
        404
    );
    let seen = mock.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0]["uri"], "/responses");
}
#[tokio::test]
async fn committed_stream_failure_does_not_replay_against_next_provider() {
    let first=mock(StatusCode::OK,Value::Null,Some("event: response.failed\ndata: {\"type\":\"response.failed\",\"response\":{\"status\":\"failed\"}}\n\n")).await;
    let next = mock(StatusCode::OK, json!({}), None).await;
    let a = route(&first, Protocol::Responses, "A");
    let alias = a.connection.alias();
    let state = GatewayState::new(vec![a.clone()], "local".into()).unwrap();
    state
        .set_failover(vec![a, route(&next, Protocol::Responses, "B")])
        .await;
    let response = send(state.clone(), &alias, "/v1/responses", true).await;
    assert_eq!(response.status(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), 100000)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("response.failed"));
    assert!(next.seen.lock().unwrap().is_empty());
    assert_eq!(state.active.load(std::sync::atomic::Ordering::SeqCst), 0);
}
#[test]
fn urls_preserve_custom_prefix_and_full_query_without_double_paths() {
    let mut c = connection(
        "https://relay.example/prefix/v2/responses",
        Protocol::Responses,
    );
    c.validate().unwrap();
    assert_eq!(
        c.upstream_url("responses").unwrap(),
        "https://relay.example/prefix/v2/responses"
    );
    c.endpoint = "https://relay.example/custom?version=demo".into();
    c.options.full_url = true;
    c.validate().unwrap();
    assert_eq!(c.upstream_url("responses").unwrap(), c.endpoint);
    assert!(c.upstream_url("responses/compact").is_err());
    assert!(normalize_address("https://relay.example/prefix?version=demo", false).is_err());
    assert!(normalize_address("https://user:secret@relay.example/v1", true).is_err());
}
#[test]
fn presets_are_native_but_old_records_keep_chat_and_custom_addresses() {
    assert!(presets().iter().all(|p| p.protocol == Protocol::Responses));
    let old = connection("https://custom.example/path", Protocol::Chat);
    let round: Connection = serde_json::from_value(serde_json::to_value(&old).unwrap()).unwrap();
    assert_eq!(round.protocol, Protocol::Chat);
    assert_eq!(round.endpoint, old.endpoint);
    assert!(!round.options.full_url);
}
#[test]
fn upstream_catalog_profiles_and_model_overrides_reach_codex() {
    for preset in presets() {
        let mut c = connection(&preset.endpoint, preset.protocol);
        c.model = preset.model;
        c.options = preset.options;
        c.context_window = preset.context_window;
        let rows = catalog(&[c.clone()]);
        let row = &rows["models"][0];
        assert_eq!(row["slug"], c.alias());
        assert_eq!(row["context_window"], c.context_window);
        assert!(row["base_instructions"].is_string());
        if c.endpoint == "https://api.deepseek.com" {
            assert_eq!(row["apply_patch_tool_type"], "freeform");
        } else {
            assert!(row.get("apply_patch_tool_type").is_none());
            assert_eq!(row["shell_type"], "shell_command");
        }
    }
    let mut c = connection("https://relay.example", Protocol::Responses);
    c.options.model_overrides.insert(c.model.clone(),serde_json::from_value(json!({"contextWindow":65536,"reasoningLevels":["high","low"],"defaultReasoningLevel":"low","inputModalities":["text"],"parallelToolCalls":true})).unwrap());
    c.validate().unwrap();
    let rows = catalog(&[c]);
    let row = &rows["models"][0];
    assert_eq!(row["context_window"], 65536);
    assert_eq!(row["default_reasoning_level"], "low");
    assert_eq!(row["input_modalities"], json!(["text"]));
    assert_eq!(row["supports_parallel_tool_calls"], true);
}
#[test]
fn remote_compaction_is_explicit_and_rollback_restores_original_config() {
    let t = tempfile::tempdir().unwrap();
    let m = ConfigManager::new(t.path().join("codex"), t.path().join("state"));
    std::fs::create_dir_all(m.path().parent().unwrap()).unwrap();
    let original = "model = \"original\"\n";
    std::fs::write(m.path(), original).unwrap();
    let catalog = t.path().join("codex-switch-catalog-test.json");
    m.enable_with_options(
        "http://127.0.0.1:123/v1",
        "local",
        "alias",
        &catalog,
        true,
        || Ok(()),
    )
    .unwrap();
    let active = std::fs::read_to_string(m.path()).unwrap();
    assert!(active.contains("name = \"OpenAI\""));
    m.disable(false).unwrap();
    assert_eq!(std::fs::read_to_string(m.path()).unwrap(), original);
    let disabled = project_with_options(
        original,
        "http://127.0.0.1:123/v1",
        "local",
        "alias",
        &catalog,
        false,
    )
    .unwrap();
    assert!(disabled.contains("name = \"Codex Switch\""));
}
async fn stream_frames(chunks: Vec<&str>, request: Value) -> Vec<Value> {
    let (_, map) = protocol::chat_request(&request).unwrap();
    let incoming = futures_util::stream::iter(
        chunks
            .into_iter()
            .map(|s| Ok::<_, std::io::Error>(bytes::Bytes::from(s.to_owned())))
            .collect::<Vec<_>>(),
    );
    let mut stream = Box::pin(protocol::chat_stream(incoming, map));
    let mut decoder = protocol::SseDecoder::default();
    let mut events = vec![];
    while let Some(chunk) = stream.next().await {
        events.extend(
            decoder
                .push(&chunk.unwrap())
                .unwrap()
                .iter()
                .map(|s| serde_json::from_str::<Value>(s).unwrap()),
        );
    }
    events
}
#[tokio::test]
async fn malformed_and_truncated_streams_never_complete_successfully() {
    for chunk in [
        "data: {bad json}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
        "data: {\"error\":{\"message\":\"test\"}}\n\n",
    ] {
        let events = stream_frames(vec![chunk], json!({"input":"test"})).await;
        assert!(events.iter().any(|e| e["type"] == "response.failed"));
        assert!(!events.iter().any(|e| e["type"] == "response.completed"));
    }
}
#[tokio::test]
async fn truncated_tool_arguments_are_never_published_as_executable_calls() {
    let events=stream_frames(vec!["data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"c\",\"function\":{\"name\":\"one\",\"arguments\":\"{\"}}]},\"finish_reason\":\"length\"}]}\n\ndata: [DONE]\n\n"],json!({"tools":[{"type":"function","name":"one"}]})).await;
    assert!(!events
        .iter()
        .any(|e| e["type"] == "response.output_item.done" && e["item"]["type"] == "function_call"));
    let done = events
        .iter()
        .find(|e| e["type"] == "response.completed")
        .unwrap();
    assert_eq!(done["response"]["status"], "incomplete");
    assert_eq!(done["response"]["output"], json!([]));
}
#[tokio::test]
async fn streaming_preserves_early_text_and_usage() {
    let events=stream_frames(vec!["data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n","data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":2,\"total_tokens\":12}}\n\ndata: [DONE]\n\n"],json!({"input":"hi"})).await;
    let text = events
        .iter()
        .position(|v| v["type"] == "response.output_text.delta")
        .unwrap();
    let done = events
        .iter()
        .position(|v| v["type"] == "response.completed")
        .unwrap();
    assert!(text < done);
    assert_eq!(events[done]["response"]["usage"]["total_tokens"], 12);
}
#[tokio::test]
async fn stream_rejects_unknown_tools() {
    let events=stream_frames(vec!["data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"c\",\"function\":{\"name\":\"unknown\",\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\ndata: [DONE]\n\n"],json!({"input":"test"})).await;
    assert!(events.iter().any(|e| e["type"] == "response.failed"));
    assert!(!events.iter().any(|e| e["type"] == "response.completed"));
}

#[tokio::test]
async fn empty_stream_can_fail_over_before_committing_a_response() {
    let first = mock(StatusCode::OK, Value::Null, Some("")).await;
    let next=mock(StatusCode::OK,Value::Null,Some("event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\",\"status\":\"completed\",\"output\":[]}}\n\n")).await;
    let a = route(&first, Protocol::Responses, "A");
    let alias = a.connection.alias();
    let state = GatewayState::new(vec![a.clone()], "local".into()).unwrap();
    state
        .set_failover(vec![a, route(&next, Protocol::Responses, "B")])
        .await;
    let response = send(state, &alias, "/v1/responses", true).await;
    assert_eq!(response.status(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), 100000)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("response.completed"));
    assert_eq!(next.seen.lock().unwrap().len(), 1);
}
#[test]
fn sqlite_roundtrip_preserves_user_endpoint_and_capabilities_in_runtime_routes() {
    use codex_switch_core::{profiles::Profile, store::Store};
    let t = tempfile::tempdir().unwrap();
    let store = Store::open(&t.path().join("state.sqlite")).unwrap();
    let profile:Profile=serde_json::from_value(json!({"id":"profile","name":"User relay","presetId":"kimi","endpoint":"https://relay.example/custom/responses?api-version=demo","protocol":"responses","models":["private-model"],"contextWindow":32768,"credentialId":"credential","routeIds":[uuid::Uuid::new_v4().to_string()],"options":{"fullUrl":true,"endpointCandidates":["https://relay.example/custom/responses?api-version=demo"],"modelOverrides":{"private-model":{"contextWindow":65536,"reasoningLevels":["high"],"defaultReasoningLevel":"high","inputModalities":["text"]}}}})).unwrap();
    store.save_profile(&profile).unwrap();
    drop(store);
    let store = Store::open(&t.path().join("state.sqlite")).unwrap();
    let routes = store.routes_for_profiles(&[profile.id]).unwrap();
    assert_eq!(routes[0].endpoint, profile.endpoint);
    assert_eq!(routes[0].options, profile.options);
    assert_eq!(catalog(&routes)["models"][0]["context_window"], 65536);
    assert_eq!(
        routes[0].upstream_url("responses").unwrap(),
        profile.endpoint
    );
}
