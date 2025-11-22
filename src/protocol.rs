use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRequest {
    pub addr: SocketAddr,
    pub tickers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidFormat,
    InvalidScheme,
    InvalidAddress,
    EmptyTickers,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ProtocolError::InvalidFormat => "invalid format",
            ProtocolError::InvalidScheme => "invalid scheme",
            ProtocolError::InvalidAddress => "invalid address",
            ProtocolError::EmptyTickers => "empty tickers",
        };
        write!(f, "{msg}")
    }
}

pub fn parse_command(input: &str) -> Result<StreamRequest, ProtocolError> {
    let trimmed = input.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(ProtocolError::InvalidFormat);
    }
    if !parts[0].eq_ignore_ascii_case("STREAM") {
        return Err(ProtocolError::InvalidFormat);
    }
    let target = parts[1];
    let ticker_list = parts[2];
    let addr_str = target
        .strip_prefix("udp://")
        .ok_or(ProtocolError::InvalidScheme)?;
    let mut addrs = addr_str
        .to_socket_addrs()
        .map_err(|_| ProtocolError::InvalidAddress)?;
    let addr = addrs.next().ok_or(ProtocolError::InvalidAddress)?;
    let tickers: Vec<String> = ticker_list
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_uppercase())
        .collect();
    if tickers.is_empty() {
        return Err(ProtocolError::EmptyTickers);
    }
    Ok(StreamRequest { addr, tickers })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stream_command() {
        let result = parse_command("STREAM udp://127.0.0.1:9999 AAPL,TSLA").unwrap();
        assert_eq!(result.tickers, vec!["AAPL".to_string(), "TSLA".to_string()]);
        assert_eq!(result.addr, "127.0.0.1:9999".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn rejects_invalid_scheme() {
        let err = parse_command("STREAM tcp://127.0.0.1:1 AAPL").unwrap_err();
        assert_eq!(err, ProtocolError::InvalidScheme);
    }
}
