use crossterm::execute;
use serde_derive::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap, io::stdout, net::Ipv4Addr, sync::Mutex};
use sysinfo::{DiskExt, System, SystemExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentStatusInfo {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Empty,
    Hello(String),
    RequireSpace(usize),
    RequireSpaceHere(usize),
    HasSpace(String, usize),
    Load(String),
    LoadHere(String, String),
    Status(String, String),
    ReadStatus,
    StatusResponse { agents: Vec<AgentStatusInfo> },

    /// list all local files created or modified after given change number
    List(usize),

    /// list all files created or modified after given change number
    ListAll(usize),

    /// result list of all entries found.
    ListResult{entries: Vec<String>},
}

pub struct Coordinator {
    socket: UdpSocket,
    hostname: String,
    known_hosts: Mutex<Vec<String>>,
    monitor: Mutex<Option<tokio::sync::mpsc::Sender<Message>>>,
    jobs: Mutex<BTreeMap<String, String>>,
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
            jobs: Mutex::new(BTreeMap::<String, String>::new()),
        }
    }

    pub async fn hello(&self) {
        info!("sending hello");
        self.broadcast(&Message::Hello(self.hostname.clone())).await;
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
        let (len, _addr) = self.socket.recv_from(&mut buf).await.unwrap();
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
                    Message::Status(jobid, status) => {
                        let mut jobs = self.jobs.try_lock().unwrap();
                        jobs.insert(jobid, status);
                        let mut idx = 10;
                        for (k, v) in jobs.iter() {
                            execute!(f, crossterm::cursor::MoveTo(1, idx)).unwrap();
                            execute!(
                                f,
                                crossterm::terminal::Clear(
                                    crossterm::terminal::ClearType::CurrentLine
                                )
                            )
                            .unwrap();
                            write!(f, "{} -> {}", k, v).unwrap();
                            idx += 1;
                        }
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
                        self.hello().await;
                    }
                    if let Some(s) = &sender {
                        s.send(Message::Empty).await.unwrap();
                    }
                }
                Message::Status(jobid, status) => {
                    info!("received status: {}", status);
                    if let Some(s) = &sender {
                        s.send(Message::Status(jobid, status)).await.unwrap();
                    } else {
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

    pub fn get_agent_infos(&self) -> Vec<AgentStatusInfo> {
        self.known_hosts
            .lock()
            .unwrap()
            .iter()
            .map(|x| AgentStatusInfo { name: x.clone() })
            .collect()
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
