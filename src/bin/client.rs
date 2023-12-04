use std::io::{read_to_string, stdin};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::level_filters::LevelFilter;
use tt_rust::{
    agent::protocol::Message,
    init_tracing,
};

#[tokio::main]
async fn main() {
    init_tracing("client", LevelFilter::INFO);
    let s = read_to_string(stdin()).unwrap();

    let mut socket = TcpStream::connect("localhost:7778").await.unwrap();

    let msg = Message::Load(s);
    let buf = serde_xdr::to_bytes(&msg).unwrap();
    socket.write_all(&buf[..]).await.unwrap();

    let mut buf = [0; 30000];
    let n = socket.read(&mut buf).await.unwrap();
    assert!(n > 0);
    let msg: Message = serde_xdr::from_bytes(&buf[..n]).unwrap();
    println!("-> {:?}", msg);
}
