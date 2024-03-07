mod client;

use client::Client;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let mut tasks = Vec::new();

    let task = task::spawn(async move {
        let target_addr = "127.0.0.1:9000";
        let mut client = Client::create(
            target_addr.to_owned(),
            "test".to_owned(),
            "testing".to_owned(),
        )
        .await;

        client.connect().await;
        client.do_login().await;

        let mut buf = [0u8; 1024];
        loop {
            let local_addr = client.socket.local_addr().unwrap();

            // TODO: Convert use of io::Socket to tokio::Socket, then await this
            let (size, peer) = client.socket.recv_from(&mut buf).await.unwrap();
            println!(
                "Client on port {} received data from {}: {:?}",
                local_addr.port(),
                peer,
                &buf[..size]
            );
        }
    });
    tasks.push(task);

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
