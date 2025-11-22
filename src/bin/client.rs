use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;
use std::time::Duration;

use clap::Parser;
use log::{error, info, warn};
use quote_server::StockQuote;
use quote_server::client::{bind_udp, build_command, load_tickers, send_command};

const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:7878";
const DEFAULT_UDP_HOST: &str = "127.0.0.1";
const DEFAULT_UDP_PORT: u16 = 34254;
const PING_INTERVAL: Duration = Duration::from_secs(2);
const SRC_WAIT: Duration = Duration::from_millis(500);
const RECV_BUF: usize = 1024;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long = "server-addr", alias = "server", default_value = DEFAULT_SERVER_ADDR)]
    server_addr: String,
    #[arg(long = "udp-host", default_value = DEFAULT_UDP_HOST)]
    udp_host: String,
    #[arg(long = "udp-port", default_value_t = DEFAULT_UDP_PORT)]
    udp_port: u16,
    #[arg(long = "tickers-file", alias = "tickers")]
    tickers_file: std::path::PathBuf,
}

fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let tickers = load_tickers(&args.tickers_file).map_err(io_error)?;
    let command = build_command(&args.udp_host, args.udp_port, &tickers).map_err(io_error)?;
    let response = send_command(&args.server_addr, &command)?;
    match response {
        Some(resp) if resp.starts_with("ERR") => {
            error!("{resp}");
            return Ok(());
        }
        Some(resp) => info!("{resp}"),
        None => warn!("no response from server"),
    }
    let socket = bind_udp(args.udp_port)?;
    let running = Arc::new(AtomicBool::new(true));
    let (src_tx, src_rx) = mpsc::channel::<SocketAddr>();
    let ping_socket = socket.try_clone()?;
    let ping_running = running.clone();
    let ping_handle = thread::spawn(move || ping_loop(ping_socket, ping_running, src_rx));
    let filter = tickers.into_iter().collect::<HashSet<_>>();
    ctrlc::set_handler({
        let running = running.clone();
        move || {
            running.store(false, Ordering::SeqCst);
        }
    })
    .ok();
    recv_loop(socket, running.clone(), src_tx, filter);
    running.store(false, Ordering::SeqCst);
    let _ = ping_handle.join();
    Ok(())
}

fn recv_loop(
    socket: std::net::UdpSocket,
    running: Arc<AtomicBool>,
    src_tx: mpsc::Sender<SocketAddr>,
    filter: HashSet<String>,
) {
    let mut buf = [0u8; RECV_BUF];
    while running.load(Ordering::SeqCst) {
        match socket.recv_from(&mut buf) {
            Ok((n, src)) => {
                let _ = src_tx.send(src);
                if let Ok(msg) = std::str::from_utf8(&buf[..n]) {
                    if let Some(quote) = StockQuote::from_string(msg) {
                        if filter.contains(&quote.ticker) {
                            println!(
                                "{} price={:.2} volume={} ts={}",
                                quote.ticker, quote.price, quote.volume, quote.timestamp
                            );
                        } else {
                            warn!("filtered {}", quote.ticker);
                        }
                    } else {
                        warn!("unparsed payload: {msg}");
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                error!("udp receive error: {e}");
                break;
            }
        }
    }
}

fn ping_loop(
    socket: std::net::UdpSocket,
    running: Arc<AtomicBool>,
    src_rx: mpsc::Receiver<SocketAddr>,
) {
    let mut target: Option<SocketAddr> = None;
    while running.load(Ordering::SeqCst) {
        if target.is_none() {
            match src_rx.recv_timeout(SRC_WAIT) {
                Ok(addr) => target = Some(addr),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            }
        }
        if let Ok(new_addr) = src_rx.try_recv() {
            target = Some(new_addr);
        }
        if let Some(addr) = target {
            let _ = socket.send_to(b"Ping", addr);
        }
        thread::sleep(PING_INTERVAL);
    }
}

fn io_error<T: ToString>(msg: T) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, msg.to_string())
}
