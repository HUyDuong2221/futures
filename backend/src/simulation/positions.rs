use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    pub direction: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub margin: f64,
    pub pnl: f64,
    pub status: String,
}

impl Position {
    pub fn mark_to_market(&mut self, current_price: f64) {
        self.current_price = current_price;
        let pnl_delta = if self.direction == "LONG" {
            current_price - self.entry_price
        } else {
            self.entry_price - current_price
        };
        self.pnl = pnl_delta * self.margin / self.entry_price.max(1.0);
    }
}
