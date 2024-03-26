async fn client_task(id: u32, address: String, account_name: String, password: String) {
  let mut client = Client::create(
      id.to_owned(),
      address.to_owned(),
      account_name.to_owned(),
      password.to_owned(),
  )
  .await;

  client.connect().await;
  client.do_login().await;

  let local_addr = client.socket.local_addr().unwrap();

  let mut buf = [0u8; 1024];
  loop {
      let (size, peer) = client.socket.recv_from(&mut buf).await.unwrap();

      println!(
          "[NET/RECV] [client: {} on port: {} recv'd {} bytes from {}]",
          client.id,
          local_addr.port(),
          size,
          peer
      );
  }
}
