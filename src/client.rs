use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, UdpSocket};
use std::path::Path;
use std::time::Duration;

use crate::protocol::parse_command;

const TCP_READ_TIMEOUT: Duration = Duration::from_secs(3);
const UDP_READ_TIMEOUT: Duration = Duration::from_millis(500);
const UDP_BIND_ADDR: &str = "0.0.0.0";

pub fn load_tickers(path: &Path) -> Result<Vec<String>, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let tickers: Vec<String> = data
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_uppercase())
        .collect();
    if tickers.is_empty() {
        return Err("no tickers found".to_string());
    }
    Ok(tickers)
}

pub fn build_command(host: &str, port: u16, tickers: &[String]) -> Result<String, String> {
    if tickers.is_empty() {
        return Err("no tickers to request".to_string());
    }
    let joined = tickers.join(",");
    let command = format!("STREAM udp://{}:{} {}", host, port, joined);
    parse_command(&command).map_err(|e| e.to_string())?;
    Ok(command)
}

pub fn send_command(server: &str, command: &str) -> std::io::Result<Option<String>> {
    let mut stream = TcpStream::connect(server)?;
    let payload = format!("{command}\n");
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;
    stream.set_read_timeout(Some(TCP_READ_TIMEOUT)).ok();
    let mut buf = String::new();
    let mut reader = BufReader::new(stream);
    match reader.read_line(&mut buf) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(buf.trim().to_string())),
        Err(e) => Err(e),
    }
}

pub fn bind_udp(port: u16) -> std::io::Result<UdpSocket> {
    let socket = UdpSocket::bind((UDP_BIND_ADDR, port))?;
    socket.set_read_timeout(Some(UDP_READ_TIMEOUT))?;
    socket.set_nonblocking(false)?;
    Ok(socket)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    #[test]
    fn loads_tickers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tickers.txt");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "AAPL\n tsla \n").unwrap();
        let tickers = load_tickers(&path).unwrap();
        assert_eq!(tickers, vec!["AAPL".to_string(), "TSLA".to_string()]);
    }

    #[test]
    fn builds_valid_command() {
        let tickers = vec!["AAPL".to_string(), "TSLA".to_string()];
        let cmd = build_command("127.0.0.1", 4000, &tickers).unwrap();
        assert_eq!(cmd, "STREAM udp://127.0.0.1:4000 AAPL,TSLA");
    }

    #[test]
    fn binds_udp_socket() {
        let socket = bind_udp(0).unwrap();
        let addr = socket.local_addr().unwrap();
        assert!(addr.port() > 0);
    }

    #[test]
    fn sends_command_and_reads_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 64];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(b"OK\n");
            }
        });
        let result = send_command(&addr.to_string(), "STREAM udp://127.0.0.1:1234 AAPL");
        assert_eq!(result.unwrap(), Some("OK".to_string()));
    }
}
