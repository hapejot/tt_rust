#[cfg(unix)]
mod param {
    use clap::Parser;
    #[derive(Parser)]
    pub struct Args {
        pub mount_point: String,
    }
}
#[cfg(unix)]
fn main() {
    use clap::Parser;
    use fuser::MountOption;
    use tracing::level_filters::LevelFilter;
    use tt_rust::{agent::fs::AgentFS, init_tracing};

    let args = param::Args::parse();
    init_tracing("agentfs", LevelFilter::DEBUG);

    let mut options = vec![MountOption::RW, MountOption::FSName("agent".to_string())];
    options.push(MountOption::AutoUnmount);
    options.push(MountOption::AllowRoot);
    fuser::mount2(AgentFS::new(), args.mount_point, &options).unwrap();
}

#[cfg(windows)]
fn main() {
    println!("user fs is not available on windows.")
}
