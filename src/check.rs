pub mod client;

use std::env;

use client::Client;

#[tokio::main]
async fn main() -> Result<(),()> {
    println!("main");
    // let args: Vec<String> = env::args().collect();

    // if args.len() != 3 {
    //     return Err((), ("wha"));
    // }

    // let bind_addr = &args[1];
    // let target_addr = &args[2];

    let bind_addr = "127.0.0.1:51472";
    let target_addr = "127.0.0.1:9000";

    let client = Client::create(bind_addr.to_owned(), target_addr.to_owned(), "test".to_owned(), "testing".to_owned());

    match client.connect() {
        Ok(received) => println!("Success: Received {} bytes.", received),
        Err(e) => println!("Failed: recv function failed: {e:?}"),
    }
    Ok(())
}
