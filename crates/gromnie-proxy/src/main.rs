use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket};
use axum::http;
use axum::routing::get;
use base64::Engine;
use bytes::Bytes;
use clap::Parser;
use cookie::Cookie;
use futures::{Sink, SinkExt, Stream, StreamExt};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tower::Layer;
use tower::Service;
use tower_http::services::ServeDir;
use tracing::{error, info};

use wisp_mux::{
    ServerMux, WispError,
    ws::TransportExt,
};

// ---------------------------------------------------------------------------
// Session cookie helpers
// ---------------------------------------------------------------------------

const SESSION_COOKIE_NAME: &str = "gromnie_session";
const SESSION_MAX_AGE_SECS: u64 = 86400;

fn sign_session(user: &str, secret: &[u8]) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let payload = format!("{user}:{ts}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    let sig = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    format!("{user}:{ts}:{sig}")
}

fn verify_session(session: &str, secret: &[u8]) -> Option<String> {
    let parts: Vec<&str> = session.splitn(3, ':').collect();
    if parts.len() != 3 {
        return None;
    }
    let (user, ts_str, sig_b64) = (parts[0], parts[1], parts[2]);
    let ts: u64 = ts_str.parse().ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if now.saturating_sub(ts) > SESSION_MAX_AGE_SECS {
        return None;
    }
    let payload = format!("{user}:{ts}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(sig_b64)
        .ok()?;
    mac.verify_slice(&sig_bytes).ok()?;
    Some(user.to_owned())
}

fn set_session_cookie(user: &str, secret: &[u8]) -> String {
    let token = sign_session(user, secret);
    Cookie::build((SESSION_COOKIE_NAME, token))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::Strict)
        .max_age(time::Duration::seconds(SESSION_MAX_AGE_SECS as i64))
        .to_string()
}

// ---------------------------------------------------------------------------
// Basic auth middleware (cookie-based for WebSocket compatibility)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AuthConfig {
    username: String,
    password: String,
    secret_key: Vec<u8>,
}

#[derive(Clone)]
struct BasicAuthLayer {
    config: AuthConfig,
}

impl BasicAuthLayer {
    fn new(config: AuthConfig) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for BasicAuthLayer {
    type Service = BasicAuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BasicAuthMiddleware {
            inner,
            config: self.config.clone(),
        }
    }
}

#[derive(Clone)]
struct BasicAuthMiddleware<S> {
    inner: S,
    config: AuthConfig,
}

impl<S> Service<http::Request<Body>> for BasicAuthMiddleware<S>
where
    S: Service<
            http::Request<Body>,
            Response = http::Response<Body>,
            Error = std::convert::Infallible,
        > + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    type Response = http::Response<Body>;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let config = self.config.clone();

        Box::pin(async move {
            // 1. Check session cookie
            if let Some(cookie_header) = req.headers().get(http::header::COOKIE)
                && let Ok(cookie_str) = cookie_header.to_str()
            {
                for part in cookie_str.split(';') {
                    let part = part.trim();
                    if let Ok(c) = cookie::Cookie::parse(part)
                        && c.name() == "gromnie_session"
                        && verify_session(c.value(), &config.secret_key).is_some()
                    {
                        return inner.call(req).await;
                    }
                }
            }

            // 2. Check HTTP Basic auth header
            if let Some(auth_header) = req.headers().get(http::header::AUTHORIZATION)
                && let Ok(auth_str) = auth_header.to_str()
                && let Some(encoded) = auth_str.strip_prefix("Basic ")
                && let Ok(decoded) = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(encoded))
                && let Ok(cred) = String::from_utf8(decoded)
                && let Some((user, pass)) = cred.split_once(':')
                && user == config.username
                && pass == config.password
            {
                let cookie_val = set_session_cookie(user, &config.secret_key);
                let mut resp = inner.call(req).await?;
                resp.headers_mut().insert(
                    http::header::SET_COOKIE,
                    http::HeaderValue::from_str(&cookie_val).unwrap(),
                );
                return Ok(resp);
            }

            // 3. No valid credentials
            Ok(http::Response::builder()
                .status(http::StatusCode::UNAUTHORIZED)
                .header(
                    http::header::WWW_AUTHENTICATE,
                    r#"Basic realm="gromnie", charset="UTF-8""#,
                )
                .body(Body::from("401 Unauthorized"))
                .unwrap())
        })
    }
}

