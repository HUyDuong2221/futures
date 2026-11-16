use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiDecision {
    pub decision: String,
    pub market_regime: String,
    pub timeframe: String,
    pub entry_type: String,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub risk_reward_ratio: f64,
    pub reasoning: Vec<String>,
    pub invalidation: String,
    pub valid_for_minutes: u32,
}

impl AiDecision {
    pub fn is_valid(&self) -> bool {
        if self.reasoning.is_empty()
            || self.market_regime.is_empty()
            || self.timeframe.is_empty()
            || self.valid_for_minutes == 0
            || !self.entry_price.is_finite()
            || !self.stop_loss.is_finite()
            || !self.take_profit.is_finite()
            || !self.risk_reward_ratio.is_finite()
            || !matches!(self.entry_type.as_str(), "MARKET" | "LIMIT")
        {
            return false;
        }

        match self.decision.as_str() {
            "WAIT" => true,
            "LONG" => {
                self.entry_price > 0.0
                    && self.stop_loss > 0.0
                    && self.take_profit > self.entry_price
                    && self.stop_loss < self.entry_price
            }
            "SHORT" => {
                self.entry_price > 0.0
                    && self.take_profit > 0.0
                    && self.stop_loss > self.entry_price
                    && self.take_profit < self.entry_price
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AiDecision;

    fn decision(direction: &str, entry: f64, stop: f64, take_profit: f64) -> AiDecision {
        AiDecision {
            decision: direction.to_owned(),
            market_regime: "SIDEWAYS".to_owned(),
            timeframe: "15m".to_owned(),
            entry_type: "MARKET".to_owned(),
            entry_price: entry,
            stop_loss: stop,
            take_profit,
            risk_reward_ratio: 0.0,
            reasoning: vec!["Dữ liệu biến động chưa rõ".to_owned()],
            invalidation: "Không áp dụng".to_owned(),
            valid_for_minutes: 15,
        }
    }

    #[test]
    fn accepts_wait_without_trade_prices() {
        assert!(decision("WAIT", 0.0, 0.0, 0.0).is_valid());
    }

    #[test]
    fn rejects_long_with_stop_above_entry() {
        assert!(!decision("LONG", 100.0, 101.0, 110.0).is_valid());
    }
}
