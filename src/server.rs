use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use log::{error, info, warn};

use crate::generator::QuoteGenerator;
use crate::protocol::{StreamRequest, parse_command};
use crate::quote::StockQuote;

const GENERATE_INTERVAL: Duration = Duration::from_millis(200);
const STREAM_TIMEOUT: Duration = Duration::from_secs(5);
const DISPATCH_TIMEOUT: Duration = Duration::from_millis(200);
const UDP_BIND_ADDR: &str = "0.0.0.0:0";
const PING_WORD: &str = "ping";
const PING_REPLY: &[u8] = b"Pong";

struct ClientEntry {
    filter: HashSet<String>,
    tx: Sender<StockQuote>,
}

pub fn run_server(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    let (quote_tx, quote_rx) = unbounded();
    let registry: Arc<Mutex<Vec<ClientEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let _gen = spawn_generator(quote_tx);
    let _dispatcher = spawn_dispatcher(quote_rx, registry.clone());
    info!("listening on {addr}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let registry_clone = registry.clone();
                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, registry_clone) {
                        error!("client error: {err}");
                    }
                });
            }
            Err(err) => error!("accept error: {err}"),
        }
    }
    Ok(())
}

fn spawn_generator(tx: Sender<StockQuote>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut generator = QuoteGenerator::default();
        loop {
            let batch = generator.generate_all();
            for quote in batch {
                let _ = tx.send(quote);
            }
            thread::sleep(GENERATE_INTERVAL);
        }
    })
}

fn spawn_dispatcher(
    rx: Receiver<StockQuote>,
    registry: Arc<Mutex<Vec<ClientEntry>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(quote) = rx.recv() {
            if let Ok(mut guard) = registry.lock() {
                let mut index = 0;
                while index < guard.len() {
                    let needs = guard[index].filter.contains(&quote.ticker);
                    let keep = if needs {
                        guard[index].tx.send(quote.clone()).is_ok()
                    } else {
                        true
                    };
                    if keep {
                        index += 1;
                    } else {
                        guard.remove(index);
                    }
                }
            }
        }
    })
}

fn handle_connection(
    stream: TcpStream,
    registry: Arc<Mutex<Vec<ClientEntry>>>,
) -> std::io::Result<()> {
    if let Ok(addr) = stream.peer_addr() {
        info!("tcp connect {addr}");
    }
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut buffer = String::new();
    reader.read_line(&mut buffer)?;
    if buffer.trim().is_empty() {
        return Ok(());
    }
    match parse_command(&buffer) {
        Ok(request) => {
            let (tx, rx) = unbounded();
            let filter = request
                .tickers
                .iter()
                .map(|t| t.to_uppercase())
                .collect::<HashSet<_>>();
            if let Ok(mut guard) = registry.lock() {
                guard.push(ClientEntry { filter, tx });
            }
            let mut writer = stream;
            let _ = writer.write_all(b"OK\n");
            let _ = writer.flush();
            info!("stream start {}", request.addr);
            thread::spawn(move || stream_quotes(request, rx));
        }
        Err(err) => {
            warn!("command parse error: {err}");
            let mut writer = stream;
            let message = format!("ERR {err}\n");
            let _ = writer.write_all(message.as_bytes());
            let _ = writer.flush();
        }
    }
    Ok(())
}

fn stream_quotes(request: StreamRequest, rx: Receiver<StockQuote>) {
    let socket = match UdpSocket::bind(UDP_BIND_ADDR) {
        Ok(s) => s,
        Err(err) => {
            error!("udp bind error: {err}");
            return;
        }
    };
    let _ = socket.set_nonblocking(true);
    let mut last_ping = Instant::now();
    let mut buf = [0u8; 256];
    loop {
        if last_ping.elapsed() > STREAM_TIMEOUT {
            break;
        }
        match socket.recv_from(&mut buf) {
            Ok((n, src)) => {
                if let Ok(msg) = std::str::from_utf8(&buf[..n]) {
                    if msg.trim().eq_ignore_ascii_case(PING_WORD) {
                        last_ping = Instant::now();
                        let _ = socket.send_to(PING_REPLY, src);
                    }
                }
            }
            Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
        match rx.recv_timeout(DISPATCH_TIMEOUT) {
            Ok(quote) => {
                let payload = quote.to_string();
                let _ = socket.send_to(payload.as_bytes(), request.addr);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }
    info!("stream stop {}", request.addr);
}