// ---------------------------------------------------------------------------
// WISP proxy (unchanged)
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "gromnie-proxy",
    about = "WISP proxy server for AC game servers"
)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:8080")]
    listen: SocketAddr,

    #[arg(long, default_value = "/wisp/")]
    wisp_path: String,

    #[arg(long, default_value = "/app/static")]
    static_dir: PathBuf,
}

/// Thin wrapper around axum's [`WebSocket`] that implements `TransportRead`
/// and `TransportWrite` (i.e. `Stream<Item = Result<Bytes, WispError>>` and
/// `Sink<Bytes, Error = WispError>`).
///
/// This replaces the previous ~80 lines of custom `AxumTransportRead` /
/// `AxumTransportWrite` / `split_axum_ws` boilerplate (mpsc channels + a
/// background `tokio::select!` task) with a direct delegation to axum's
/// built-in `Stream` and `Sink` implementations for `WebSocket`.
///
/// Benefits over the old approach:
/// - No background task to leak (the old `poll_close` was a no-op that left
///   the task dangling).
/// - `poll_ready` / `poll_close` properly delegate to the underlying socket,
///   so backpressure and graceful shutdown work correctly.
struct AxumWsTransport {
    ws: WebSocket,
}

impl AxumWsTransport {
    fn new(ws: WebSocket) -> Self {
        Self { ws }
    }
}

impl Stream for AxumWsTransport {
    type Item = Result<Bytes, WispError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut ws = Pin::new(&mut self.get_mut().ws);
        loop {
            match ws.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(Message::Binary(data)))) => {
                    return Poll::Ready(Some(Ok(data)));
                }
                Poll::Ready(Some(Ok(Message::Close(_)))) => return Poll::Ready(None),
                // Skip non-binary, non-close frames (Text, Ping, Pong).
                Poll::Ready(Some(Ok(_))) => continue,
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(WispError::WsImplError(Box::new(e)))));
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl Sink<Bytes> for AxumWsTransport {
    type Error = WispError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().ws)
            .poll_ready(cx)
            .map_err(|e| WispError::WsImplError(Box::new(e)))
    }

    fn start_send(self: Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().ws)
            .start_send(Message::Binary(item))
            .map_err(|e| WispError::WsImplError(Box::new(e)))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().ws)
            .poll_flush(cx)
            .map_err(|e| WispError::WsImplError(Box::new(e)))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().ws)
            .poll_close(cx)
            .map_err(|e| WispError::WsImplError(Box::new(e)))
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,gromnie_proxy=debug".into()),
        )
        .init();

    let args = Args::parse();

    // Auth configuration via environment variables
    let auth_config = match std::env::var("AUTH_ENABLE") {
        Ok(val) if val == "true" || val == "1" => {
            let user =
                std::env::var("AUTH_USER").expect("AUTH_USER must be set when AUTH_ENABLE=true");
            let pass = std::env::var("AUTH_PASSWORD")
                .expect("AUTH_PASSWORD must be set when AUTH_ENABLE=true");
            let secret = std::env::var("AUTH_SECRET").unwrap_or_else(|_| {
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(user.as_bytes());
                h.update(pass.as_bytes());
                h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
            });
            info!(user = %user, "basic auth enabled");
            Some(AuthConfig {
                username: user,
                password: pass,
                secret_key: secret.into_bytes(),
            })
        }
        _ => {
            info!("basic auth disabled");
            None
        }
    };

    let app = Router::new()
        .route(
            &args.wisp_path,
            get(
                |ws: axum::extract::ws::WebSocketUpgrade| async move {
                    ws.on_upgrade(handle_ws)
                },
            ),
        )
        .fallback_service(ServeDir::new(&args.static_dir).append_index_html_on_directories(true));

    let app = if let Some(config) = auth_config {
        app.layer(BasicAuthLayer::new(config))
    } else {
        app
    };

    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    info!(listen = %args.listen, wisp_path = %args.wisp_path, static_dir = %args.static_dir.display(), "listening");

    axum::serve(listener, app).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// WebSocket / stream handlers (unchanged)
