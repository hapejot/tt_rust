use std::io::{read_to_string, stdin};

use clap::{Parser, Subcommand};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::{info, level_filters::LevelFilter};
use tt_rust::{agent::protocol::Message, init_tracing};

#[derive(Parser)]
#[command(author, version, about, long_about = Some("Client for talking to a local file agent."))]
struct Params {
    /// Commands for the client
    #[command(subcommand)]
    cmd: Commands,

    #[arg(short, long, default_value_t = LevelFilter::INFO)]
    /// Define the log level of the client
    trace_level: LevelFilter,
}

#[derive(Subcommand)]
enum Commands {
    /// test if the agent is running.
    Test,

    /// reading a request from stdin.
    /// the request has the form of a "fetch" you get from Chrome when copying a request as "fetch"
    Load,

    /// Show available files for all connected agents
    List,
}

// fetch("https://www.imis.bfs.de/ogc/opendata/ows", {
//     "headers": {
//       "accept": "*/*",
//       "accept-language": "de,de-DE;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6",
//       "content-type": "text/plain;charset=UTF-8",
//       "sec-ch-ua": "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Microsoft Edge\";v=\"120\"",
//       "sec-ch-ua-mobile": "?0",
//       "sec-ch-ua-platform": "\"Linux\"",
//       "sec-fetch-dest": "empty",
//       "sec-fetch-mode": "cors",
//       "sec-fetch-site": "same-site"
//     },
//     "referrerPolicy": "same-origin",
//     "body": "<GetFeature xmlns=\"http://www.opengis.net/wfs\" service=\"WFS\" version=\"1.1.0\" outputFormat=\"application/json\" viewParams=\"_dc:1703519430853;\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://www.opengis.net/wfs http://schemas.opengis.net/wfs/1.1.0/wfs.xsd\"><Query typeName=\"odlinfo_odl_1h_latest\"/></GetFeature>",
//     "method": "POST",
//     "mode": "cors",
//     "credentials": "omit"
//   });

struct ClientError {
    msg: String,
}

struct Client {
    socket: TcpStream,
}

impl Client {
    pub async fn new() -> Result<Self, ClientError> {
        match TcpStream::connect("localhost:7778").await {
            Ok(socket) => {
                info!("connected to local agent");
                Ok(Self { socket })
            }
            Err(e) => Err(ClientError {
                msg: format!("Unable to connect to local agent: {}", e),
            }),
        }
    }
}

impl Client {
    async fn process_request(&mut self, msg: Message) -> Message {
        let buf = serde_xdr::to_bytes(&msg).unwrap();
        self.socket.write_all(&buf[..]).await.unwrap();

        let mut buf = [0; 30000];
        let n = self.socket.read(&mut buf).await.unwrap();
        assert!(n > 0);
        let msg: Message = serde_xdr::from_bytes(&buf[..n]).unwrap();
        println!("-> {:?}", msg);
        msg
    }
}
#[tokio::main]
async fn main() {
    let params = Params::parse();
    init_tracing("client", LevelFilter::INFO);
    match Client::new().await {
        Ok(mut clt) => match params.cmd {
            Commands::Test => {
                println!("successfully connected to agent.");
            }
            Commands::Load => {
                let s = read_to_string(stdin()).unwrap();

                let _ = clt.process_request(Message::Load(s)).await;
            }
            Commands::List => {
                let _ = clt.process_request(Message::List(0)).await;
            }
            // _ => todo!(),
        },
        Err(e) => println!("{}", e.msg),
    }
}
