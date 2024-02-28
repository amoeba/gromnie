use std::env;

use libgromnie::uptime_client::UptimeClient;

#[tokio::main]
async fn main() -> Result<(),()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        return Err(())
    }

    let bind_addr = &args[1];
    let target_addr = &args[2];

    let client = UptimeClient {
        bind_address: bind_addr.to_owned(),
        connect_address: target_addr.to_owned(),
    };

    match client.check() {
        Ok(received) => println!("Success: Received {} bytes.", received),
        Err(e) => println!("Failed: recv function failed: {e:?}"),
    }

    Ok(())
}
