use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;
use rand::rngs::ThreadRng;

use crate::quote::StockQuote;

const START_PRICE_MIN: f64 = 50.0;
const START_PRICE_MAX: f64 = 500.0;
const MIN_PRICE: f64 = 1.0;
const DRIFT_RANGE: f64 = 2.5;
const ROUND_FACTOR: f64 = 100.0;
const POPULAR_VOLUME_BASE: u32 = 1000;
const POPULAR_VOLUME_SPREAD: f64 = 5000.0;
const REGULAR_VOLUME_BASE: u32 = 100;
const REGULAR_VOLUME_SPREAD: f64 = 1000.0;

pub const DEFAULT_TICKERS: &[&str] = &[
    "AAPL", "MSFT", "GOOGL", "AMZN", "NVDA", "META", "TSLA", "JPM", "JNJ", "V", "PG", "UNH", "HD",
    "DIS", "PYPL", "NFLX", "ADBE", "CRM", "INTC", "CSCO", "PFE", "ABT", "TMO", "ABBV", "LLY",
    "PEP", "COST", "TXN", "AVGO", "ACN", "QCOM", "DHR", "MDT", "NKE", "UPS", "RTX", "HON", "ORCL",
    "LIN", "AMGN", "LOW", "SBUX", "SPGI", "INTU", "ISRG", "T", "BMY", "DE", "PLD", "CI", "CAT",
    "GS", "UNP", "AMT", "AXP", "MS", "BLK", "GE", "SYK", "GILD", "MMM", "MO", "LMT", "FISV", "ADI",
    "BKNG", "C", "SO", "NEE", "ZTS", "TGT", "DUK", "ICE", "BDX", "PNC", "CMCSA", "SCHW", "MDLZ",
    "TJX", "USB", "CL", "EMR", "APD", "COF", "FDX", "AON", "WM", "ECL", "ITW", "VRTX", "D", "NSC",
    "PGR", "ETN", "FIS", "PSA", "KLAC", "MCD", "ADP", "APTV", "AEP", "MCO", "SHW", "DD", "ROP",
    "SLB", "HUM", "BSX", "NOC", "EW",
];

pub struct QuoteGenerator {
    prices: HashMap<String, f64>,
    rng: ThreadRng,
}

impl QuoteGenerator {
    pub fn new<T: IntoIterator<Item = String>>(tickers: T) -> Self {
        let mut rng = rand::thread_rng();
        let mut prices = HashMap::new();
        for ticker in tickers {
            let price = rng.gen_range(START_PRICE_MIN..START_PRICE_MAX);
            prices.insert(ticker.to_uppercase(), price);
        }
        QuoteGenerator { prices, rng }
    }

    pub fn default() -> Self {
        let tickers = DEFAULT_TICKERS
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>();
        QuoteGenerator::new(tickers)
    }

    pub fn generate_all(&mut self) -> Vec<StockQuote> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let mut quotes = Vec::with_capacity(self.prices.len());
        for (ticker, price) in self.prices.iter_mut() {
            let drift = self.rng.gen_range(-DRIFT_RANGE..DRIFT_RANGE);
            let updated = (*price + drift).max(MIN_PRICE);
            *price = updated;
            let volume = match ticker.as_str() {
                "AAPL" | "MSFT" | "TSLA" => {
                    POPULAR_VOLUME_BASE + self.rng.gen_range(0.0..POPULAR_VOLUME_SPREAD) as u32
                }
                _ => REGULAR_VOLUME_BASE + self.rng.gen_range(0.0..REGULAR_VOLUME_SPREAD) as u32,
            };
            let rounded = (updated * ROUND_FACTOR).round() / ROUND_FACTOR;
            quotes.push(StockQuote {
                ticker: ticker.clone(),
                price: rounded,
                volume,
                timestamp: now,
            });
        }
        quotes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_quotes() {
        let mut generator = QuoteGenerator::default();
        let quotes = generator.generate_all();
        assert!(!quotes.is_empty());
        assert!(quotes.iter().all(|q| q.price > 0.0 && q.volume > 0));
    }
}
