use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use futures::{SinkExt, StreamExt};
use tokio::net::UdpSocket;
use tokio_tungstenite::accept_async;
use tracing::{error, info};

use wisp_mux::{
    ServerMux, WispV2Handshake,
    extensions::{AnyProtocolExtensionBuilder, udp::UdpProtocolExtensionBuilder},
    packet::ConnectPacket,
    stream::MuxStream,
    ws::{Payload, TokioTungsteniteTransport},
};

type WsTransport = TokioTungsteniteTransport<tokio::net::TcpStream>;
type WsSplitWrite = futures::stream::SplitSink<WsTransport, Payload>;
type ProxyMuxStream = MuxStream<WsSplitWrite>;

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
    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    info!(listen = %args.listen, wisp_path = %args.wisp_path, "listening");

    loop {
        let (stream, addr) = listener.accept().await?;
        info!(%addr, "new connection");
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr).await {
                error!(%addr, "connection error: {e:#}");
            }
        });
    }
}

async fn handle_connection(stream: tokio::net::TcpStream, addr: SocketAddr) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    info!(%addr, "websocket established");

    let transport = TokioTungsteniteTransport(ws_stream);
    let (ws_write, ws_read) = futures::StreamExt::split(transport);

    let handshake = WispV2Handshake::new(vec![AnyProtocolExtensionBuilder::new(
        UdpProtocolExtensionBuilder,
    )]);

    let client = ServerMux::new(ws_read, ws_write, 65536, Some(handshake)).await?;

    let (mux, mux_task) = client.with_no_required_extensions();
    info!(%addr, "wisp handshake complete");

    tokio::spawn(async move {
        if let Err(e) = mux_task.await {
            error!(%addr, "mux task error: {e}");
        }
    });

    loop {
        match mux.wait_for_stream().await {
            Some((connect_pkt, stream)) => {
                let host = connect_pkt.host.clone();
                let port = connect_pkt.port;
                let stream_type = connect_pkt.stream_type;
                info!(%addr, %host, port, ?stream_type, "client opened stream");
                tokio::spawn(handle_stream(addr, connect_pkt, stream));
            }
            None => {
                info!(%addr, "connection closed");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_stream(
    client_addr: SocketAddr,
    connect_pkt: ConnectPacket,
    stream: ProxyMuxStream,
) -> Result<()> {
    let host = &connect_pkt.host;
    let port = connect_pkt.port;

    let game_addr = tokio::net::lookup_host(format!("{host}:{port}"))
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!("no addresses found for {host}:{port}"))?;
    let game_socket = UdpSocket::bind("0.0.0.0:0").await?;
    game_socket.connect(&game_addr).await?;
    info!(%client_addr, %game_addr, "udp forwarding started");

    let game_socket = std::sync::Arc::new(game_socket);
    let game_socket_read = game_socket.clone();
    let game_socket_write = game_socket.clone();

    // Split stream into write/read halves — split() returns (SplitSink, SplitStream)
    let (mut stream_tx, mut stream_rx) = futures::StreamExt::split(stream);

    let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();
    futures::pin_mut!(close_rx);

    // WISP -> UDP
    let forward = async {
        loop {
            match StreamExt::next(&mut stream_rx).await {
                Some(Ok(payload)) => {
                    let data: Vec<u8> = payload.into();
                    if let Err(e) = game_socket_write.send(&data).await {
                        error!(%client_addr, %game_addr, "udp send error: {e}");
                        break;
                    }
                }
                Some(Err(e)) => {
                    error!(%client_addr, %game_addr, "wisp recv error: {e}");
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
                            let payload = Payload::copy_from_slice(&buf[..len]);
                            if let Err(e) = stream_tx.send(payload).await {
                                error!(%client_addr, %game_addr, "wisp send error: {e}");
                                break;
                            }
                        }
                        Err(e) => {
                            error!(%client_addr, %game_addr, "udp recv error: {e}");
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

    info!(%client_addr, %game_addr, "udp forwarding stopped");
    Ok(())
}
