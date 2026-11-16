use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const INITIAL_BALANCE: f64 = 10_000.0;

#[derive(Debug, Clone, Deserialize)]
pub struct PaperOrderRequest {
    pub direction: String,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub margin_amount: f64,
    pub leverage: i32,
    pub max_loss_percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaperSettingsRequest {
    pub margin_per_order: f64,
    pub leverage: i32,
    pub max_loss_percent: f64,
    pub auto_trade_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperTradingSettings {
    pub margin_per_order: f64,
    pub leverage: i32,
    pub max_loss_percent: f64,
    pub auto_trade_enabled: bool,
    pub allow_multiple_positions: bool,
}

impl Default for PaperTradingSettings {
    fn default() -> Self {
        Self {
            margin_per_order: 100.0,
            leverage: 10,
            max_loss_percent: 5.0,
            auto_trade_enabled: false,
            allow_multiple_positions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperPosition {
    pub id: String,
    pub direction: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub max_loss_price: f64,
    pub quantity: f64,
    pub leverage: i32,
    pub margin: f64,
    pub pnl: f64,
    pub opened_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClosedPaperTrade {
    pub id: String,
    pub direction: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub pnl: f64,
    pub close_reason: String,
    pub closed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperAccountSummary {
    pub initial_balance: f64,
    pub balance: f64,
    pub equity: f64,
    pub available_balance: f64,
    pub used_margin: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub total_pnl: f64,
    pub margin_per_order: f64,
    pub leverage: i32,
    pub max_leverage: i32,
    pub max_loss_percent: f64,
    pub auto_trade_enabled: bool,
    pub allow_multiple_positions: bool,
}

#[derive(Debug)]
pub struct PaperSimulation {
    initial_balance: f64,
    realized_pnl: f64,
    positions: Vec<PaperPosition>,
    trades: Vec<ClosedPaperTrade>,
    settings: PaperTradingSettings,
    next_auto_scan_at: Option<DateTime<Utc>>,
    last_auto_signal_key: Option<String>,
}

impl Default for PaperSimulation {
    fn default() -> Self {
        Self {
            initial_balance: INITIAL_BALANCE,
            realized_pnl: 0.0,
            positions: Vec::new(),
            trades: Vec::new(),
            settings: PaperTradingSettings::default(),
            next_auto_scan_at: None,
            last_auto_signal_key: None,
        }
    }
}

impl PaperSimulation {
    pub fn summary(&self, max_leverage: i32) -> PaperAccountSummary {
        let unrealized_pnl = self
            .positions
            .iter()
            .map(|position| position.pnl)
            .sum::<f64>();
        let used_margin = self
            .positions
            .iter()
            .map(|position| position.margin)
            .sum::<f64>();
        let balance = self.initial_balance + self.realized_pnl;

        PaperAccountSummary {
            initial_balance: self.initial_balance,
            balance,
            equity: balance + unrealized_pnl,
            available_balance: (balance - used_margin).max(0.0),
            used_margin,
            unrealized_pnl,
            realized_pnl: self.realized_pnl,
            total_pnl: self.realized_pnl + unrealized_pnl,
            margin_per_order: self.settings.margin_per_order,
            leverage: self.settings.leverage,
            max_leverage,
            max_loss_percent: self.settings.max_loss_percent,
            auto_trade_enabled: self.settings.auto_trade_enabled,
            allow_multiple_positions: self.settings.allow_multiple_positions,
        }
    }

    pub fn settings(&self) -> &PaperTradingSettings {
        &self.settings
    }

    pub fn update_settings(
        &mut self,
        request: PaperSettingsRequest,
        max_leverage: i32,
    ) -> anyhow::Result<()> {
        if !request.margin_per_order.is_finite() || request.margin_per_order <= 0.0 {
            anyhow::bail!("Số tiền ký quỹ phải lớn hơn 0.");
        }
        if !(1..=max_leverage).contains(&request.leverage) {
            anyhow::bail!("Đòn bẩy phải nằm trong khoảng 1 đến {max_leverage}x.");
        }
        if !request.max_loss_percent.is_finite()
            || !(0.1..=100.0).contains(&request.max_loss_percent)
        {
            anyhow::bail!("Mức lỗ tối đa phải nằm trong khoảng 0,1% đến 100% ký quỹ.");
        }
        if request.margin_per_order > self.initial_balance {
            anyhow::bail!("Số tiền ký quỹ mỗi lệnh vượt quá vốn tài khoản mô phỏng.");
        }
        if request.auto_trade_enabled
            && request.margin_per_order > self.summary(max_leverage).available_balance
        {
            anyhow::bail!("Không đủ số dư khả dụng để bật AI tự vào lệnh với mức ký quỹ này.");
        }

        if request.auto_trade_enabled != self.settings.auto_trade_enabled {
            self.next_auto_scan_at = None;
        }
        self.settings.margin_per_order = request.margin_per_order;
        self.settings.leverage = request.leverage;
        self.settings.max_loss_percent = request.max_loss_percent;
        self.settings.auto_trade_enabled = request.auto_trade_enabled;
        Ok(())
    }

    pub fn auto_scan_due(&mut self, timeframe_seconds: i64) -> bool {
        if !self.settings.auto_trade_enabled {
            return false;
        }
        if timeframe_seconds <= 0 {
            return false;
        }
        let now = Utc::now();
        if self.next_auto_scan_at.is_some_and(|next| next > now) {
            return false;
        }
        let next_boundary = (now.timestamp().div_euclid(timeframe_seconds) + 1)
            * timeframe_seconds;
        self.next_auto_scan_at = DateTime::from_timestamp(next_boundary, 0);
        true
    }

    pub fn record_auto_signal(&mut self, signal_key: String) -> bool {
        if self.last_auto_signal_key.as_deref() == Some(signal_key.as_str()) {
            return false;
        }
        self.last_auto_signal_key = Some(signal_key);
        true
    }

    pub fn positions(&self) -> &[PaperPosition] {
        &self.positions
    }

    pub fn trades(&self) -> &[ClosedPaperTrade] {
        &self.trades
    }

    pub fn open_position(
        &mut self,
        request: PaperOrderRequest,
        entry_price: f64,
        max_leverage: i32,
    ) -> anyhow::Result<PaperPosition> {
        if !entry_price.is_finite() || entry_price <= 0.0 {
            anyhow::bail!("Giá thị trường hiện tại không hợp lệ.");
        }
        if !matches!(request.direction.as_str(), "LONG" | "SHORT") {
            anyhow::bail!("Chiều lệnh phải là LONG hoặc SHORT.");
        }
        if !request.stop_loss.is_finite()
            || !request.take_profit.is_finite()
            || !request.margin_amount.is_finite()
            || request.stop_loss <= 0.0
            || request.take_profit <= 0.0
            || request.margin_amount <= 0.0
        {
            anyhow::bail!("Ký quỹ, giá cắt lỗ và chốt lời phải lớn hơn 0.");
        }

        let levels_valid = match request.direction.as_str() {
            "LONG" => request.stop_loss < entry_price && request.take_profit > entry_price,
            "SHORT" => request.take_profit < entry_price && request.stop_loss > entry_price,
            _ => false,
        };
        if !levels_valid {
            anyhow::bail!("Mức cắt lỗ/chốt lời không đúng chiều so với giá vào hiện tại.");
        }

        if !(1..=max_leverage).contains(&request.leverage) {
            anyhow::bail!("Đòn bẩy phải nằm trong khoảng 1 đến {max_leverage}x.");
        }
        if !request.max_loss_percent.is_finite()
            || !(0.1..=100.0).contains(&request.max_loss_percent)
        {
            anyhow::bail!("Mức lỗ tối đa phải nằm trong khoảng 0,1% đến 100% ký quỹ.");
        }

        let margin = request.margin_amount;
        let available_balance = self.summary(max_leverage).available_balance;
        if margin > available_balance {
            anyhow::bail!("Không đủ số dư khả dụng cho số tiền ký quỹ đã nhập.");
        }
        let quantity = margin * f64::from(request.leverage) / entry_price;
        if !quantity.is_finite() || quantity <= 0.0 {
            anyhow::bail!("Không thể tính khối lượng vị thế từ số tiền và đòn bẩy.");
        }
        let max_loss_price_delta = margin * request.max_loss_percent / 100.0 / quantity;
        let max_loss_price = match request.direction.as_str() {
            "LONG" => entry_price - max_loss_price_delta,
            "SHORT" => entry_price + max_loss_price_delta,
            _ => unreachable!(),
        };
        let stop_loss = match request.direction.as_str() {
            "LONG" => request.stop_loss.max(max_loss_price),
            "SHORT" => request.stop_loss.min(max_loss_price),
            _ => unreachable!(),
        };

        let position = PaperPosition {
            id: Uuid::new_v4().to_string(),
            direction: request.direction,
            entry_price,
            current_price: entry_price,
            stop_loss,
            take_profit: request.take_profit,
            max_loss_price,
            quantity,
            leverage: request.leverage,
            margin,
            pnl: 0.0,
            opened_at: Utc::now().to_rfc3339(),
        };
        self.positions.push(position.clone());
        Ok(position)
    }

    pub fn update_price(&mut self, price: f64) {
        let mut remaining = Vec::with_capacity(self.positions.len());
        for mut position in self.positions.drain(..) {
            let close = match position.direction.as_str() {
                "LONG" if price <= position.stop_loss => Some((
                    position.stop_loss,
                    if position.stop_loss == position.max_loss_price {
                        "MAX_LOSS"
                    } else {
                        "STOP_LOSS"
                    },
                )),
                "LONG" if price >= position.take_profit => {
                    Some((position.take_profit, "TAKE_PROFIT"))
                }
                "SHORT" if price >= position.stop_loss => Some((
                    position.stop_loss,
                    if position.stop_loss == position.max_loss_price {
                        "MAX_LOSS"
                    } else {
                        "STOP_LOSS"
                    },
                )),
                "SHORT" if price <= position.take_profit => {
                    Some((position.take_profit, "TAKE_PROFIT"))
                }
                _ => None,
            };

            if let Some((exit_price, reason)) = close {
                let pnl = calculate_pnl(&position, exit_price);
                self.realized_pnl += pnl;
                self.trades.push(ClosedPaperTrade {
                    id: position.id,
                    direction: position.direction,
                    entry_price: position.entry_price,
                    exit_price,
                    quantity: position.quantity,
                    pnl,
                    close_reason: reason.to_owned(),
                    closed_at: Utc::now().to_rfc3339(),
                });
            } else {
                position.current_price = price;
                position.pnl = calculate_pnl(&position, price);
                remaining.push(position);
            }
        }
        self.positions = remaining;
    }

    pub fn close_position(
        &mut self,
        id: &str,
        exit_price: f64,
    ) -> anyhow::Result<ClosedPaperTrade> {
        if !exit_price.is_finite() || exit_price <= 0.0 {
            anyhow::bail!("Giá đóng vị thế không hợp lệ.");
        }
        let index = self
            .positions
            .iter()
            .position(|position| position.id == id)
            .ok_or_else(|| anyhow::anyhow!("Không tìm thấy vị thế đang mở."))?;
        let position = self.positions.remove(index);
        let pnl = calculate_pnl(&position, exit_price);
        self.realized_pnl += pnl;
        let trade = ClosedPaperTrade {
            id: position.id,
            direction: position.direction,
            entry_price: position.entry_price,
            exit_price,
            quantity: position.quantity,
            pnl,
            close_reason: "MANUAL".to_owned(),
            closed_at: Utc::now().to_rfc3339(),
        };
        self.trades.push(trade.clone());
        Ok(trade)
    }
}

fn calculate_pnl(position: &PaperPosition, exit_price: f64) -> f64 {
    let price_delta = match position.direction.as_str() {
        "LONG" => exit_price - position.entry_price,
        _ => position.entry_price - exit_price,
    };
    price_delta * position.quantity
}

#[cfg(test)]
mod tests {
    use super::{PaperOrderRequest, PaperSettingsRequest, PaperSimulation};

    fn order(direction: &str, stop_loss: f64, take_profit: f64) -> PaperOrderRequest {
        PaperOrderRequest {
            direction: direction.to_owned(),
            stop_loss,
            take_profit,
            margin_amount: 100.0,
            leverage: 10,
            max_loss_percent: 5.0,
        }
    }

    #[test]
    fn opens_risk_sized_position_and_realizes_take_profit() {
        let mut simulation = PaperSimulation::default();
        let position = simulation
            .open_position(order("LONG", 90.0, 120.0), 100.0, 20)
            .unwrap();

        assert_eq!(position.quantity, 10.0);
        simulation.update_price(121.0);

        assert!(simulation.positions().is_empty());
        assert_eq!(simulation.trades()[0].close_reason, "TAKE_PROFIT");
        assert_eq!(simulation.summary(20).realized_pnl, 200.0);
    }

    #[test]
    fn closes_position_at_max_loss_percentage_of_margin() {
        let mut simulation = PaperSimulation::default();
        let position = simulation
            .open_position(order("LONG", 90.0, 120.0), 100.0, 20)
            .unwrap();

        assert_eq!(position.stop_loss, 99.5);
        simulation.update_price(99.0);

        assert!(simulation.positions().is_empty());
        assert_eq!(simulation.trades()[0].close_reason, "MAX_LOSS");
        assert_eq!(simulation.trades()[0].pnl, -5.0);
    }

    #[test]
    fn rejects_stop_loss_on_wrong_side() {
        let mut simulation = PaperSimulation::default();
        assert!(
            simulation
                .open_position(order("LONG", 101.0, 120.0), 100.0, 20)
                .is_err()
        );
    }

    #[test]
    fn automatic_settings_scan_immediately_and_deduplicate_same_candle_signal() {
        let mut simulation = PaperSimulation::default();
        simulation
            .update_settings(
                PaperSettingsRequest {
                    margin_per_order: 250.0,
                    leverage: 15,
                    max_loss_percent: 5.0,
                    auto_trade_enabled: true,
                },
                20,
            )
            .unwrap();

        assert!(simulation.auto_scan_due(15 * 60));
        assert!(!simulation.auto_scan_due(15 * 60));
        assert!(simulation.record_auto_signal("15m:123:LONG".to_owned()));
        assert!(!simulation.record_auto_signal("15m:123:LONG".to_owned()));
        assert_eq!(simulation.settings().margin_per_order, 250.0);
        assert_eq!(simulation.settings().leverage, 15);
        assert_eq!(simulation.settings().max_loss_percent, 5.0);
    }
}
