mod client;

use client::Client;
use tokio::task;

async fn client_task(address: String, account_name: String, password: String) {
    let mut client = Client::create(
        address.to_owned(),
        account_name.to_owned(),
        password.to_owned(),
    )
    .await;

    client.connect().await;
    client.do_login().await;

    let mut buf = [0u8; 1024];
    loop {
        // TODO: Convert use of io::Socket to tokio::Socket, then await this
        let (size, peer) = client.socket.recv_from(&mut buf).await.unwrap();

        let local_addr = client.socket.local_addr().unwrap();
        println!(
            "Client on port {} received data from {}: {:?}",
            local_addr.port(),
            peer,
            &buf[..size]
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    // TODO: Wrap this up nicer
    let address = "localhost:9000";
    let account_name_prefix = "test";
    let password = "password";

    let n = 2;
    let mut tasks = Vec::with_capacity(2);

    for i in 0..n {
        let mut account_name = account_name_prefix.to_owned();
        let suffix = i.to_string();
        account_name.push_str(suffix.as_ref());

        tasks.push(tokio::spawn(client_task(
            address.to_owned(),
            account_name.to_owned(),
            password.to_owned(),
        )));
    }

    for task in tasks {
        println!("about to await a task...");
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
