use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::Result;
use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::routing::get;
use bytes::Bytes;
use clap::Parser;
use futures::{Sink, SinkExt, Stream, StreamExt};
use tower_http::services::ServeDir;
use tracing::{error, info};

use wisp_mux::{ServerMux, WispError, ws::TransportExt};

// ---------------------------------------------------------------------------
// WISP proxy
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "gromnie-proxy",
    about = "WISP proxy server for AC game servers"
)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:8080", env = "GROMNIE_LISTEN")]
    listen: SocketAddr,

    #[arg(long, default_value = "/")]
    wisp_path: String,

    #[arg(long)]
    static_dir: Option<PathBuf>,
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

    if let Some(sha) = option_env!("GIT_SHA") {
        info!(git_sha = sha, "starting");
    }

    let args = Args::parse();

    let app = Router::new().route(
        &args.wisp_path,
        get(|ws: axum::extract::ws::WebSocketUpgrade| async move { ws.on_upgrade(handle_ws) }),
    );
    let app = if let Some(ref dir) = args.static_dir {
        app.fallback_service(ServeDir::new(dir).append_index_html_on_directories(true))
    } else {
        app
    };

    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    info!(listen = %args.listen, wisp_path = %args.wisp_path, "listening");

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

    let (close_tx, mut close_rx) = tokio::sync::oneshot::channel::<()>();

    // WISP -> UDP: forward packets from the WISP stream to the game server.
    // When this direction ends (error, stream closed, or send failure),
    // signal the backward direction to shut down via close_tx.
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

    // UDP -> WISP: forward packets from the game server to the WISP stream.
    // Uses biased select to prioritize the shutdown signal (close_rx) so
    // the backward direction exits promptly when the forward direction ends.
    let backward = async {
        let mut buf = vec![0u8; 65536];
        loop {
            tokio::select! {
                biased;
                _ = &mut close_rx => {
                    info!(%game_addr, "UDP -> WISP: shutting down (forward direction ended)");
                    break;
                }
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
