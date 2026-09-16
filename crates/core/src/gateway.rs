use crate::{
    message,
    protocol::{self, SseDecoder},
    providers::{Connection, Protocol},
    Result,
};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Route {
    pub connection: Connection,
    pub key: String,
    /// Old client model identifiers that now use this current route.
    pub aliases: Vec<String>,
}
fn route_map(mut routes: Vec<Route>) -> HashMap<String, Route> {
    let mut map = HashMap::new();
    for route in &mut routes {
        for alias in std::mem::take(&mut route.aliases) {
            map.insert(alias, route.clone());
        }
    }
    for route in routes {
        map.insert(route.connection.alias(), route);
    }
    map
}
struct History {
    route: String,
    input: Vec<Value>,
    calls: Vec<Value>,
    created: Instant,
}
#[derive(Clone)]
pub struct GatewayState {
    pub routes: Arc<RwLock<HashMap<String, Route>>>,
    pub token: String,
    failover: Arc<RwLock<Vec<Route>>>,
    breakers: Arc<RwLock<HashMap<String, Arc<cc_switch_codex::CircuitBreaker>>>>,
    visible_models: Arc<RwLock<Vec<String>>>,
    client: reqwest::Client,
    history: Arc<RwLock<VecDeque<(String, History)>>>,
    pub active: Arc<AtomicUsize>,
    pub cancel: CancellationToken,
}
impl GatewayState {
    pub fn new(routes: Vec<Route>, token: String) -> Result<Self> {
        Ok(Self {
            visible_models: Arc::new(RwLock::new(
                routes.iter().map(|r| r.connection.alias()).collect(),
            )),
            routes: Arc::new(RwLock::new(route_map(routes))),
            token,
            failover: Default::default(),
            breakers: Default::default(),
            client: http_client()?,
            history: Arc::new(RwLock::new(VecDeque::new())),
            active: Arc::new(AtomicUsize::new(0)),
            cancel: CancellationToken::new(),
        })
    }
    pub async fn set_failover(&self, routes: Vec<Route>) {
        *self.failover.write().await = routes;
        self.breakers.write().await.clear();
    }
    pub async fn replace_routes(&self, routes: Vec<Route>) {
        let visible = routes.iter().map(|r| r.connection.alias()).collect();
        let mut m = self.routes.write().await;
        if let Some(default) = routes.first() {
            // An old conversation keeps its alias, but never its retired provider or Key.
            for route in m.values_mut() {
                *route = default.clone();
            }
        } else {
            m.clear();
        }
        m.extend(route_map(routes));
        *self.visible_models.write().await = visible;
    }
    async fn remember(&self, route: &str, input: &Value, response: &Value) {
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
    async fn remember_response_event(&self, alias: &str, input: &Value, frame: &str) {
        if let Ok(event) = serde_json::from_str::<Value>(frame) {
            if event["type"] == "response.completed" {
                self.remember(alias, input, &event["response"]).await;
            }
        }
    }
    async fn enrich(&self, route: &str, body: &mut Value) -> Result<()> {
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
pub fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|_| message("无法创建网络连接。"))
}
pub fn router(state: GatewayState) -> Router {
    Router::new()
        .route(
            "/healthz",
            get(|| async { Json(json!({"service":"codex-switch","status":"ok"})) }),
        )
        .route("/v1/models", get(models))
        .route("/v1/responses", post(responses))
        .route("/v1/responses/compact", post(compact))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .with_state(state)
}
fn authenticated(h: &HeaderMap, s: &GatewayState) -> bool {
    h.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .is_some_and(|t| bool::from(t.as_bytes().ct_eq(s.token.as_bytes())))
}
fn error(status: StatusCode, text: &str) -> Response {
    (
        status,
        Json(json!({"error":{"message":text,"type":"codex_switch_error"}})),
    )
        .into_response()
}
async fn models(State(s): State<GatewayState>, h: HeaderMap) -> Response {
    if !authenticated(&h, &s) {
        return error(StatusCode::UNAUTHORIZED, "本地连接凭据无效。");
    }
    Json(json!({"object":"list","data":s.visible_models.read().await.iter().map(|id|json!({"id":id,"object":"model","owned_by":"codex-switch"})).collect::<Vec<_>>()})).into_response()
}
struct Flight(Arc<AtomicUsize>);
impl Drop for Flight {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
async fn responses(
    State(s): State<GatewayState>,
    h: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    forward(s, h, body, false).await
}
async fn compact(State(s): State<GatewayState>, h: HeaderMap, Json(body): Json<Value>) -> Response {
    forward(s, h, body, true).await
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
async fn forward(s: GatewayState, h: HeaderMap, body: Value, is_compact: bool) -> Response {
    if !authenticated(&h, &s) {
        return error(StatusCode::UNAUTHORIZED, "本地连接凭据无效。");
    }
    if s.cancel.is_cancelled() {
        return error(StatusCode::SERVICE_UNAVAILABLE, "路由正在关闭。");
    }
    if !body.is_object() {
        return error(StatusCode::BAD_REQUEST, "请求必须为 JSON 对象。");
    }
    let alias = body["model"].as_str().unwrap_or_default().to_owned();
    let Some(primary) = s.routes.read().await.get(&alias).cloned() else {
        return error(
            StatusCode::BAD_REQUEST,
            "找不到此模型对应的连接，请重新选择模型。",
        );
    };
    let queue = s.failover.read().await.clone();
    if queue.is_empty() {
        return forward_route(s, body, is_compact, primary, alias).await;
    }
    // Like CC Switch, explicit failover uses only the ordered queue. Never infer a different protocol or reuse another provider's key.
    let mut last = None;
    for route in queue {
        if s.cancel.is_cancelled() {
            break;
        }
        let breaker = {
            let mut all = s.breakers.write().await;
            all.entry(route.connection.id.clone())
                .or_insert_with(|| {
                    Arc::new(cc_switch_codex::CircuitBreaker::new(Default::default()))
                })
                .clone()
        };
        let permit = breaker.allow_request().await;
        if !permit.allowed {
            continue;
        }
        let response =
            forward_route(s.clone(), body.clone(), is_compact, route, alias.clone()).await;
        let status = response.status();
        // Only pre-response transport/auth/rate-limit/availability failures are retried.
        // Local request validation errors and a committed SSE body are never replayed.
        if matches!(
            status.as_u16(),
            401 | 403 | 404 | 405 | 408 | 429 | 500 | 502 | 503 | 504
        ) {
            breaker.record_failure(permit.used_half_open_permit).await;
            last = Some(response);
            continue;
        }
        breaker.record_success(permit.used_half_open_permit).await;
        return response;
    }
    last.unwrap_or_else(|| {
        error(
            StatusCode::SERVICE_UNAVAILABLE,
            "备用队列中的服务暂时不可用。",
        )
    })
}
async fn forward_route(
    s: GatewayState,
    mut body: Value,
    is_compact: bool,
    route: Route,
    alias: String,
) -> Response {
    if s.active.fetch_add(1, Ordering::SeqCst) >= 8 {
        s.active.fetch_sub(1, Ordering::SeqCst);
        return error(StatusCode::TOO_MANY_REQUESTS, "并发请求过多，请稍后重试。");
    }
    let guard = Flight(s.active.clone());
    if route.connection.protocol == Protocol::Chat || body.get("previous_response_id").is_some() {
        if let Err(e) = s.enrich(&alias, &mut body).await {
            return error(StatusCode::BAD_REQUEST, &e.to_string());
        }
    }
    let input = body["input"].clone();
    let streaming = body["stream"].as_bool().unwrap_or(false);
    body["model"] = json!(route.connection.model);
    let (upstream, map) = if route.connection.protocol == Protocol::Chat {
        match protocol::chat_request_for(
            &body,
            route.connection.options.chat_reasoning.as_ref(),
            &route.connection.endpoint,
        ) {
            Ok(r) => r,
            Err(e) => return error(StatusCode::BAD_REQUEST, &e.to_string()),
        }
    } else {
        (body, protocol::ToolMap::default())
    };
    let path = if route.connection.protocol == Protocol::Chat {
        "chat/completions"
    } else if is_compact {
        "responses/compact"
    } else {
        "responses"
    };
    let url = match route.connection.upstream_url(path) {
        Ok(v) => v,
        Err(e) => return error(StatusCode::BAD_REQUEST, &e.to_string()),
    };
    let sent = tokio::select! {_ = s.cancel.cancelled()=>return error(StatusCode::SERVICE_UNAVAILABLE,"路由已关闭。"),r=s.client.post(url).bearer_auth(&route.key).json(&upstream).send()=>r};
    let response = match sent {
        Ok(r) => r,
        Err(_) => {
            return error(
                StatusCode::BAD_GATEWAY,
                "无法连接上游服务，请检查网络与地址。",
            )
        }
    };
    if !response.status().is_success() {
        return error(
            response.status(),
            status_message(response.status().as_u16()),
        );
    }
    if !streaming {
        let raw = match bounded_json(response).await {
            Ok(v) => v,
            Err(e) => return error(StatusCode::BAD_GATEWAY, &e.to_string()),
        };
        let output = if route.connection.protocol == Protocol::Chat {
            match protocol::chat_response(&raw, &alias, map) {
                Ok(v) => v,
                Err(e) => return error(StatusCode::BAD_GATEWAY, &e.to_string()),
            }
        } else {
            raw
        };
        s.remember(&alias, &input, &output).await;
        return Json(output).into_response();
    }
    if !response
        .headers()
        .get("content-type")
        .and_then(|x| x.to_str().ok())
        .is_some_and(|x| x.contains("text/event-stream"))
    {
        return error(StatusCode::BAD_GATEWAY, "上游没有返回流式事件。");
    }
    let mut raw = response.bytes_stream();
    let first = loop {
        let next = tokio::select! {_ = s.cancel.cancelled()=>return error(StatusCode::SERVICE_UNAVAILABLE,"路由已关闭。"),next=tokio::time::timeout(Duration::from_secs(90),raw.next())=>next};
        match next {
            Ok(Some(Ok(chunk))) if chunk.is_empty() => continue,
            Ok(Some(Ok(chunk))) => break chunk,
            _ => return error(StatusCode::BAD_GATEWAY, "上游流在返回数据前中断或超时。"),
        }
    };
    let raw = futures_util::stream::once(async move { Ok::<_, reqwest::Error>(first) }).chain(raw);
    let incoming: std::pin::Pin<
        Box<
            dyn futures_util::Stream<Item = std::result::Result<bytes::Bytes, std::io::Error>>
                + Send,
        >,
    > = if route.connection.protocol == Protocol::Chat {
        Box::pin(protocol::chat_stream(raw, map))
    } else {
        Box::pin(raw.map(|v| v.map_err(|_| std::io::Error::other("上游读取失败。"))))
    };
    let stream = async_stream::stream! {
        let _guard=guard; let mut incoming=incoming; let mut decoder=SseDecoder::default(); let mut capture=true;
        loop {
            let next=tokio::select!{_ = s.cancel.cancelled()=>{yield Ok::<_,std::io::Error>(bytes::Bytes::from(protocol::failure("路由已关闭。")));break}, r=tokio::time::timeout(Duration::from_secs(90),incoming.next())=>r};
            match next {
                Ok(Some(Ok(chunk)))=>{
                    if capture {match decoder.push(&chunk) {
                        Ok(frames)=>for frame in frames {s.remember_response_event(&alias,&input,&frame).await;},
                        Err(_)=>{capture=false;decoder=SseDecoder::default();}
                    }}
                    yield Ok(chunk);
                },
                Ok(None)=>break,
                _=>{yield Ok(bytes::Bytes::from(protocol::failure("上游连接中断或超时。")));break;}
            }
        }
    };
    (
        [
            ("content-type", "text/event-stream"),
            ("cache-control", "no-cache"),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}

pub struct Gateway {
    pub state: GatewayState,
    pub port: u16,
    task: tokio::task::JoinHandle<()>,
}
impl Gateway {
    pub async fn start(routes: Vec<Route>, port: u16, token: String) -> Result<Self> {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .map_err(|_| message("本地端口被占用，未修改 Codex 配置。"))?;
        let port = listener.local_addr()?.port();
        let state = GatewayState::new(routes, token)?;
        let cancel = state.cancel.clone();
        let app = router(state.clone());
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(cancel.cancelled_owned())
                .await;
        });
        Ok(Self { state, port, task })
    }
    pub fn requests(&self) -> usize {
        self.state.active.load(Ordering::SeqCst)
    }
    pub fn stop(&self) {
        self.state.cancel.cancel();
    }
}
impl Drop for Gateway {
    fn drop(&mut self) {
        self.state.cancel.cancel();
        self.task.abort();
    }
}

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

#[cfg(test)]
mod tests {
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
}

#[cfg(test)]
mod review_regressions {
    use super::*;
    #[tokio::test]
    async fn review_stream_tool_replay_restores_reasoning_without_previous_id() {
        let tools = json!([{"type":"function","name":"read_file","parameters":{"type":"object","properties":{}}}]);
        let (_, map) =
            protocol::chat_request(&json!({"model":"mock","input":"read","tools":tools})).unwrap();
        let chunk = json!({"choices":[{"delta":{"reasoning_content":"synthetic reasoning","tool_calls":[{"index":0,"id":"call_mock","type":"function","function":{"name":"read_file","arguments":"{}"}}]},"finish_reason":"tool_calls"}]});
        let incoming = futures_util::stream::iter(vec![Ok::<_, std::io::Error>(
            bytes::Bytes::from(format!("data: {chunk}\n\ndata: [DONE]\n\n")),
        )]);
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
            .find(|v| {
                v["type"] == "response.output_item.done" && v["item"]["type"] == "function_call"
            })
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
        s.remember("route-a", &json!([]), &json!({"id":"r2","status":"completed","output":[call_item("a", "conflicting metadata")]})).await;
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
}
