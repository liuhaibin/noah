use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SafetyTier {
    ReadOnly,
    SafeAction,
    NeedsApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub description: String,
    pub undo_tool: String,
    pub undo_input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: String,
    pub data: Value,
    pub changes: Vec<ChangeRecord>,
}

impl ToolResult {
    pub fn read_only(output: String, data: Value) -> Self {
        Self {
            output,
            data,
            changes: vec![],
        }
    }

    pub fn with_changes(output: String, data: Value, changes: Vec<ChangeRecord>) -> Self {
        Self {
            output,
            data,
            changes,
        }
    }
}
