use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

use noah_tools::{ChangeRecord, SafetyTier, Tool, ToolResult};

// ── MacPrinterList ─────────────────────────────────────────────────────

pub struct MacPrinterList;

#[async_trait]
impl Tool for MacPrinterList {
    fn name(&self) -> &str {
        "mac_printer_list"
    }

    fn description(&self) -> &str {
        "List all configured printers and the default printer."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn safety_tier(&self) -> SafetyTier {
        SafetyTier::ReadOnly
    }

    async fn execute(&self, _input: &Value) -> Result<ToolResult> {
        let output = Command::new("lpstat")
            .args(["-p", "-d"])
            .output()
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                if stdout.is_empty() && !stderr.is_empty() {
                    stderr
                } else {
                    stdout
                }
            })
            .unwrap_or_else(|e| format!("lpstat failed: {}", e));

        Ok(ToolResult::read_only(
            output.clone(),
            json!({ "raw_output": output.trim() }),
        ))
    }
}

// ── MacPrintQueue ──────────────────────────────────────────────────────

pub struct MacPrintQueue;

#[async_trait]
impl Tool for MacPrintQueue {
    fn name(&self) -> &str {
        "mac_print_queue"
    }

    fn description(&self) -> &str {
        "Show all pending print jobs across all printers."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn safety_tier(&self) -> SafetyTier {
        SafetyTier::ReadOnly
    }

    async fn execute(&self, _input: &Value) -> Result<ToolResult> {
        let output = Command::new("lpstat")
            .arg("-o")
            .output()
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                if stdout.trim().is_empty() {
                    "No pending print jobs.".to_string()
                } else {
                    stdout
                }
            })
            .unwrap_or_else(|e| format!("lpstat failed: {}", e));

        Ok(ToolResult::read_only(
            output.clone(),
            json!({ "raw_output": output.trim() }),
        ))
    }
}

// ── MacCancelPrintJobs ─────────────────────────────────────────────────

pub struct MacCancelPrintJobs;

#[async_trait]
impl Tool for MacCancelPrintJobs {
    fn name(&self) -> &str {
        "mac_cancel_print_jobs"
    }

    fn description(&self) -> &str {
        "Cancel all pending print jobs. Requires user approval."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn safety_tier(&self) -> SafetyTier {
        SafetyTier::SafeAction
    }

    async fn execute(&self, _input: &Value) -> Result<ToolResult> {
        let output = Command::new("cancel")
            .arg("-a")
            .output()
            .map(|o| {
                if o.status.success() {
                    "All print jobs cancelled successfully.".to_string()
                } else {
                    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                    format!("cancel command completed with errors: {}", stderr.trim())
                }
            })
            .unwrap_or_else(|e| format!("cancel failed: {}", e));

        Ok(ToolResult::with_changes(
            output.clone(),
            json!({ "status": output }),
            vec![ChangeRecord {
                description: "Cancelled all pending print jobs".to_string(),
                undo_tool: String::new(),
                undo_input: json!(null),
            }],
        ))
    }
}

// ── MacRestartCups ─────────────────────────────────────────────────────

pub struct MacRestartCups;

#[async_trait]
impl Tool for MacRestartCups {
    fn name(&self) -> &str {
        "mac_restart_cups"
    }

    fn description(&self) -> &str {
        "Restart the CUPS printing service. This can fix stuck print queues."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn safety_tier(&self) -> SafetyTier {
        SafetyTier::SafeAction
    }

    async fn execute(&self, _input: &Value) -> Result<ToolResult> {
        // Try to restart CUPS via launchctl
        let stop = Command::new("launchctl")
            .args(["unload", "/System/Library/LaunchDaemons/org.cups.cupsd.plist"])
            .output();

        let start = Command::new("launchctl")
            .args(["load", "/System/Library/LaunchDaemons/org.cups.cupsd.plist"])
            .output();

        let msg = match (stop, start) {
            (Ok(_), Ok(s)) if s.status.success() => {
                "CUPS printing service restarted successfully.".to_string()
            }
            _ => {
                // Fallback: try killall cupsd
                let fallback = Command::new("killall")
                    .arg("cupsd")
                    .output();
                match fallback {
                    Ok(o) if o.status.success() => {
                        "CUPS restarted via killall (it will auto-restart).".to_string()
                    }
                    _ => "Failed to restart CUPS. You may need to restart manually via System Settings.".to_string(),
                }
            }
        };

        Ok(ToolResult::with_changes(
            msg.clone(),
            json!({ "status": msg }),
            vec![ChangeRecord {
                description: "Restarted CUPS printing service".to_string(),
                undo_tool: String::new(),
                undo_input: json!(null),
            }],
        ))
    }
}
