use std::env;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub app_env: String,
    pub server_port: u16,
    pub database_url: String,
    pub okx_instrument: String,
    pub okx_rest_url: String,
    pub okx_ws_public: String,
    pub okx_ws_business: String,
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
    pub gemini_api_mode: String,
    pub analysis_timeframe: String,
    pub auto_analyze: bool,
    pub auto_paper_trading: bool,
    pub paper_trading_only: bool,
    pub default_risk_per_trade: f64,
    pub max_leverage: i32,
    pub max_daily_loss_percent: f64,
    pub timezone: String,
}

impl Settings {
    pub fn from_env() -> anyhow::Result<Self> {
        let configured_gemini_api_mode =
            env::var("GEMINI_API_MODE").unwrap_or_else(|_| "generateContent".to_owned());
        let gemini_api_mode = match configured_gemini_api_mode.as_str() {
            "generateContent" => "generateContent".to_owned(),
            "interactions" => {
                tracing::warn!("GEMINI_API_MODE=interactions is deprecated; using generateContent");
                "generateContent".to_owned()
            }
            _ => anyhow::bail!("GEMINI_API_MODE must be 'generateContent'"),
        };

        let settings = Self {
            app_env: env::var("APP_ENV").unwrap_or_else(|_| "development".to_owned()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_owned())
                .parse()?,
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgresql://postgres:postgres@localhost:5432/btc_futures_ai_lab".to_owned()
            }),
            okx_instrument: env::var("OKX_INSTRUMENT")
                .unwrap_or_else(|_| "BTC-USDT-SWAP".to_owned()),
            okx_rest_url: env::var("OKX_REST_URL")
                .unwrap_or_else(|_| "https://www.okx.com".to_owned()),
            okx_ws_public: env::var("OKX_WS_PUBLIC")
                .unwrap_or_else(|_| "wss://ws.okx.com/ws/v5/public".to_owned()),
            okx_ws_business: env::var("OKX_WS_BUSINESS")
                .unwrap_or_else(|_| "wss://ws.okx.com/ws/v5/business".to_owned()),
            gemini_api_key: env::var("GEMINI_API_KEY").ok(),
            gemini_model: env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-3.6-flash".to_owned()),
            gemini_api_mode,
            analysis_timeframe: env::var("ANALYSIS_TIMEFRAME").unwrap_or_else(|_| "15m".to_owned()),
            auto_analyze: env::var("AUTO_ANALYZE")
                .unwrap_or("false".to_owned())
                .parse()
                .unwrap_or(false),
            auto_paper_trading: env::var("AUTO_PAPER_TRADING")
                .unwrap_or("false".to_owned())
                .parse()
                .unwrap_or(false),
            paper_trading_only: env::var("PAPER_TRADING_ONLY")
                .unwrap_or("true".to_owned())
                .parse()
                .unwrap_or(true),
            default_risk_per_trade: env::var("DEFAULT_RISK_PER_TRADE")
                .unwrap_or("0.01".to_owned())
                .parse()?,
            max_leverage: env::var("MAX_LEVERAGE")
                .unwrap_or("20".to_owned())
                .parse()?,
            max_daily_loss_percent: env::var("MAX_DAILY_LOSS_PERCENT")
                .unwrap_or("0.05".to_owned())
                .parse()?,
            timezone: env::var("TIMEZONE").unwrap_or_else(|_| "Asia/Ho_Chi_Minh".to_owned()),
        };

        Ok(settings)
    }
}
