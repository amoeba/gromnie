use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::Result;
use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket};
use axum::routing::get;
use bytes::Bytes;
use clap::Parser;
use futures::{Sink, SinkExt, Stream, StreamExt};
use tower_http::services::ServeDir;
use tracing::{error, info};

use wisp_mux::{
    ServerMux, WispV2Handshake,
    extensions::{AnyProtocolExtensionBuilder, udp::UdpProtocolExtensionBuilder},
};

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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,gromnie_proxy=debug".into()),
        )
        .init();

    let args = Args::parse();

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

    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    info!(listen = %args.listen, wisp_path = %args.wisp_path, static_dir = %args.static_dir.display(), "listening");

    axum::serve(listener, app).await?;

    Ok(())
}

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
                tokio::spawn(handle_stream(connect_pkt, stream));
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
