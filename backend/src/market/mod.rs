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
        .map(|row| {
            let values = row
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("OKX returned a malformed candle"))?;
            if values.len() < 6 {
                anyhow::bail!("OKX returned a candle with fewer than six fields");
            }

            let timestamp_ms = values[0]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("OKX candle timestamp is missing"))?
                .parse::<i64>()?;

            Ok(Candle {
                time: timestamp_ms / 1000,
                open: parse_value(&values[1], "open")?,
                high: parse_value(&values[2], "high")?,
                low: parse_value(&values[3], "low")?,
                close: parse_value(&values[4], "close")?,
                volume: parse_value(&values[5], "volume")?,
            })
        })
        .collect::<anyhow::Result<_>>()?;
    candles.reverse();
    Ok(candles)
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

fn value_as_f64(value: &Value, key: &str) -> anyhow::Result<f64> {
    let text = value[key]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("OKX ticker field '{key}' is missing"))?;
    text.parse::<f64>()
        .map_err(|error| anyhow::anyhow!("Invalid OKX ticker field '{key}': {error}"))
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
