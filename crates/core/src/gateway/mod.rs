//! Authenticated loopback gateway. Protocol conversion remains in `protocol`.
mod history;
mod http;
mod probe;
use history::History;
pub use http::{bounded_json, http_client};
pub use probe::{probe, status_message, ProbeReceipt};
#[cfg(test)]
mod review_regressions;
#[cfg(test)]
mod tests;

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
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
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
#[derive(Clone)]
pub struct GatewayState {
    pub routes: Arc<RwLock<HashMap<String, Route>>>,
    pub token: String,
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
            client: http_client()?,
            history: Arc::new(RwLock::new(VecDeque::new())),
            active: Arc::new(AtomicUsize::new(0)),
            cancel: CancellationToken::new(),
        })
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
    forward_route(s, body, is_compact, primary, alias).await
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
    let sent = tokio::select! {
        _ = s.cancel.cancelled() => return error(StatusCode::SERVICE_UNAVAILABLE, "路由已关闭。"),
        response = s.client.post(url).bearer_auth(&route.key).json(&upstream).send() => response,
    };
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
        let next = tokio::select! {
            _ = s.cancel.cancelled() => return error(StatusCode::SERVICE_UNAVAILABLE, "路由已关闭。"),
            next = tokio::time::timeout(Duration::from_secs(90), raw.next()) => next,
        };
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
        // Keep the request counted until the response stream is dropped, not just
        // until its headers are returned; shutdown/restart consults this count.
        let _guard = guard;
        let mut incoming = incoming;
        let mut decoder = SseDecoder::default();
        let mut capture = true;
        loop {
            let next = tokio::select! {
                _ = s.cancel.cancelled() => {
                    yield Ok::<_, std::io::Error>(bytes::Bytes::from(protocol::failure("路由已关闭。")));
                    break;
                },
                next = tokio::time::timeout(Duration::from_secs(90), incoming.next()) => next,
            };
            match next {
                Ok(Some(Ok(chunk))) => {
                    if capture {
                        match decoder.push(&chunk) {
                            Ok(frames) => {
                                for frame in frames {
                                    s.remember_response_event(&alias, &input, &frame).await;
                                }
                            },
                            Err(_) => {
                                // A cache capture failure must not alter the upstream bytes.
                                capture = false;
                                decoder = SseDecoder::default();
                            },
                        }
                    }
                    yield Ok(chunk);
                },
                Ok(None) => break,
                _ => {
                    yield Ok(bytes::Bytes::from(protocol::failure("上游连接中断或超时。")));
                    break;
                },
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
