use deku::prelude::*;
use clap::{Parser, Subcommand};

use gromnie::{client::client::Client, net::{packet::PacketHeaderFlags, packets::connect_request::ConnectRequestHeader, transit_header::TransitHeader}};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Enables debug mode
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}
#[derive(Subcommand)]
enum Commands {
    /// connect
    ///
    /// Connect to a server.
    ///
    /// Usage: gromnie connect -a localhost:9000 -u admin -p password
    Connect {
        /// Address to connect to in host:port syntax
        #[arg(short, long, value_name = "ADDRESS")]
        address: Option<String>,

        /// Account name
        #[arg(short, long, value_name = "USERNME")]
        username: Option<String>,

        /// Password
        #[arg(short, long, value_name = "PASSWORD")]
        password: Option<String>,
    },
}

async fn client_task(id: u32, address: String, account_name: String, password: String) {
    let mut client = Client::new(
        id.to_owned(),
        address.to_owned(),
        account_name.to_owned(),
        password.to_owned(),
    )
    .await;

    match client.connect().await {
        Ok(_) => {},
        Err(_) => panic!(),

    };

    match client.do_login().await {
        Ok(_) => {},
        Err(_) => panic!(),
    }

    // Grab this for logging later on
    let local_addr = client.socket.local_addr().unwrap();

    let mut buf = [0u8; 1024];

    loop {
        let (size, peer) = client.socket.recv_from(&mut buf).await.unwrap();

        // Pull out TransitHeader first and inspect
        let (_rest, packet) = TransitHeader::from_bytes((buf.as_ref(), 0)).unwrap();

        println!(
            "[NET/RECV] [client: {} on port: {} recv'd {} bytes from {}]",
            client.id,
            local_addr.port(),
            size,
            peer
        );
        println!("           -> raw: {:02X?}", &buf[..size]);
        println!("           -> packet: {:?}", packet);

        match PacketHeaderFlags::from_bits(packet.flags) {
            Some(v) => {
                println!("[RECVLOOP] Processing packet with PacketHeaderFlags: {}", v.to_string());

                if v == PacketHeaderFlags::ConnectRequest {
                    let packet = ConnectRequestHeader::from_bytes((&buf[..size], size)).unwrap();
                    println!("{:?}", packet.1);

                    let _ = client.do_connect_response(packet.1.cookie).await;
                }

                if v == PacketHeaderFlags::AckSequence {
                    println!("TODO: Send AckResponse")
                }
            },
            None => panic!("Failed to parse PacketHeaderFlags."),
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), ()> {
    // TODO: Finish CLI
    let _ = Cli::parse();

    // TODO: Wrap this up nicer
    let address = "localhost:9000";
    let account_name_prefix = "user";
    let _password = "password";

    let n = 1;
    let mut tasks = Vec::with_capacity(2);

    for i in 0..n {
        let mut account_name = account_name_prefix.to_owned();
        let suffix = i.to_string();
        account_name.push_str(suffix.as_ref());

        tasks.push(tokio::spawn(client_task(
            i.to_owned(),
            address.to_owned(),
            "testing".to_owned(),
            "testing".to_owned(),
        )));
    }

    for task in tasks {
        task.await.unwrap();
    }

    Ok(())
}
