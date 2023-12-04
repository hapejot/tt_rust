use std::sync::Arc;

use bytebuffer::ByteBuffer;
use futures_util::StreamExt;
use regex::Regex;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Url,
};
use serde::Deserialize;
use serde_derive::Deserialize;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, BufRead, Write},
    str::{from_utf8, FromStr},
};
use sysinfo::DiskExt;
use sysinfo::System;
use sysinfo::SystemExt;
use tokio::io::AsyncReadExt;
use tokio::{io::AsyncWriteExt, net::TcpListener, signal};
use tracing::info;
use tracing::{error, level_filters::LevelFilter};
use tt_rust::{
    agent::protocol::{remote_call, system, Coordinator, Message},
    init_tracing, TRACING,
};

#[derive(Deserialize, Debug)]
struct Request {
    headers: BTreeMap<String, String>,
    body: Option<Vec<u8>>,
    method: Option<String>,
    url: Option<String>,
    path: Option<String>,
}

pub struct Agent {
    coordinator: Arc<Coordinator>,
}

impl Agent {
    pub fn new(coordinator: Arc<Coordinator>) -> Self {
        Self { coordinator }
    }

    pub async fn run(&self, lstnr: TcpListener) {
        loop {
            match lstnr.accept().await {
                Ok((mut socket, _)) => {
                    let coord = self.coordinator.clone();
                    tokio::spawn(async move {
                        let mut bbuf = ByteBuffer::new();
                        let mut buf = [0; 1000];
                        let mut msg = Message::Empty;
                        loop {
                            let x = socket.read(&mut buf).await.unwrap();
                            if x > 0 {
                                bbuf.write_bytes(&buf[..x]);
                                if let Ok(m) = serde_xdr::from_bytes(bbuf.as_bytes()) {
                                    msg = m;
                                    break;
                                } else {
                                    info!("incomplete message. waiting for more data.");
                                }
                            }
                        }

                        match msg {
                            Message::RequireSpace(n) => {
                                info!("Required Space: {}", n);
                                find_system_with_space(&coord, n, &mut socket).await;
                            }
                            Message::RequireSpaceHere(n) => {
                                info!("required space here: {n}");
                                check_if_local_system_has_space(n, coord.clone(), &mut socket)
                                    .await;
                            }
                            Message::Load(fetch) => {
                                info!("load fetch {}", fetch.len());
                                load_fetch_request(&mut socket, coord.clone(), fetch).await;
                            }
                            Message::LoadHere(_, fetch) => {
                                let buf = serde_xdr::to_bytes(&Message::Empty).unwrap();
                                socket.write_all(&buf[..]).await.unwrap();
                                Agent::new(coord).load_here(fetch).await
                            }
                            _ => todo!("{:?}", msg),
                        }
                    });
                }
                Err(e) => error!("accept: {e}"),
            }
        }
    }

    async fn load(&self, fetch: String) {
        if let Some(req) = deserialize_fetch(fetch.clone()) {
            info!("loading request {:#?}", req);
            if let Some(size) = req.head().await {
                info!("needed space: {size}");
                let (_, host) = self.coordinator.require_space(size as usize).await;
                info!("found host {host}");
                let res = remote_call(
                    host.as_str(),
                    &Message::LoadHere(host.clone(), fetch.clone()),
                )
                .await;
                // println!("load -> {:?}", res);
            } else {
                info!("no size retrieved.");
            }
        } else {
            error!("no request!");
        }
    }

    async fn load_here(&self, fetch: String) {
        let fileid = generate_hash(&fetch);
        let req = deserialize_fetch(fetch).unwrap();
        if let Some(size) = req.head().await {
            let (host, _) = self.coordinator.require_space(size as usize).await;

            req.load_into_file(
                self.coordinator.clone(),
                format!("{:}.mp4", fileid).as_str(),
            )
            .await;
        }
    }
}

fn generate_hash(fetch: &String) -> String {
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(fetch.as_bytes());
    let result = hasher.finalize();
    let fileid = result.iter().map(|x| format!("{x:02x}")).collect();
    fileid
}

async fn load_fetch_request(
    socket: &mut tokio::net::TcpStream,
    coord: Arc<Coordinator>,
    fetch: String,
) {
    let buf = serde_xdr::to_bytes(&Message::Empty).unwrap();
    socket.write_all(&buf[..]).await.unwrap();
    info!("invoke agent to load");
    Agent::new(coord).load(fetch).await;
    info!("finished loading");
}

