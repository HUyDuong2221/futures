mod ai;
mod analysis;
mod api;
mod app_state;
mod common;
mod config;
mod market;
mod simulation;
mod storage;

use std::sync::Arc;
use std::time::Duration;

use axum::{Router, routing::get};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::{app_state::AppState, config::Settings};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let settings = Settings::from_env()?;
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()?;
    let ai_http = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    info!(
        "Starting BTC Futures AI Lab backend using {}",
        settings.okx_instrument
    );

    let db = match PgPool::connect(&settings.database_url).await {
        Ok(pool) => {
            info!("Connected to PostgreSQL database");
            Some(pool)
        }
        Err(err) => {
            info!("Database unavailable at startup: {}", err);
            None
        }
    };

    let app_state = AppState {
        settings: Arc::new(settings),
        db,
        http,
        ai_http,
        paper: Arc::new(tokio::sync::Mutex::new(
            simulation::paper::PaperSimulation::default(),
        )),
    };
    let auto_trade_state = app_state.clone();
    tokio::spawn(async move {
        let mut schedule_check = tokio::time::interval(Duration::from_secs(5));
        loop {
            schedule_check.tick().await;
            api::monitor_paper_positions(&auto_trade_state).await;
            api::run_auto_paper_cycle(&auto_trade_state).await;
        }
    });

    let app = Router::new()
        .merge(api::router())
        .route(
            "/",
            get(|| async { "BTC Futures AI Lab backend is running" }),
        )
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_owned());
    let address = format!("0.0.0.0:{port}");
    info!("Listening on {}", address);
    let listener = tokio::net::TcpListener::bind(&address).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
