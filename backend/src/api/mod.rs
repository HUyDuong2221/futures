use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/market/status", get(market_status))
        .route("/api/market/ticker", get(market_ticker))
        .route("/api/market/candles", get(market_candles))
        .route("/api/market/indicators", get(market_indicators))
        .route("/api/ai/analyze", post(ai_analyze))
        .route("/api/ai/latest", get(ai_latest))
        .route("/api/ai/history", get(ai_history))
        .route("/api/ai/status", get(ai_status))
        .route("/api/settings", get(settings))
        .route("/api/simulation/accounts", get(simulation_accounts))
        .route("/api/simulation/orders", post(create_paper_order))
        .route(
            "/api/simulation/settings",
            get(simulation_settings).put(update_simulation_settings),
        )
        .route("/api/simulation/positions", get(simulation_positions))
        .route(
            "/api/simulation/positions/{id}",
            delete(close_paper_position),
        )
        .route("/api/simulation/trades", get(simulation_trades))
        .route("/api/performance/summary", get(performance_summary))
        .route("/api/performance/equity-curve", get(equity_curve))
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "app": "btc-futures-ai-lab-backend",
        "instrument": state.settings.okx_instrument,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "paper_trading_only": state.settings.paper_trading_only,
    }))
}

async fn market_status(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let ticker = crate::market::fetch_okx_ticker(
        &state.http,
        &state.settings.okx_rest_url,
        &state.settings.okx_instrument,
    )
    .await
    .map_err(api_error)?;
    let status = crate::market::pseudo_market_status(&ticker.symbol, &ticker.timestamp);
    Ok(Json(json!(status)))
}

async fn market_ticker(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let ticker = crate::market::fetch_okx_ticker(
        &state.http,
        &state.settings.okx_rest_url,
        &state.settings.okx_instrument,
    )
    .await
    .map_err(api_error)?;

    Ok(Json(json!(ticker)))
}

#[derive(Debug, Deserialize)]
struct CandleQuery {
    timeframe: Option<String>,
}

async fn market_candles(
    State(state): State<AppState>,
    Query(query): Query<CandleQuery>,
) -> Result<Json<Value>, ApiError> {
    let timeframe = query
        .timeframe
        .as_deref()
        .unwrap_or(&state.settings.analysis_timeframe);
    if !matches!(
        timeframe,
        "1m" | "3m" | "5m" | "15m" | "30m" | "1H" | "2H" | "4H" | "6H" | "12H" | "1D" | "1W"
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Unsupported timeframe" })),
        ));
    }

    let candles = crate::market::fetch_okx_candles(
        &state.http,
        &state.settings.okx_rest_url,
        &state.settings.okx_instrument,
        timeframe,
    )
    .await
    .map_err(api_error)?;

    Ok(Json(json!({
        "symbol": state.settings.okx_instrument,
        "timeframe": timeframe,
        "candles": candles,
    })))
}

async fn ai_analyze(
    State(state): State<AppState>,
    Query(query): Query<CandleQuery>,
) -> Result<Json<Value>, ApiError> {
    if state
        .settings
        .gemini_api_key
        .as_deref()
        .is_none_or(|key| key.trim().is_empty())
    {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Chưa cấu hình GEMINI_API_KEY trong file .env." })),
        ));
    }
    let timeframe = query
        .timeframe
        .as_deref()
        .unwrap_or(&state.settings.analysis_timeframe);
    if !matches!(
        timeframe,
        "1m" | "3m" | "5m" | "15m" | "30m" | "1H" | "2H" | "4H" | "6H" | "12H" | "1D" | "1W"
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Khung thời gian không được hỗ trợ." })),
        ));
    }

    let analysis = generate_ai_analysis(&state, timeframe)
        .await
        .map_err(ai_error)?;
    Ok(Json(json!(analysis)))
}

