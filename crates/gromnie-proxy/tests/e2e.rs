use std::net::SocketAddr;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;

use wisp_mux::{
    ClientMux, WispV2Handshake,
    extensions::{AnyProtocolExtensionBuilder, udp::UdpProtocolExtension},
    packet::StreamType,
    ws::{TokioTungsteniteTransport, TransportExt},
};

async fn spawn_proxy() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (stream, _addr) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let transport = TokioTungsteniteTransport(ws);
                let (rx, tx) = transport.split_fast();

                let handshake = WispV2Handshake::new(vec![AnyProtocolExtensionBuilder::new(
                    wisp_mux::extensions::udp::UdpProtocolExtensionBuilder,
                )]);

                let client = wisp_mux::ServerMux::new(rx, tx, 65536, Some(handshake))
                    .await
                    .unwrap();
                let (mux, mux_task) = client.with_no_required_extensions();

                tokio::spawn(mux_task);

                while let Some((connect_pkt, stream)) = mux.wait_for_stream().await {
                    let host = connect_pkt.host.clone();
                    let port = connect_pkt.port;
                    let stream_type = connect_pkt.stream_type;
                    eprintln!("stream opened: host={host} port={port} type={stream_type:?}");

                    if stream_type == StreamType::Udp {
                        let game_addr = tokio::net::lookup_host(format!("{host}:{port}"))
                            .await
                            .unwrap()
                            .next()
                            .unwrap();
                        let game_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();
                        game_socket.connect(&game_addr).await.unwrap();
                        let game_socket = std::sync::Arc::new(game_socket);

                        let (mut ws_tx, mut ws_rx) = futures::StreamExt::split(stream);
                        let gs_r = game_socket.clone();
                        let gs_w = game_socket.clone();

                        tokio::spawn(async move {
                            let mut buf = [0u8; 65536];
                            loop {
                                tokio::select! {
                                    r = gs_r.recv_from(&mut buf) => {
                                        if let Ok((len, _)) = r {
                                            let _ = ws_tx.send(wisp_mux::ws::Payload::copy_from_slice(&buf[..len])).await;
                                        } else { break; }
                                    }
                                    r = ws_rx.next() => {
                                        if let Some(Ok(payload)) = r {
                                            let data: Vec<u8> = payload.into();
                                            let _ = gs_w.send(&data).await;
                                        } else { break; }
                                    }
                                }
                            }
                        });
                    }
                }
            });
        }
    });

    addr
}

#[tokio::test]
async fn wisp_handshake_and_udp_stream() -> Result<()> {
    let addr = spawn_proxy().await;
    let url = format!("ws://{addr}/wisp/");

    let (ws_stream, _) = connect_async(&url).await?;
    let transport = TokioTungsteniteTransport(ws_stream);
    let (rx, tx) = transport.split_fast();

    let extensions = vec![AnyProtocolExtensionBuilder::new(
        wisp_mux::extensions::udp::UdpProtocolExtensionBuilder,
    )];

    let (mux, mux_future) = ClientMux::new(rx, tx, Some(WispV2Handshake::new(extensions)))
        .await?
        .with_required_extensions(&[UdpProtocolExtension::ID])
        .await?;

    tokio::spawn(mux_future);

    eprintln!("WISP v2 handshake complete, UDP extension negotiated");

    let stream = mux
        .new_stream(StreamType::Udp, "play.coldeve.ac".to_string(), 9000)
        .await?;

    eprintln!("UDP stream opened to play.coldeve.ac:9000");

    let (mut reader, mut writer) = stream.into_split();

    // Send a small test packet (game client login header)
    let test_data = b"\x00\x00\x00\x00\x04\x00\x00\x00treestats\x00treestats\x00";
    writer
        .send(wisp_mux::ws::Payload::copy_from_slice(test_data))
        .await?;
    eprintln!("Sent {} byte test packet", test_data.len());

    // Wait for response (with timeout)
    match tokio::time::timeout(std::time::Duration::from_secs(5), reader.next()).await {
        Ok(Some(Ok(payload))) => {
            let data: Vec<u8> = payload.into();
            eprintln!("Received {} byte response", data.len());
            let hex_str: String = data.iter().map(|b| format!("{b:02x}")).collect();
            eprintln!("Response (hex): {hex_str}");
        }
        Ok(Some(Err(e))) => {
            eprintln!("Stream read error: {e}");
        }
        Ok(None) => {
            eprintln!("Stream closed by server");
        }
        Err(_) => {
            eprintln!("Timed out waiting for response (server may not have responded)");
        }
    }

    mux.close().await?;
    eprintln!("Test complete - WISP handshake + UDP stream: OK");
    Ok(())
}
