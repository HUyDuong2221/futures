use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker {
    pub symbol: String,
    pub last_price: f64,
    pub mark_price: Option<f64>,
    pub bid_price: f64,
    pub ask_price: f64,
    pub volume_24h: f64,
    pub funding_rate: Option<f64>,
    pub open_interest: Option<f64>,
    pub high_24h: f64,
    pub low_24h: f64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStatus {
    pub instrument: String,
    pub online: bool,
    pub last_ts: String,
    pub data_age_seconds: u64,
    pub stale_data: bool,
}

pub async fn fetch_okx_ticker(
    client: &reqwest::Client,
    rest_url: &str,
    symbol: &str,
) -> anyhow::Result<Ticker> {
    let url = format!("{}/api/v5/market/ticker", rest_url.trim_end_matches('/'));
    let payload: Value = client
        .get(url)
        .query(&[("instId", symbol)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    ensure_okx_success(&payload)?;
    let item = payload["data"]
        .as_array()
        .and_then(|data| data.first())
        .ok_or_else(|| anyhow::anyhow!("OKX ticker response did not contain market data"))?;

    Ok(Ticker {
        symbol: symbol.to_owned(),
        last_price: value_as_f64(item, "last")?,
        mark_price: None,
        bid_price: value_as_f64(item, "bidPx")?,
        ask_price: value_as_f64(item, "askPx")?,
        volume_24h: value_as_f64(item, "volCcy24h")?,
        funding_rate: None,
        open_interest: None,
        high_24h: value_as_f64(item, "high24h")?,
        low_24h: value_as_f64(item, "low24h")?,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

pub async fn fetch_okx_candles(
    client: &reqwest::Client,
    rest_url: &str,
    symbol: &str,
    timeframe: &str,
) -> anyhow::Result<Vec<Candle>> {
    let url = format!("{}/api/v5/market/candles", rest_url.trim_end_matches('/'));
    let payload: Value = client
        .get(url)
        .query(&[("instId", symbol), ("bar", timeframe), ("limit", "100")])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    ensure_okx_success(&payload)?;

    let rows = payload["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("OKX candle response did not contain candle data"))?;

    let mut candles: Vec<Candle> = rows
        .iter()
        .map(parse_okx_candle)
        .collect::<anyhow::Result<_>>()?;
    candles.reverse();
    Ok(candles)
}

pub fn timeframe_seconds(timeframe: &str) -> Option<i64> {
    match timeframe {
        "1m" => Some(60),
        "3m" => Some(3 * 60),
        "5m" => Some(5 * 60),
        "15m" => Some(15 * 60),
        "30m" => Some(30 * 60),
        "1H" => Some(60 * 60),
        "2H" => Some(2 * 60 * 60),
        "4H" => Some(4 * 60 * 60),
        "6H" => Some(6 * 60 * 60),
        "12H" => Some(12 * 60 * 60),
        "1D" => Some(24 * 60 * 60),
        "1W" => Some(7 * 24 * 60 * 60),
        _ => None,
    }
}

fn ensure_okx_success(payload: &Value) -> anyhow::Result<()> {
    if payload["code"].as_str() != Some("0") {
        anyhow::bail!(
            "OKX API error {}: {}",
            payload["code"].as_str().unwrap_or("unknown"),
            payload["msg"].as_str().unwrap_or("no error message")
        );
    }

    Ok(())
}

fn parse_okx_candle(row: &Value) -> anyhow::Result<Candle> {
    let values = row
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("OKX returned a malformed candle"))?;
    if values.len() < 9 {
        anyhow::bail!("OKX returned a candle without its confirmation status");
    }

    let timestamp_ms = values[0]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("OKX candle timestamp is missing"))?
        .parse::<i64>()?;
    let confirmed = match values[8].as_str() {
        Some("1") => true,
        Some("0") => false,
        _ => anyhow::bail!("OKX returned an invalid candle confirmation status"),
    };

    Ok(Candle {
        time: timestamp_ms / 1000,
        open: parse_value(&values[1], "open")?,
        high: parse_value(&values[2], "high")?,
        low: parse_value(&values[3], "low")?,
        close: parse_value(&values[4], "close")?,
        volume: parse_value(&values[5], "volume")?,
        confirmed,
    })
}

fn value_as_f64(value: &Value, key: &str) -> anyhow::Result<f64> {
    let text = value[key]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("OKX ticker field '{key}' is missing"))?;
    text.parse::<f64>()
        .map_err(|error| anyhow::anyhow!("Invalid OKX ticker field '{key}': {error}"))
}

#[cfg(test)]
mod tests {
    use super::{parse_okx_candle, timeframe_seconds};

    #[test]
    fn parses_okx_candle_confirmation_status() {
        let closed: serde_json::Value = serde_json::from_str(
            r#"["1700000000000","1","2","0.5","1.5","10","10","10","1"]"#,
        )
        .unwrap();
        let open: serde_json::Value = serde_json::from_str(
            r#"["1700000000000","1","2","0.5","1.5","10","10","10","0"]"#,
        )
        .unwrap();

        assert!(parse_okx_candle(&closed).unwrap().confirmed);
        assert!(!parse_okx_candle(&open).unwrap().confirmed);
    }

    #[test]
    fn returns_configured_candle_duration() {
        assert_eq!(timeframe_seconds("15m"), Some(900));
        assert_eq!(timeframe_seconds("1H"), Some(3600));
        assert_eq!(timeframe_seconds("unsupported"), None);
    }
}

fn parse_value(value: &Value, field: &str) -> anyhow::Result<f64> {
    value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("OKX candle field '{field}' is missing"))?
        .parse::<f64>()
        .map_err(|error| anyhow::anyhow!("Invalid OKX candle field '{field}': {error}"))
}

pub fn pseudo_market_status(symbol: &str, last_ts: &str) -> MarketStatus {
    MarketStatus {
        instrument: symbol.to_owned(),
        online: true,
        last_ts: last_ts.to_owned(),
        data_age_seconds: 0,
        stale_data: false,
    }
}