#[derive(Debug, serde::Serialize)]
struct GeneratedAnalysis {
    decision: crate::ai::response::AiDecision,
    symbol: String,
    model: String,
    generated_at: String,
    signal_key: String,
}

async fn generate_ai_analysis(
    state: &AppState,
    timeframe: &str,
) -> anyhow::Result<GeneratedAnalysis> {
    let api_key = state
        .settings
        .gemini_api_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("GEMINI_API_KEY chưa được cấu hình."))?;
    let candles = crate::market::fetch_okx_candles(
        &state.http,
        &state.settings.okx_rest_url,
        &state.settings.okx_instrument,
        timeframe,
    )
    .await?;
    if candles.len() < 30 {
        anyhow::bail!("Không đủ dữ liệu nến để phân tích.");
    }

    let closes: Vec<f64> = candles.iter().map(|candle| candle.close).collect();
    let highs: Vec<f64> = candles.iter().map(|candle| candle.high).collect();
    let lows: Vec<f64> = candles.iter().map(|candle| candle.low).collect();
    let volumes: Vec<f64> = candles.iter().map(|candle| candle.volume).collect();
    let indicators = crate::analysis::build_market_snapshot(&closes, &highs, &lows, &volumes);
    let snapshot = json!({
        "symbol": state.settings.okx_instrument,
        "timeframe": timeframe,
        "latest_candle": candles.last(),
        "indicators": indicators,
    });

    let provider = crate::ai::gemini::GeminiClient::new(
        api_key.to_owned(),
        state.settings.gemini_model.clone(),
        state.ai_http.clone(),
    );
    let raw_decision = crate::ai::provider::AiProvider::analyze(&provider, &snapshot).await?;
    let mut decision: crate::ai::response::AiDecision = serde_json::from_value(raw_decision)?;
    decision.timeframe = timeframe.to_owned();

    if !decision.is_valid() {
        anyhow::bail!("AI trả về tín hiệu không hợp lệ; không tạo lệnh.");
    }
    if decision.decision == "LONG" || decision.decision == "SHORT" {
        let risk = (decision.entry_price - decision.stop_loss).abs();
        let reward = (decision.take_profit - decision.entry_price).abs();
        decision.risk_reward_ratio = reward / risk;
    } else {
        decision.risk_reward_ratio = 0.0;
    }

    let latest_candle_time = candles
        .last()
        .ok_or_else(|| anyhow::anyhow!("Không có nến mới nhất để định danh tín hiệu."))?
        .time;
    Ok(GeneratedAnalysis {
        signal_key: format!("{timeframe}:{latest_candle_time}:{}", decision.decision),
        decision,
        symbol: state.settings.okx_instrument.clone(),
        model: state.settings.gemini_model.clone(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub async fn run_auto_paper_cycle(state: &AppState) {
    let auto_settings = {
        let mut paper = state.paper.lock().await;
        if paper.auto_scan_due() {
            Some(paper.settings().clone())
        } else {
            None
        }
    };
    let Some(auto_settings) = auto_settings else {
        return;
    };
    if state
        .settings
        .gemini_api_key
        .as_deref()
        .is_none_or(|key| key.trim().is_empty())
    {
        tracing::error!("Automatic paper trading is enabled but GEMINI_API_KEY is missing");
        return;
    }

    let timeframe = state.settings.analysis_timeframe.as_str();
    let analysis = match generate_ai_analysis(state, timeframe).await {
        Ok(analysis) => analysis,
        Err(error) => {
            tracing::error!("Automatic AI analysis failed: {error:#}");
            return;
        }
    };
    if !matches!(analysis.decision.decision.as_str(), "LONG" | "SHORT") {
        tracing::info!(
            "Automatic paper scan returned WAIT for {}",
            state.settings.okx_instrument
        );
        return;
    }

    let ticker = match crate::market::fetch_okx_ticker(
        &state.http,
        &state.settings.okx_rest_url,
        &state.settings.okx_instrument,
    )
    .await
    {
        Ok(ticker) => ticker,
        Err(error) => {
            tracing::error!("Failed to fetch entry price for automatic paper order: {error:#}");
            return;
        }
    };

    let mut paper = state.paper.lock().await;
    if !paper.settings().auto_trade_enabled {
        return;
    }
    if !auto_settings.allow_multiple_positions && !paper.positions().is_empty() {
        return;
    }
    if !paper.record_auto_signal(analysis.signal_key) {
        tracing::info!("Skipped duplicate automatic paper signal");
        return;
    }
    let request = crate::simulation::paper::PaperOrderRequest {
        direction: analysis.decision.decision,
        stop_loss: analysis.decision.stop_loss,
        take_profit: analysis.decision.take_profit,
        margin_amount: paper.settings().margin_per_order,
        leverage: paper.settings().leverage,
    };
    match paper.open_position(request, ticker.last_price, state.settings.max_leverage) {
        Ok(position) => tracing::info!(
            "Opened automatic paper {} position {} at {}",
            position.direction,
            position.id,
            position.entry_price
        ),
        Err(error) => tracing::error!("Automatic paper order rejected: {error:#}"),
    }
}

pub async fn monitor_paper_positions(state: &AppState) {
    if state.paper.lock().await.positions().is_empty() {
        return;
    }
    if let Err((_, error)) = refresh_paper_price(state).await {
        tracing::error!("Failed to update automatic paper positions: {}", error.0);
    }
}

type ApiError = (StatusCode, Json<Value>);

fn api_error(error: anyhow::Error) -> ApiError {
    tracing::error!("Failed to fetch market data from OKX: {error:#}");
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({ "error": "Market data provider request failed" })),
    )
}

fn ai_error(error: anyhow::Error) -> ApiError {
    tracing::error!("AI analysis failed: {error:#}");
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({ "error": format!("Phân tích AI thất bại: {error}") })),
    )
}

