use std::{path::Path, sync::Mutex};

use once_cell::sync::Lazy;
use tracing::info;

pub mod controls;
pub mod data;
pub mod dbx;
pub mod error;
pub mod parser;
pub mod runtime;
pub mod tsort;
pub mod ui;

pub static TRACING: Lazy<bool> = Lazy::new(init_tracing);

pub fn init_tracing() -> bool {
    use tracing_subscriber::filter::LevelFilter;
    let n = format!("test-{}.log", chrono::Utc::now());
    let path = Path::new(&n);
    let log_file = std::fs::File::create(path).unwrap();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
    true
}