// ---------------------------------------------------------------------------

async fn handle_ws(socket: WebSocket) {
    let transport = AxumWsTransport::new(socket);
    let (transport_read, transport_write) = transport.split_fast();

    let handshake = gromnie_wisp::default_wisp_handshake();

    let client = match ServerMux::new(transport_read, transport_write, 65536, Some(handshake)).await
    {
        Ok(c) => c,
        Err(e) => {
            error!("wisp handshake failed: {e}");
            return;
        }
    };

    let (mux, mux_task) = client.with_no_required_extensions();
    info!("wisp handshake complete");

    tokio::spawn(async move {
        if let Err(e) = mux_task.await {
            error!("mux task error: {e}");
        }
    });

    loop {
        match mux.wait_for_stream().await {
            Some((connect_pkt, stream)) => {
                let host = connect_pkt.host.clone();
                let port = connect_pkt.port;
                let stream_type = connect_pkt.stream_type;
                info!(%host, port, ?stream_type, "client opened stream");
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(connect_pkt, stream).await {
                        error!(host = %host, port, "stream failed: {e:#}");
                    }
                });
            }
            None => {
                info!("wisp connection closed");
                break;
            }
        }
    }
}

async fn handle_stream<W: wisp_mux::ws::TransportWrite>(
    connect_pkt: wisp_mux::packet::ConnectPacket,
    stream: wisp_mux::stream::MuxStream<W>,
) -> Result<()> {
    let host = &connect_pkt.host;
    let port = connect_pkt.port;
    info!(%host, port, "handle_stream: starting forwarding");

    let game_addr = tokio::net::lookup_host(format!("{host}:{port}"))
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!("no addresses found for {host}:{port}"))?;
    let game_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    game_socket.connect(&game_addr).await?;
    info!(%game_addr, local = %game_socket.local_addr().unwrap(), "udp forwarding started");

    let game_socket = std::sync::Arc::new(game_socket);
    let game_socket_read = game_socket.clone();
    let game_socket_write = game_socket.clone();

    let (mut stream_tx, mut stream_rx) = futures::StreamExt::split(stream);

    let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();
    futures::pin_mut!(close_rx);

    // WISP -> UDP
    let forward = async {
        let mut pkt_count: u64 = 0;
        loop {
            match StreamExt::next(&mut stream_rx).await {
                Some(Ok(payload)) => {
                    pkt_count += 1;
                    let hex_preview = gromnie_wisp::hex_preview(&payload, 20);
                    info!(%game_addr, len = payload.len(), pkt_count, "WISP -> UDP: forwarding {} bytes | {}", payload.len(), hex_preview);
                    if let Err(e) = game_socket_write.send(&payload).await {
                        error!(%game_addr, "udp send error: {e}");
                        break;
                    }
                }
                Some(Err(e)) => {
                    error!(%game_addr, "wisp recv error: {e}");
                    break;
                }
                None => {
                    info!(%game_addr, "WISP -> UDP: stream ended (no more data from client)");
                    break;
                }
            }
        }
        let _ = close_tx.send(());
    };

    // UDP -> WISP
    let backward = async {
        let mut buf = vec![0u8; 65536];
        loop {
            tokio::select! {
                result = game_socket_read.recv_from(&mut buf) => {
                    match result {
                        Ok((len, src)) => {
                            let hex_preview = gromnie_wisp::hex_preview(&buf[..len], 20);
                            info!(%game_addr, len, %src, "UDP -> WISP: received {} bytes from game server | {}", len, hex_preview);
                            let data = Bytes::copy_from_slice(&buf[..len]);
                            if let Err(e) = stream_tx.send(data).await {
                                error!(%game_addr, "wisp send error: {e}");
                                break;
                            }
                        }
                        Err(e) => {
                            error!(%game_addr, "udp recv error: {e}");
                            break;
                        }
                    }
                }
                _ = &mut close_rx => {
                    info!(%game_addr, "UDP -> WISP: shutting down (forward direction ended)");
                    break;
                }
            }
        }
    };

    tokio::select! {
        _ = forward => {},
        _ = backward => {},
    }

    info!(%game_addr, "udp forwarding stopped");
    Ok(())
}