async fn market_indicators(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({
        "ema_20": 68520.0,
        "ema_50": 68290.0,
        "ema_200": 67880.0,
        "rsi_14": 52.0,
        "macd": 0.4,
        "atr_14": 98.0,
        "regime": "Bullish Trending",
        "support": 68200.0,
        "resistance": 68850.0,
    }))
}

async fn ai_latest(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({
        "available": false,
        "message": "Chưa chạy phân tích trong phiên này.",
    }))
}

async fn ai_history(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({
        "signals": [],
        "total": 0,
    }))
}

async fn ai_status(State(state): State<AppState>) -> Json<Value> {
    let configured = state
        .settings
        .gemini_api_key
        .as_deref()
        .is_some_and(|key| !key.trim().is_empty());
    Json(json!({
        "status": if configured { "ready" } else { "not_configured" },
        "model": state.settings.gemini_model,
        "auto_analyze": state.settings.auto_analyze,
        "api_mode": state.settings.gemini_api_mode,
        "rate_limited": false,
    }))
}

async fn settings(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "app_env": state.settings.app_env,
        "analysis_timeframe": state.settings.analysis_timeframe,
        "timezone": state.settings.timezone,
        "paper_trading_only": state.settings.paper_trading_only,
        "default_risk_per_trade": state.settings.default_risk_per_trade,
        "max_leverage": state.settings.max_leverage,
    }))
}

async fn simulation_accounts(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let mark_price = refresh_paper_price(&state).await?;
    let paper = state.paper.lock().await;
    let account = paper.summary(state.settings.max_leverage);
    Ok(Json(json!({
        "account": account,
        "mark_price": mark_price,
    })))
}

async fn simulation_positions(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let mark_price = refresh_paper_price(&state).await?;
    let paper = state.paper.lock().await;
    Ok(Json(json!({
        "positions": paper.positions(),
        "count": paper.positions().len(),
        "mark_price": mark_price,
    })))
}

