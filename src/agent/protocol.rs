use crossterm::execute;
use serde_derive::{Deserialize, Serialize};
use std::{cmp::Ordering, io::stdout, net::Ipv4Addr, sync::Mutex};
use sysinfo::{DiskExt, System, SystemExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
};
use tracing::{info, error};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Empty,
    Hello(String),
    RequireSpace(usize),
    RequireSpaceHere(usize),
    HasSpace(String, usize),
    Load(String),
    LoadHere(String, String),
    Status(String),
}

pub struct Coordinator {
    socket: UdpSocket,
    hostname: String,
    known_hosts: Mutex<Vec<String>>,
    monitor: Mutex<Option<tokio::sync::mpsc::Sender<Message>>>,
}

impl Coordinator {
    pub fn hostname(&self) -> String {
        self.hostname.clone()
    }

    pub async fn new(addr: &str) -> Self {
        let socket = UdpSocket::bind(addr).await.unwrap();
        socket.set_broadcast(true).unwrap();
        let hostname = gethostname::gethostname().to_str().unwrap().to_string();

        Self {
            socket,
            hostname,
            known_hosts: Mutex::new(vec![]),
            monitor: Mutex::new(None),
        }
    }

    pub async fn hello(&self) {
        self.broadcast(&Message::Hello(self.hostname.clone()));
    }


    pub async fn broadcast(&self, msg: &Message) {
        let mbuf = serde_xdr::to_bytes(msg).unwrap();

        self.socket
            .send_to(&mbuf[..], (Ipv4Addr::BROADCAST, 7777u16))
            .await
            .unwrap();
    }

    pub async fn receive(&self) -> Message {
        let mut buf = [0; 1024];
        let (len, addr) = self.socket.recv_from(&mut buf).await.unwrap();
        // println!("{:?} bytes received from {:?}", len, addr);
        let msg: Message = serde_xdr::from_bytes(&buf[..len]).unwrap();

        // println!("message: {:?}", msg);
        msg
    }

    pub async fn show_monitor(&self) {
        use std::io::Write;
        // crossterm::terminal::enable_raw_mode().unwrap();
        let mut f = stdout();
        execute!(
            f,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        )
        .unwrap();
        let (a, mut b) = tokio::sync::mpsc::channel::<Message>(10);
        {
            let mut mon = self.monitor.try_lock().unwrap();
            *mon = Some(a);
        }
        info!("monitor started.");
        loop {
            execute!(f, crossterm::cursor::MoveTo(1, 1), crossterm::cursor::Hide).unwrap();
            write!(f, "Host: {}", self.hostname).unwrap();
            {
                let mut line = 3;
                for h in self.known_hosts.try_lock().unwrap().iter() {
                    execute!(f, crossterm::cursor::MoveTo(1, line)).unwrap();
                    write!(f, "known Host: {}", h).unwrap();
                    line += 1;
                }
            }
            f.flush().unwrap();
            info!("monitor waiting for message");
            if let Some(msg) = b.recv().await {
                match msg {
                    Message::Status(status) => {
                        execute!(f, crossterm::cursor::MoveTo(1, 10)).unwrap();
                        write!(f, "Status: {}", status).unwrap();
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
    }

    pub async fn run(&self) {
        loop {
            let msg = self.receive().await;
            let sender = { self.monitor.try_lock().unwrap().clone() };
            match msg {
                Message::Hello(x) => {
                    let mut known_hosts = self.known_hosts.try_lock().unwrap();

                    if !known_hosts.iter().any(|y| x == *y) {
                        known_hosts.push(x);
                        self.hello();
                    }
                    if let Some(s) = &sender {
                        s.send(Message::Empty).await.unwrap();
                    }
                }
                Message::Status(status) => {
                    info!("received status: {}", status);
                    if let Some(s) = &sender {
                        s.send(Message::Status(status)).await.unwrap();
                    }
                    else {
                        error!("no monitor attached.");
                    }
                }
                _ => info!("msg -> {:?}", msg),
            }
        }
    }

    pub async fn require_space(&self, n: usize) -> (usize, String) {
        let mut results = vec![];
        let hosts: Vec<String> = {
            self.known_hosts
                .try_lock()
                .unwrap()
                .iter()
                .cloned()
                .collect()
        };
        for h in hosts.iter() {
            if let Message::HasSpace(_, s) =
                remote_call(h.as_str(), &Message::RequireSpaceHere(n)).await
            {
                results.push((h.as_str(), s));
            }
        }
        info!("space request results: {:?}", results);
        results.sort_by(|(_, x), (_, y)| {
            if *x > *y {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });
        let (host, size) = results[0];
        (size, String::from(host))
    }
}

pub async fn remote_call(host: &str, msg: &Message) -> Message {
    let mut socket = TcpStream::connect(format!("{}:7778", host)).await.unwrap();

    let buf = serde_xdr::to_bytes(msg).unwrap();
    socket.write_all(&buf[..]).await.unwrap();

    let mut buf = [0; 30000];
    let n = socket.read(&mut buf).await.unwrap();
    assert!(n > 0);
    let rmsg: Message = serde_xdr::from_bytes(&buf[..n]).unwrap();
    rmsg
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
