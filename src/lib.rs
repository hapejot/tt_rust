use std::sync::Mutex;

use tracing::info;

pub mod parser;
pub mod runtime;
pub mod controls;
pub mod dbx;
pub mod data;
pub mod tsort;
pub mod error;
pub mod ui;


pub fn init_tracing(name: &str) {
    use tracing_subscriber::filter::LevelFilter;
    let log_file = std::fs::File::create(format!("{name}.log")).unwrap();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    info!("starting");

}