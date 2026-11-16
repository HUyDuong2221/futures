use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub initial_balance: f64,
    pub available_balance: f64,
    pub used_margin: f64,
    pub equity: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub total_fees: f64,
    pub total_funding: f64,
    pub leverage: i32,
    pub risk_per_trade: f64,
    pub paper_trading_enabled: bool,
}

impl Account {
    pub fn new(id: &str, initial_balance: f64, leverage: i32, risk_per_trade: f64) -> Self {
        let balance = initial_balance;
        Self {
            id: id.to_string(),
            initial_balance: balance,
            available_balance: balance,
            used_margin: 0.0,
            equity: balance,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            total_fees: 0.0,
            total_funding: 0.0,
            leverage,
            risk_per_trade,
            paper_trading_enabled: true,
        }
    }

    pub fn can_allocate_margin(&self, required_margin: f64) -> bool {
        self.available_balance >= required_margin
    }
}
