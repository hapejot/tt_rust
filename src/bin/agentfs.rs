use clap::Parser;
use fuser::MountOption;
use tracing::level_filters::LevelFilter;
use tt_rust::{agent::fs::AgentFS, init_tracing};
#[derive(Parser)]
struct Args {
    mount_point: String,
}

fn main() {
    let args = Args::parse();
    init_tracing("agentfs", LevelFilter::INFO);

    let mut options = vec![MountOption::RW, MountOption::FSName("hello".to_string())];
    options.push(MountOption::AutoUnmount);
    options.push(MountOption::AllowRoot);
    fuser::mount2(AgentFS::new(), args.mount_point, &options).unwrap();
}