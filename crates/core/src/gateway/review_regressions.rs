use super::*;
use std::time::Instant;
#[tokio::test]
async fn review_stream_tool_replay_restores_reasoning_without_previous_id() {
    let tools = json!([{"type":"function","name":"read_file","parameters":{"type":"object","properties":{}}}]);
    let (_, map) =
        protocol::chat_request(&json!({"model":"mock","input":"read","tools":tools})).unwrap();
    let chunk = json!({"choices":[{"delta":{"reasoning_content":"synthetic reasoning","tool_calls":[{"index":0,"id":"call_mock","type":"function","function":{"name":"read_file","arguments":"{}"}}]},"finish_reason":"tool_calls"}]});
    let incoming = futures_util::stream::iter(vec![Ok::<_, std::io::Error>(bytes::Bytes::from(
        format!("data: {chunk}\n\ndata: [DONE]\n\n"),
    ))]);
    let mut stream = Box::pin(protocol::chat_stream(incoming, map));
    let mut decoder = SseDecoder::default();
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
    let done = events
        .iter()
        .find(|v| v["type"] == "response.output_item.done" && v["item"]["type"] == "function_call")
        .unwrap();
    let response = &events
        .iter()
        .find(|v| v["type"] == "response.completed")
        .unwrap()["response"];
    let state = GatewayState::new(vec![], "synthetic-local-token".into()).unwrap();
    state
        .remember(
            "route_mock",
            &json!([{"role":"user","content":"read"}]),
            response,
        )
        .await;
    let mut item = done["item"].clone();
    item.as_object_mut().unwrap().remove("reasoning_content");
    let mut replay = json!({"model":"mock","tools":tools,"input":[{"role":"user","content":"read"},item,{"type":"function_call_output","call_id":"call_mock","output":"synthetic result"}]});
    state.enrich("route_mock", &mut replay).await.unwrap();
    let (out, _) = protocol::chat_request(&replay).unwrap();
    assert_eq!(
        out["messages"][1]["reasoning_content"], "synthetic reasoning",
        "cached provider metadata must survive output_item.done replay"
    );
}
fn call_item(id: &str, reasoning: &str) -> Value {
    json!({"type":"function_call","call_id":id,"name":"read_file","arguments":"{}","reasoning_content":reasoning})
}
fn result_item(id: &str) -> Value {
    json!({"type":"function_call_output","call_id":id,"output":"synthetic"})
}
#[tokio::test]
async fn call_id_fallback_restores_groups_and_rejects_other_routes_expiry_and_collisions() {
    let s = GatewayState::new(vec![], "synthetic".into()).unwrap();
    let response = json!({"id":"r1","status":"completed","output":[call_item("a", "real metadata"),call_item("b", "real metadata")]});
    s.remember("route-a", &json!([]), &response).await;
    let mut body = json!({"input":[result_item("a"),result_item("b")]});
    assert!(s.enrich("route-b", &mut body.clone()).await.is_err());
    s.enrich("route-a", &mut body).await.unwrap();
    let (chat, _) = protocol::chat_request(&body).unwrap();
    assert_eq!(
        chat["messages"][0]["tool_calls"].as_array().unwrap().len(),
        2
    );
    assert_eq!(chat["messages"][0]["reasoning_content"], "real metadata");
    s.remember(
        "route-a",
        &json!([]),
        &json!({"id":"r2","status":"completed","output":[call_item("a", "conflicting metadata")]}),
    )
    .await;
    assert!(s
        .enrich("route-a", &mut json!({"input":[result_item("a")]}))
        .await
        .is_err());
    for (_, h) in s.history.write().await.iter_mut() {
        h.created = Instant::now() - Duration::from_secs(3601);
    }
    assert!(s
        .enrich("route-a", &mut json!({"input":[result_item("b")]}))
        .await
        .is_err());
}
#[tokio::test]
async fn full_replay_with_previous_id_does_not_duplicate_calls() {
    let s = GatewayState::new(vec![], "synthetic".into()).unwrap();
    let user = json!({"role":"user","content":"read"});
    let mut call = call_item("a", "real metadata");
    call["id"] = json!("item_a");
    call["status"] = json!("completed");
    s.remember(
        "route",
        &json!([user]),
        &json!({"id":"r1","status":"completed","output":[call]}),
    )
    .await;
    let mut typed = call.clone();
    for key in ["reasoning_content", "id", "status"] {
        typed.as_object_mut().unwrap().remove(key);
    }
    let mut body = json!({"previous_response_id":"r1","input":[user,typed,result_item("a")]});
    s.enrich("route", &mut body).await.unwrap();
    assert_eq!(body["input"].as_array().unwrap().len(), 3);
    assert!(protocol::chat_request(&body).is_ok());
    let mut altered = call.clone();
    altered["arguments"] = json!("{\"other\":true}");
    assert!(s
        .enrich("route", &mut json!({"input":[altered,result_item("a")]}))
        .await
        .is_err());
}
