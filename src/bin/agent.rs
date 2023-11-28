use tokio::{io::AsyncWriteExt, net::TcpListener, signal};
use tracing::error;
use tracing::info;
use tt_rust::{
    agent::protocol::{system, Coordinator, Message},
    TRACING,
};

#[tokio::main]
async fn main() {
    assert!(TRACING.clone());
    system();
    let c = Coordinator::new("0.0.0.0:7777").await;

    c.hello().await;
    let local_net = format!("{}:{}", "0.0.0.0", 7778);

    // Bind the listener to the address
    match TcpListener::bind(&local_net).await {
        Ok(listener) => {
            let ctrl_c = signal::ctrl_c();
            tokio::select! {
                _ = run_agent_loop(c) => { error!("agent loop ended.");}
                _ =  run(listener) => { error!("processing ended.");}
                _ = ctrl_c => { info!("shutting down on ctrl-c.");}
            }
        }
        Err(e) => error!("bind: {local_net} <- {e}"),
    };
}
async fn run_agent_loop(c: Coordinator) {
    loop {
        let msg = c.receive().await;
        info!("msg -> {:?}", msg);
    }
}
async fn run(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                info!("new connection {:?}", socket);
                tokio::spawn(async move {
                    let mut buf = [0; 30000];
                    let x = socket.try_read(&mut buf).unwrap();
                    let msg: Message = serde_xdr::from_bytes(&buf[..x]).unwrap();
                    println!("received message -> {:?}", msg);
                    match msg {
                        Message::RequireSpace(n) => {
                            let response = Message::HasSpace(12345);
                            let buf = serde_xdr::to_bytes(&response).unwrap();
                            socket.write_all(&buf[..]).await.unwrap();
                        }
                        _ => todo!("{:?}", msg),
                    }
                });
            }
            Err(e) => error!("accept: {e}"),
        }
    }
}
