use libgromnie::uptime_client::UptimeClient;

#[tokio::main]
async fn main() -> Result<(),()> {
    let client = UptimeClient {
        bind_address: "0.0.0.0:9000".to_owned(),
        connect_address: "play.coldeve.online:9000".to_owned(),
    };

    match client.check() {
        Ok(received) => println!("Success: Received {} bytes.", received),
        Err(e) => println!("Failed: recv function failed: {e:?}"),
    }

    Ok(())
}
