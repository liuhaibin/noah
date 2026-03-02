use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::types::{SafetyTier, ToolResult};

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    fn safety_tier(&self) -> SafetyTier;

    /// Determine the safety tier based on the specific input.
    /// Override this to implement input-dependent safety checks (e.g.,
    /// auto-approve safe shell commands but block dangerous ones).
    fn safety_tier_for_input(&self, _input: &Value) -> SafetyTier {
        self.safety_tier()
    }

    async fn execute(&self, input: &Value) -> Result<ToolResult>;
}
