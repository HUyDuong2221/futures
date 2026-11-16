use std::sync::Arc;

use sqlx::PgPool;

use crate::{config::Settings, simulation::paper::PaperSimulation};

#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<Settings>,
    pub db: Option<PgPool>,
    pub http: reqwest::Client,
    pub ai_http: reqwest::Client,
    pub paper: std::sync::Arc<tokio::sync::Mutex<PaperSimulation>>,
}