async fn create_paper_order(
    State(state): State<AppState>,
    Json(request): Json<crate::simulation::paper::PaperOrderRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let ticker = crate::market::fetch_okx_ticker(
        &state.http,
        &state.settings.okx_rest_url,
        &state.settings.okx_instrument,
    )
    .await
    .map_err(api_error)?;
    let mut paper = state.paper.lock().await;
    paper.update_price(ticker.last_price);
    let position = paper
        .open_position(request, ticker.last_price, state.settings.max_leverage)
        .map_err(|error| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": error.to_string() })),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "position": position,
            "account": paper.summary(state.settings.max_leverage),
        })),
    ))
}

async fn simulation_settings(State(state): State<AppState>) -> Json<Value> {
    let paper = state.paper.lock().await;
    Json(json!({ "settings": paper.settings() }))
}

async fn update_simulation_settings(
    State(state): State<AppState>,
    Json(request): Json<crate::simulation::paper::PaperSettingsRequest>,
) -> Result<Json<Value>, ApiError> {
    if request.auto_trade_enabled
        && state
            .settings
            .gemini_api_key
            .as_deref()
            .is_none_or(|key| key.trim().is_empty())
    {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Cần cấu hình GEMINI_API_KEY trước khi bật AI tự vào lệnh." })),
        ));
    }

    let mut paper = state.paper.lock().await;
    paper
        .update_settings(request, state.settings.max_leverage)
        .map_err(|error| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": error.to_string() })),
            )
        })?;
    Ok(Json(json!({ "settings": paper.settings() })))
}

async fn close_paper_position(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let ticker = crate::market::fetch_okx_ticker(
        &state.http,
        &state.settings.okx_rest_url,
        &state.settings.okx_instrument,
    )
    .await
    .map_err(api_error)?;
    let mut paper = state.paper.lock().await;
    paper.update_price(ticker.last_price);
    let trade = paper
        .close_position(&id, ticker.last_price)
        .map_err(|error| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": error.to_string() })),
            )
        })?;
    Ok(Json(json!({ "trade": trade })))
}

async fn simulation_trades(State(state): State<AppState>) -> Json<Value> {
    let paper = state.paper.lock().await;
    Json(json!({
        "trades": paper.trades(),
        "count": paper.trades().len(),
    }))
}

async fn performance_summary(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    refresh_paper_price(&state).await?;
    let paper = state.paper.lock().await;
    let trades = paper.trades();
    let wins = trades.iter().filter(|trade| trade.pnl > 0.0).count();
    let win_rate = if trades.is_empty() {
        0.0
    } else {
        wins as f64 / trades.len() as f64 * 100.0
    };
    let account = paper.summary(state.settings.max_leverage);
    Ok(Json(json!({
        "total_trades": trades.len(),
        "win_rate": win_rate,
        "net_pnl": account.total_pnl,
        "realized_pnl": account.realized_pnl,
        "unrealized_pnl": account.unrealized_pnl,
    })))
}

async fn equity_curve(State(state): State<AppState>) -> Json<Value> {
    let paper = state.paper.lock().await;
    let account = paper.summary(state.settings.max_leverage);
    let mut equity = account.initial_balance;
    let mut curve = vec![json!({
        "time": chrono::Utc::now().to_rfc3339(),
        "equity": equity,
    })];
    for trade in paper.trades() {
        equity += trade.pnl;
        curve.push(json!({
            "time": trade.closed_at,
            "equity": equity,
        }));
    }

    Json(json!({ "equity_curve": curve }))
}

async fn refresh_paper_price(state: &AppState) -> Result<f64, ApiError> {
    let ticker = crate::market::fetch_okx_ticker(
        &state.http,
        &state.settings.okx_rest_url,
        &state.settings.okx_instrument,
    )
    .await
    .map_err(api_error)?;
    state.paper.lock().await.update_price(ticker.last_price);
    Ok(ticker.last_price)
}
