use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMarketTick {
    pub timestamp: String,
    pub instrument: String,
    pub price: f64,
    pub volume: f64,
    pub side: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub instrument: String,
    pub timestamp: String,
    pub last_price: f64,
    pub mark_price: f64,
    pub volume: f64,
    pub funding_rate: f64,
}
