use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub direction: String,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub leverage: i32,
    pub capital: f64,
    pub risk_per_trade: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperOrder {
    pub id: String,
    pub direction: String,
    pub status: String,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub leverage: i32,
    pub filled: bool,
    pub fill_price: f64,
    pub created_at: String,
}

impl OrderRequest {
    pub fn validate(&self) -> bool {
        self.capital > 0.0
            && self.entry_price > 0.0
            && self.stop_loss > 0.0
            && self.take_profit > 0.0
            && matches!(self.direction.as_str(), "LONG" | "SHORT")
    }
}
