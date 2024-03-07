pub mod client;

use client::Client;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let target_addr = "127.0.0.1:9000";

    // TODO: This is marked mut I guess because the connect method modifies
    // the socket field when it's called. A refactor of how the socket is
    // created and set would probably be a good improvement
    let mut client = Client::create(
        target_addr.to_owned(),
        "test".to_owned(),
        "testing".to_owned(),
    );

    match client.connect() {
        Ok(received) => println!("Success: Received {} bytes.", received),
        Err(e) => println!("Failed: recv function failed: {e:?}"),
    }

    Ok(())
}
