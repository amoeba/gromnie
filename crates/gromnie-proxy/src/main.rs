use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;

use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::http;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket};
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
    ServerMux, WispV2Handshake,
    extensions::{AnyProtocolExtensionBuilder, udp::UdpProtocolExtensionBuilder},
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
    S: Service<http::Request<Body>, Response = http::Response<Body>, Error = std::convert::Infallible>
        + Clone
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
                    if let Some(val) = part.strip_prefix("gromnie_session=")
                        && verify_session(val, &config.secret_key).is_some()
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

#[derive(Clone)]
struct AppState {}

struct AxumTransportRead {
    rx: tokio::sync::mpsc::Receiver<Result<Bytes, wisp_mux::WispError>>,
}

struct AxumTransportWrite {
    tx: tokio::sync::mpsc::Sender<Bytes>,
    _ws_task: tokio::task::JoinHandle<()>,
}

impl Stream for AxumTransportRead {
    type Item = Result<Bytes, wisp_mux::WispError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(result)) => Poll::Ready(Some(result)),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Sink<Bytes> for AxumTransportWrite {
    type Error = wisp_mux::WispError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        self.tx
            .try_send(item)
            .map_err(|_| wisp_mux::WispError::WsImplSocketClosed)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

fn split_axum_ws(mut ws: WebSocket) -> (AxumTransportRead, AxumTransportWrite) {
    let (read_tx, read_rx) = tokio::sync::mpsc::channel(64);
    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    let ws_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = ws.next() => {
                    match msg {
                        Some(Ok(Message::Binary(data))) => {
                            let payload = Bytes::from(data.to_vec());
                            if read_tx.send(Ok(payload)).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Close(_))) => break,
                        Some(Ok(_)) => {}
                        Some(Err(_)) => break,
                        None => break,
                    }
                }
                data = write_rx.recv() => {
                    match data {
                        Some(bytes) => {
                            if ws.send(Message::Binary(bytes)).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    });

    (
        AxumTransportRead { rx: read_rx },
        AxumTransportWrite {
            tx: write_tx,
            _ws_task: ws_task,
        },
    )
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
            let user = std::env::var("AUTH_USER")
                .expect("AUTH_USER must be set when AUTH_ENABLE=true");
            let pass = std::env::var("AUTH_PASSWORD")
                .expect("AUTH_PASSWORD must be set when AUTH_ENABLE=true");
            let secret = std::env::var("AUTH_SECRET")
                .unwrap_or_else(|_| {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut h = DefaultHasher::new();
                    user.hash(&mut h);
                    pass.hash(&mut h);
                    format!("{:016x}", h.finish())
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

    let state = AppState {};

    let app = Router::new()
        .route(
            &args.wisp_path,
            get(
                |ws: axum::extract::ws::WebSocketUpgrade, _state: State<AppState>| async move {
                    ws.on_upgrade(handle_ws)
                },
            ),
        )
        .fallback_service(ServeDir::new(&args.static_dir).append_index_html_on_directories(true))
        .with_state(state);

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
    let (transport_read, transport_write) = split_axum_ws(socket);

    let handshake = WispV2Handshake::new(vec![AnyProtocolExtensionBuilder::new(
        UdpProtocolExtensionBuilder,
    )]);

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

async fn handle_stream(
    connect_pkt: wisp_mux::packet::ConnectPacket,
    stream: wisp_mux::stream::MuxStream<AxumTransportWrite>,
) -> Result<()> {
    let host = &connect_pkt.host;
    let port = connect_pkt.port;

    let game_addr = tokio::net::lookup_host(format!("{host}:{port}"))
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!("no addresses found for {host}:{port}"))?;
    let game_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    game_socket.connect(&game_addr).await?;
    info!(%game_addr, "udp forwarding started");

    let game_socket = std::sync::Arc::new(game_socket);
    let game_socket_read = game_socket.clone();
    let game_socket_write = game_socket.clone();

    let (mut stream_tx, mut stream_rx) = futures::StreamExt::split(stream);

    let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();
    futures::pin_mut!(close_rx);

    // WISP -> UDP
    let forward = async {
        loop {
            match StreamExt::next(&mut stream_rx).await {
                Some(Ok(payload)) => {
                    if let Err(e) = game_socket_write.send(&payload).await {
                        error!(%game_addr, "udp send error: {e}");
                        break;
                    }
                }
                Some(Err(e)) => {
                    error!(%game_addr, "wisp recv error: {e}");
                    break;
                }
                None => break,
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
                        Ok((len, _)) => {
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
