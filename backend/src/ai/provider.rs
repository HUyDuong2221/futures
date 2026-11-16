use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait AiProvider {
    async fn analyze(&self, snapshot: &Value) -> anyhow::Result<Value>;
}
