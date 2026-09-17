use super::*;
use axum::http::Request;
use tower::ServiceExt;
#[tokio::test]
async fn models_require_token_without_opening_a_port() {
    let s = GatewayState::new(vec![], "local-token".into()).unwrap();
    let app = router(s);
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
    let r = app
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .header("authorization", "Bearer local-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
}
#[tokio::test]
async fn model_list_only_exposes_current_models_and_redirects_old_aliases() {
    let old = test_route();
    let old_alias = old.connection.alias();
    let new = test_route();
    let new_alias = new.connection.alias();
    let s = GatewayState::new(vec![old], "token".into()).unwrap();
    s.replace_routes(vec![new]).await;
    assert!(s.routes.read().await.contains_key(&old_alias));
    let response = router(s)
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .header("authorization", "Bearer token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 10000)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["data"].as_array().unwrap().len(), 1);
    assert_eq!(value["data"][0]["id"], new_alias);
}
#[tokio::test]
async fn history_is_scoped_to_route() {
    let s = GatewayState::new(vec![], "t".into()).unwrap();
    s.remember("route-a",&json!("hello"),&json!({"id":"r1","status":"completed","output":[{"type":"message","role":"assistant","content":"hi"}]})).await;
    let mut body = json!({"previous_response_id":"r1","input":"continue"});
    assert!(s.enrich("route-b", &mut body).await.is_err());
    s.enrich("route-a", &mut body).await.unwrap();
    assert_eq!(body["input"].as_array().unwrap().len(), 3);
}
#[tokio::test]
async fn unknown_alias_fails_without_network() {
    let app = router(GatewayState::new(vec![], "t".into()).unwrap());
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer t")
                .header("content-type", "application/json")
                .body(Body::from("{\"model\":\"missing\"}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
}
fn test_route() -> Route {
    Route {
        key: "synthetic".into(),
        aliases: vec![],
        connection: Connection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "test".into(),
            preset_id: "custom".into(),
            endpoint: "https://example.com".into(),
            model: "test".into(),
            protocol: Protocol::Chat,
            context_window: 32768,
            options: Default::default(),
        },
    }
}
#[tokio::test]
async fn chat_compact_uses_upstream_chat_bridge_and_releases_request_slot() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let upstream=Router::new().route("/chat/completions",post(|Json(v):Json<Value>|async move {assert!(v["messages"].is_array());Json(json!({"id":"mock","choices":[{"message":{"content":"summary"},"finish_reason":"stop"}]}))}));
    let task = tokio::spawn(async move {
        axum::serve(listener, upstream).await.unwrap();
    });
    let mut r = test_route();
    r.connection.endpoint = format!("http://127.0.0.1:{port}");
    let alias = r.connection.alias();
    let s = GatewayState::new(vec![r], "t".into()).unwrap();
    let app = router(s.clone());
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses/compact")
                .header("authorization", "Bearer t")
                .header("content-type", "application/json")
                .body(Body::from(json!({"model":alias}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    task.abort();
    assert_eq!(s.active.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn route_updates_send_old_aliases_to_current_provider_model_and_key() {
    let mut old = test_route();
    old.connection.preset_id = "zhipu".into();
    old.connection.endpoint = "https://glm.example/v4".into();
    old.connection.model = "glm-test-model".into();
    old.key = "synthetic-glm-key".into();
    let alias = old.connection.alias();
    let s = GatewayState::new(vec![old], "t".into()).unwrap();
    let mut kimi = test_route();
    kimi.connection.preset_id = "kimi".into();
    kimi.connection.endpoint = "https://kimi.example/v1".into();
    kimi.connection.model = "kimi-test-model".into();
    kimi.key = "synthetic-kimi-key".into();
    let kimi_alias = kimi.connection.alias();
    s.replace_routes(vec![kimi]).await;
    let routes = s.routes.read().await;
    assert_eq!(routes.len(), 2);
    let glm = &routes[&alias];
    assert_eq!(glm.connection.endpoint, "https://kimi.example/v1");
    assert_eq!(glm.connection.model, "kimi-test-model");
    assert_eq!(glm.key, "synthetic-kimi-key");
    let kimi = &routes[&kimi_alias];
    assert_eq!(kimi.connection.endpoint, "https://kimi.example/v1");
    assert_eq!(kimi.connection.model, "kimi-test-model");
    assert_eq!(kimi.key, "synthetic-kimi-key");
}
#[tokio::test]
async fn repeated_switches_and_restart_aliases_only_use_active_credentials() {
    let a = test_route();
    let a_alias = a.connection.alias();
    let mut b = test_route();
    b.key = "synthetic-b".into();
    let b_alias = b.connection.alias();
    let mut c = test_route();
    c.key = "synthetic-c".into();
    let c_alias = c.connection.alias();
    let s = GatewayState::new(vec![a.clone(), b.clone()], "t".into()).unwrap();
    // Already issued requests own a clone and can finish, while subsequent resolution changes.
    let issued = s.routes.read().await[&a_alias].clone();
    s.replace_routes(vec![b.clone(), c.clone()]).await;
    assert_eq!(s.routes.read().await[&a_alias].key, "synthetic-b");
    assert_eq!(s.routes.read().await[&c_alias].key, "synthetic-c");
    assert_eq!(issued.key, a.key);
    s.replace_routes(vec![c.clone()]).await;
    assert!(s.routes.read().await.values().all(|r| r.key == c.key));
    // A fresh runtime only needs old aliases, never their old Keys or endpoints.
    c.aliases = vec![a_alias.clone(), b_alias, c_alias];
    let fresh = GatewayState::new(vec![c.clone()], "t".into()).unwrap();
    assert!(fresh
        .routes
        .read()
        .await
        .values()
        .all(|r| r.key == "synthetic-c"));
    fresh.replace_routes(vec![a.clone()]).await;
    assert!(fresh.routes.read().await.values().all(|r| r.key == a.key));
    assert_eq!(fresh.visible_models.read().await.as_slice(), [a_alias]);
}
#[tokio::test]
async fn old_conversation_context_follows_switch_without_sending_old_response_id() {
    let old = test_route();
    let alias = old.connection.alias();
    let s = GatewayState::new(vec![old], "t".into()).unwrap();
    let input = json!([{ "role": "user", "content": "old question" }]);
    // Capture a native Responses completion from split SSE chunks as production does.
    let event = json!({"type":"response.completed","response":{
        "id":"old-provider-id", "status":"completed",
        "output":[{"type":"message","role":"assistant","content":"old answer"}]
    }});
    let wire = format!("event: response.completed\ndata: {event}\n\n");
    let mut decoder = SseDecoder::default();
    for chunk in wire.as_bytes().chunks(7) {
        for frame in decoder.push(chunk).unwrap() {
            s.remember_response_event(&alias, &input, &frame).await;
        }
    }
    s.replace_routes(vec![test_route()]).await;
    let mut body =
        json!({"model":alias,"previous_response_id":"old-provider-id","input":"continue"});
    s.enrich(&alias, &mut body).await.unwrap();
    assert!(body.get("previous_response_id").is_none());
    assert_eq!(body["input"].as_array().unwrap().len(), 3);
    assert_eq!(body["input"][0]["content"], "old question");
    let mut foreign = json!({"previous_response_id":"old-provider-id","input":"continue"});
    assert!(s
        .enrich("another-session-alias", &mut foreign)
        .await
        .is_err());
}
#[tokio::test]
async fn history_is_bounded_and_evicts_old_references() {
    let s = GatewayState::new(vec![], "t".into()).unwrap();
    for i in 0..65 {
        s.remember(
            "a",
            &json!("hello"),
            &json!({"id":i.to_string(),"status":"completed","output":[]}),
        )
        .await;
    }
    assert_eq!(s.history.read().await.len(), 64);
    assert!(s
        .enrich("a", &mut json!({"previous_response_id":"0"}))
        .await
        .is_err());
}
#[tokio::test]
async fn cancelled_gateway_rejects_before_any_upstream_request() {
    let r = test_route();
    let alias = r.connection.alias();
    let s = GatewayState::new(vec![r], "t".into()).unwrap();
    s.cancel.cancel();
    let r = router(s)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer t")
                .header("content-type", "application/json")
                .body(Body::from(json!({"model":alias}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 503);
}
