use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StockQuote {
    pub ticker: String,
    pub price: f64,
    pub volume: u32,
    pub timestamp: u64,
}

impl StockQuote {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_string(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_string() {
        let quote = StockQuote {
            ticker: "AAPL".to_string(),
            price: 150.25,
            volume: 1200,
            timestamp: 123456,
        };
        let encoded = quote.to_string();
        let decoded = StockQuote::from_string(&encoded).unwrap();
        assert_eq!(quote, decoded);
    }
}
