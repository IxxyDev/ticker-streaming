mod server;

use crate::server::run_server;
use log::error;

const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:7878";

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    if let Err(err) = run_server(DEFAULT_SERVER_ADDR) {
        error!("server error: {err}");
    }
}
