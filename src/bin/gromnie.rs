use clap::{Parser, Subcommand};
use deku::DekuContainerRead;
use gromnie::{client::client::Client, net::{packet::PacketHeaderFlags, transit_header::TransitHeader}};

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

    let local_addr = client.socket.local_addr().unwrap();

    let mut buf = [0u8; 1024];
    loop {
        let (size, peer) = client.socket.recv_from(&mut buf).await.unwrap();

        // Try to parse into a Packet (basically, TransitHeader)
        let (_rest, packet) = TransitHeader::from_bytes((buf.as_ref(), 0)).unwrap();

        println!(
            "[NET/RECV] [client: {} on port: {} recv'd {} bytes from {}]",
            client.id,
            local_addr.port(),
            size,
            peer
        );
        println!("           {:02X?}", &buf[..size]);
        println!("           {:?}", packet);

        match PacketHeaderFlags::from_bits(packet.flags) {
            Some(v) => println!("{:?}", v),
            None => println!("Failed to parse PacketHeaderFlags."),
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

        // tasks.push(tokio::spawn(client_task(
        //     i.to_owned(),
        //     address.to_owned(),
        //     account_name.to_owned(),
        //     password.to_owned(),
        // )));
        tasks.push(tokio::spawn(client_task(
            i.to_owned(),
            address.to_owned(),
            // "acservertracker".to_owned(),
            // "jj9h26hcsggc".to_owned(),
            "testing".to_owned(),
            "testing".to_owned(),
        )));
    }

    for task in tasks {
        task.await.unwrap();
    }

    // Receive code
    // // //
    // let mut recv_buffer = [0u8; 1024];

    // let nbytes = socket.recv(&mut recv_buffer);

    // // TODO: Temporary code to parse response. Move this elsewhere when it's ready.
    // let mut recv_cursor = Cursor::new(&recv_buffer);
    // // parse_response(&mut recv_cursor);
    // parse_response(&recv_buffer);

    // nbytes
    Ok(())
}

use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    header: DekuHeader,
    data: DekuData,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
struct DekuHeader(u8);

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
struct DekuData(u16);