async fn check_if_local_system_has_space(
    n: usize,
    coord: Arc<Coordinator>,
    client_socket: &mut tokio::net::TcpStream,
) {
    let mut sys = System::new_all();
    sys.refresh_all();
    let mut msg = Message::Empty;
    for d in sys.disks() {
        if d.available_space() > n as u64 {
            msg = Message::HasSpace(coord.hostname(), d.available_space() as usize);
            break;
        }
    }
    let buf = serde_xdr::to_bytes(&msg).unwrap();
    client_socket.write_all(&buf[..]).await.unwrap();
}

async fn find_system_with_space(
    coord: &Arc<Coordinator>,
    n: usize,
    client_socket: &mut tokio::net::TcpStream,
) {
    let (size, host) = coord.require_space(n).await;

    let response = Message::HasSpace(host, size);
    let buf = serde_xdr::to_bytes(&response).unwrap();
    client_socket.write_all(&buf[..]).await.unwrap();
}

impl Request {
    pub async fn head(&self) -> Option<u64> {
        let clt = self.prepare_client();
        let url = self.url();

        let res = clt.head(url).send().await;
        match res {
            Ok(r) => {
                if r.status() == 200 {
                    r.content_length()
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub async fn load_into_file(&self, coord: Arc<Coordinator>, path: &str) {
        let clt = self.prepare_client();
        let url = self.url();

        let res = clt.get(url).send().await;
        match res {
            Ok(r) => {
                if r.status() == 200 {
                    let size = r.content_length().unwrap();
                    let part_size = size / 100;
                    let mut next_part = 0;
                    let mut part_count = 0;
                    let mut f = File::create(path).unwrap();
                    let mut st = r.bytes_stream();
                    let mut count: u64 = 0;
                    while let Some(c) = st.next().await {
                        match c {
                            Ok(chunk) => {
                                count += chunk.len() as u64;

                                if count >= next_part {
                                    coord
                                        .broadcast(&Message::Status(format!(
                                            "{} {}%",
                                            path, part_count
                                        )))
                                        .await;
                                    next_part += part_size;
                                    part_count += 1;
                                }
                                f.write_all(&chunk).unwrap();
                            }
                            Err(x) => {
                                error!("Network read failed: {}", x);
                                break;
                            }
                        }
                    }
                } else {
                    // println!("status. {}", r.status());
                }
            }
            Err(_) => panic!(),
        }
    }

    fn url(&self) -> Url {
        let url = Url::parse(self.url.as_ref().unwrap().as_str()).unwrap();
        url
    }

    fn prepare_client(&self) -> reqwest::Client {
        let bld = reqwest::Client::builder();
        let mut headers = HeaderMap::new();
        for (key, value) in self.headers.iter() {
            let n = HeaderName::from_str(key.as_str()).unwrap();
            if n == "range" {
                continue;
            }
            headers.append(n, HeaderValue::from_str(value.as_str()).unwrap());
        }
        let clt = bld
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.54 Safari/537.36 Edg/101.0.1210.39")
            .default_headers(headers)
            .connection_verbose(true)
            .brotli(false)
            .build().unwrap();
        clt
    }
}

fn deserialize_fetch(fetch: String) -> Option<Request> {
    let mut lines = fetch.lines();
    if let Some(l) = lines.next() {
        let pattern = Regex::new(r#"^fetch\("(.*)" *, *"#).unwrap();
        if let Some(c) = pattern.captures(l) {
            if let Some(url) = c.get(1) {
                let mut buf = String::from("{");
                let pattern = Regex::new(r#"\);$"#).unwrap();
                for line in lines {
                    let l = line;
                    if let Some(c) = pattern.captures(l) {
                        let x = c.get(0).unwrap();
                        buf.push_str(&l[0..x.start()]);
                        break;
                    }
                    buf.push_str(l);
                }

                let mut req: Request = serde_yaml::from_str(buf.as_str()).unwrap();
                req.url = Some(url.as_str().into());
                Some(req)
            } else {
                error!("no match  for fetch()");
                None
            }
        } else {
            error!("no captures at all.");
            None
        }
    } else {
        error!("no lines to fetch from.");
        None
    }
}

#[tokio::main]
async fn main() {
    init_tracing("agent", LevelFilter::INFO);
    // system();
    let c = Arc::new(Coordinator::new("0.0.0.0:7777").await);

    c.hello().await;
    let local_net = format!("{}:{}", "0.0.0.0", 7778);

    let agent = Agent::new(c.clone());

    // Bind the listener to the address
    match TcpListener::bind(&local_net).await {
        Ok(listener) => {
            let ctrl_c = signal::ctrl_c();
            tokio::select! {
                _ = c.run() => { error!("agent loop ended.");}
                _ = c.show_monitor() => { error!("monitor");}
                _ = agent.run(listener) => { error!("processing ended.");}
                _ = ctrl_c => { info!("shutting down on ctrl-c.");}
            }
        }
        Err(e) => error!("bind: {local_net} <- {e}"),
    };
}
