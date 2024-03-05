pub mod client;

use client::Client;

#[tokio::main]
async fn main() -> Result<(),()> {
    let bind_addr = "127.0.0.1:51472";
    let target_addr = "127.0.0.1:9000";

    let client = Client::create(bind_addr.to_owned(), target_addr.to_owned(), "test".to_owned(), "testing".to_owned());

    match client.connect() {
        Ok(received) => println!("Success: Received {} bytes.", received),
        Err(e) => println!("Failed: recv function failed: {e:?}"),
    }

    Ok(())
}
