use serde_derive::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use sysinfo::{DiskExt, System, SystemExt};
use tokio::net::UdpSocket;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Hello(String),
    RequireSpace(usize),
    HasSpace(usize),
}

pub struct Coordinator {
    socket: UdpSocket,
    hostname: String,
}

impl Coordinator {
    pub async fn new(addr: &str) -> Self {
        let socket = UdpSocket::bind(addr).await.unwrap();
        socket.set_broadcast(true).unwrap();
        let hostname = gethostname::gethostname().to_str().unwrap().to_string();

        Self { socket, hostname }
    }

    pub async fn hello(&self) {
        let mbuf = serde_xdr::to_bytes(&Message::Hello(self.hostname.clone())).unwrap();

        self.socket
            .send_to(&mbuf[..], (Ipv4Addr::BROADCAST, 7777u16))
            .await
            .unwrap();
    }

    pub async fn receive(&self) -> Message {
        let mut buf = [0; 1024];
        let (len, addr) = self.socket.recv_from(&mut buf).await.unwrap();
        println!("{:?} bytes received from {:?}", len, addr);
        let msg: Message = serde_xdr::from_bytes(&buf[..len]).unwrap();
        println!("message: {:?}", msg);
        msg
    }
}

pub fn system() {
    let mut sys = System::new_all();
    sys.refresh_all();
    for d in sys.disks() {
        println!("{:#?}", d);
        println!(
            "{} {}",
            d.mount_point().to_str().unwrap(),
            d.available_space()
        );
    }
}
