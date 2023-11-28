use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    signal,
};
use tracing::error;
use tracing::info;
use tt_rust::agent::protocol::{system, Coordinator, Message};

#[tokio::main]
async fn main() {
    let mut socket = TcpStream::connect("localhost:7778").await.unwrap();

    let msg = Message::RequireSpace(10000);
    let buf = serde_xdr::to_bytes(&msg).unwrap();
    socket.write_all(&buf[..]).await.unwrap();

    let mut buf = [0; 30000];
    let n = socket.read(&mut buf).await.unwrap();
    assert!(n > 0);
    let msg: Message = serde_xdr::from_bytes(&buf[..n]).unwrap();
    println!("-> {:?}", msg);
}
