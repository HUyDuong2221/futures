use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndicatorSnapshot {
    pub ema_20: f64,
    pub ema_50: f64,
    pub ema_200: f64,
    pub rsi_14: Option<f64>,
    pub macd: Option<f64>,
    pub signal: Option<f64>,
    pub atr_14: Option<f64>,
    pub bollinger_upper: Option<f64>,
    pub bollinger_middle: Option<f64>,
    pub bollinger_lower: Option<f64>,
    pub volume_ma: Option<f64>,
    pub relative_volume: Option<f64>,
    pub support: Option<f64>,
    pub resistance: Option<f64>,
    pub market_structure: String,
    pub regime: String,
    pub data_version: u64,
}

pub fn calculate_ema(values: &[f64], period: usize) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut prev = values[0];
    let multiplier = 2.0 / (period as f64 + 1.0);
    for value in values.iter().skip(1) {
        prev = (value - prev) * multiplier + prev;
    }
    prev
}

pub fn calculate_rsi(values: &[f64], period: usize) -> Option<f64> {
    if values.len() < period + 1 {
        return None;
    }
    let mut gains = 0.0;
    let mut losses = 0.0;
    for window in values.windows(2).rev().take(period) {
        let delta = window[1] - window[0];
        if delta >= 0.0 {
            gains += delta;
        } else {
            losses -= delta;
        }
    }
    if gains + losses == 0.0 {
        return Some(50.0);
    }
    let rs = gains / losses.max(1e-12);
    Some(100.0 - (100.0 / (1.0 + rs)))
}

pub fn calculate_macd(values: &[f64]) -> (f64, f64) {
    if values.len() < 2 {
        return (0.0, 0.0);
    }
    let macd_line: Vec<f64> = (0..values.len())
        .map(|index| {
            let history = &values[..=index];
            calculate_ema(history, 12) - calculate_ema(history, 26)
        })
        .collect();
    (
        *macd_line.last().unwrap_or(&0.0),
        calculate_ema(&macd_line, 9),
    )
}

pub fn calculate_atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Option<f64> {
    if high.len() < period || low.len() < period || close.len() < period {
        return None;
    }
    let mut true_ranges = Vec::new();
    for i in 0..high.len().min(low.len().min(close.len())) {
        let prev_close = if i == 0 { close[i] } else { close[i - 1] };
        let tr = ((high[i] - low[i]).abs())
            .max((high[i] - prev_close).abs())
            .max((low[i] - prev_close).abs());
        true_ranges.push(tr);
    }
    if true_ranges.is_empty() {
        return None;
    }
    let average = true_ranges.iter().rev().take(period).sum::<f64>() / period as f64;
    Some(average)
}

pub fn compute_bollinger(values: &[f64], period: usize, stdev: f64) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let window = if values.len() < period {
        values.len()
    } else {
        period
    };
    let slice = &values[values.len() - window..];
    let mean = slice.iter().sum::<f64>() / slice.len() as f64;
    let variance = slice.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / slice.len() as f64;
    let sigma = variance.sqrt() * stdev;
    (mean + sigma, mean, mean - sigma)
}

pub fn calculate_support_resistance(values: &[f64]) -> (Option<f64>, Option<f64>) {
    if values.is_empty() {
        return (None, None);
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (Some(min), Some(max))
}

pub fn build_snapshot(price_series: &[f64]) -> IndicatorSnapshot {
    let ema_20 = calculate_ema(price_series, 20);
    let ema_50 = calculate_ema(price_series, 50);
    let ema_200 = calculate_ema(price_series, 200);
    let rsi = calculate_rsi(price_series, 14);
    let (macd, signal) = calculate_macd(price_series);
    let (bollinger_upper, bollinger_middle, bollinger_lower) =
        compute_bollinger(price_series, 20, 2.0);
    let (support, resistance) = calculate_support_resistance(price_series);
    let regime = if ema_20 > ema_50 && ema_50 > ema_200 {
        "Bullish Trending".to_owned()
    } else if ema_20 < ema_50 && ema_50 < ema_200 {
        "Bearish Trending".to_owned()
    } else {
        "Sideways".to_owned()
    };

    IndicatorSnapshot {
        ema_20,
        ema_50,
        ema_200,
        rsi_14: rsi,
        macd: Some(macd),
        signal: Some(signal),
        atr_14: None,
        bollinger_upper: Some(bollinger_upper),
        bollinger_middle: Some(bollinger_middle),
        bollinger_lower: Some(bollinger_lower),
        volume_ma: None,
        relative_volume: None,
        support,
        resistance,
        market_structure: "Unavailable".to_owned(),
        regime,
        data_version: 1,
    }
}

pub fn build_market_snapshot(
    closes: &[f64],
    highs: &[f64],
    lows: &[f64],
    volumes: &[f64],
) -> IndicatorSnapshot {
    let mut snapshot = build_snapshot(closes);
    snapshot.atr_14 = calculate_atr(highs, lows, closes, 14);

    if let Some(latest_volume) = volumes.last() {
        let period = volumes.len().min(20);
        if period > 0 {
            let volume_average =
                volumes[volumes.len() - period..].iter().sum::<f64>() / period as f64;
            snapshot.volume_ma = Some(volume_average);
            if volume_average > 0.0 {
                snapshot.relative_volume = Some(latest_volume / volume_average);
            }
        }
    }

    let range_period = highs.len().min(lows.len()).min(20);
    if range_period > 0 {
        snapshot.support = Some(
            lows[lows.len() - range_period..]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
        );
        snapshot.resistance = Some(
            highs[highs.len() - range_period..]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max),
        );
    }

    let structure_period = highs.len().min(lows.len());
    if structure_period >= 4 {
        let half = structure_period / 2;
        let older_high = highs[highs.len() - structure_period..highs.len() - half]
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let recent_high = highs[highs.len() - half..]
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let older_low = lows[lows.len() - structure_period..lows.len() - half]
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let recent_low = lows[lows.len() - half..]
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);

        snapshot.market_structure = if recent_high > older_high && recent_low > older_low {
            "Higher highs and higher lows".to_owned()
        } else if recent_high < older_high && recent_low < older_low {
            "Lower highs and lower lows".to_owned()
        } else {
            "Mixed structure".to_owned()
        };
    }

    snapshot
}
