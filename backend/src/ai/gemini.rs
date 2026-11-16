use async_trait::async_trait;
use serde_json::{Value, json};

use super::provider::AiProvider;

#[derive(Debug, Clone)]
pub struct GeminiClient {
    api_key: String,
    model: String,
    http_client: reqwest::Client,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String, http_client: reqwest::Client) -> Self {
        Self {
            api_key,
            model,
            http_client,
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        )
    }
}

#[async_trait]
impl AiProvider for GeminiClient {
    async fn analyze(&self, snapshot: &Value) -> anyhow::Result<Value> {
        let prompt = format!(
            "Analyze this BTC perpetual-futures market snapshot using only the supplied data. \
             You are an informational paper-trading assistant, not a financial adviser. \
             Return one JSON object only, with keys: decision (LONG, SHORT, or WAIT), \
             market_regime, timeframe, entry_type (MARKET or LIMIT), entry_price, stop_loss, \
             take_profit, risk_reward_ratio, reasoning (array of concise strings), \
             invalidation, valid_for_minutes. Never invent missing indicators or prices. \
             Write all reasoning and invalidation text in Vietnamese. \
             Use WAIT when the data is insufficient or no setup is clear. For WAIT, numeric \
             trade prices and risk_reward_ratio may be zero. For LONG, stop_loss < entry_price \
             < take_profit; for SHORT, take_profit < entry_price < stop_loss. \
             Market snapshot: {snapshot}"
        );

        let response = self
            .http_client
            .post(self.endpoint())
            .header("x-goog-api-key", &self.api_key)
            .json(&json!({
                "contents": [{
                    "role": "user",
                    "parts": [{ "text": prompt }]
                }],
                "generationConfig": {
                    "responseMimeType": "application/json",
                    "temperature": 0.2
                }
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let details = response.text().await?;
            anyhow::bail!("Gemini API returned HTTP {status}: {details}");
        }

        let payload: Value = response.json().await?;
        let text = payload["candidates"]
            .as_array()
            .and_then(|candidates| candidates.first())
            .and_then(|candidate| candidate["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|part| part["text"].as_str())
            .ok_or_else(|| anyhow::anyhow!("Gemini response did not contain a decision"))?;

        serde_json::from_str(text)
            .map_err(|error| anyhow::anyhow!("Gemini returned invalid JSON: {error}"))
    }
}
