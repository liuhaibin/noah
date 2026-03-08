//! Headless debug runner for end-to-end backend testing.
//!
//! Instantiates a real Orchestrator with a mock Tauri app handle,
//! sends prompts through the full agentic loop, and returns structured results.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use tokio::sync::Mutex;

use crate::agent::llm_client::LlmClient;
use crate::agent::orchestrator::{Orchestrator, PendingApprovals};
use crate::agent::tool_router::ToolRouter;
use crate::commands::agent::{parse_assistant_ui, AssistantActionType, AssistantUiPayload};
use crate::knowledge;
use crate::machine_context::MachineContext;
use crate::platform;
use crate::playbooks;
use crate::safety::journal;
use crate::ui_tools;

#[derive(Debug, Clone)]
pub struct PromptRunResult {
    pub session_id: String,
    pub turns: Vec<(String, String)>,
    pub reached_done: bool,
}

fn default_app_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("NOAH_APP_DIR") {
        return Ok(PathBuf::from(dir));
    }

    let home = std::env::var("HOME").context("HOME is not set")?;
    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from(home).join("Library/Application Support/com.itman.app"))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from(home).join(".local/share/com.itman.app"))
    }
    #[cfg(target_os = "windows")]
    {
        Ok(PathBuf::from(home).join("AppData/Roaming/com.itman.app"))
    }
}

/// Auto-approves all pending approval requests (for headless testing).
async fn spawn_auto_approver(pending: PendingApprovals) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let senders = {
                let mut map = pending.lock().await;
                if map.is_empty() {
                    Vec::new()
                } else {
                    map.drain().map(|(_, sender)| sender).collect::<Vec<_>>()
                }
            };

            for sender in senders {
                let _ = sender.send(true);
            }

            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
    })
}

/// Run a prompt through the full orchestrator pipeline and return results.
///
/// This creates a real Orchestrator with all tools registered, sends the prompt,
/// and auto-confirms any actions. Useful for end-to-end testing of the UI tool flow.
pub async fn run_prompt_flow(prompt: &str, max_turns: usize) -> Result<PromptRunResult> {
    let app_dir = default_app_dir()?;
    std::fs::create_dir_all(&app_dir).context("Failed to create app dir")?;

    let db_path = app_dir.join("journal.db");
    let db = journal::init_db(
        db_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid DB path"))?,
    )?;
    let db_arc = Arc::new(Mutex::new(db));

    let knowledge_dir = knowledge::init_knowledge_dir(&app_dir)?;
    {
        let conn = db_arc.lock().await;
        journal::run_file_migrations(&conn, &knowledge_dir)?;
    }

    let mut router = ToolRouter::new();
    platform::register_platform_tools(&mut router, Some(&db_path));
    ui_tools::register_ui_tools(&mut router);
    router.register(Box::new(knowledge::WriteKnowledgeTool::new(
        knowledge_dir.clone(),
    )));
    router.register(Box::new(knowledge::SearchKnowledgeTool::new(
        knowledge_dir.clone(),
    )));
    router.register(Box::new(knowledge::ReadKnowledgeTool::new(
        knowledge_dir.clone(),
    )));
    router.register(Box::new(knowledge::ListKnowledgeTool::new(
        knowledge_dir.clone(),
    )));

    // In debug/CLI mode, read bundled playbooks from the source tree.
    let bundled_playbooks = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("playbooks");
    let playbook_registry = playbooks::PlaybookRegistry::init(&knowledge_dir, &bundled_playbooks)?;
    router.register(Box::new(playbooks::ActivatePlaybookTool::new(
        playbook_registry,
    )));

    let auth = super::load_auth(&app_dir);
    let llm = LlmClient::with_auth(auth);
    if !llm.has_auth() {
        return Err(anyhow!(
            "No auth configured. Set proxy/api key in app config or ANTHROPIC_API_KEY."
        ));
    }

    let pending_approvals: PendingApprovals =
        Arc::new(Mutex::new(HashMap::<String, tokio::sync::oneshot::Sender<bool>>::new()));
    let mut orchestrator = Orchestrator::new(
        llm,
        router,
        MachineContext::load_or_gather(&app_dir).to_prompt_string(),
        pending_approvals.clone(),
        db_arc.clone(),
        knowledge_dir,
    );
    let session_id = orchestrator.create_session();
    {
        let conn = db_arc.lock().await;
        journal::create_session_record(&conn, &session_id, &chrono::Utc::now().to_rfc3339())?;
    }

    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();
    let approver = spawn_auto_approver(pending_approvals).await;

    let mut turns = Vec::new();
    let mut input = prompt.to_string();
    let mut reached_done = false;

    for _ in 0..max_turns {
        {
            let conn = db_arc.lock().await;
            journal::save_message(&conn, &session_id, "user", &input)?;
        }

        let output = match tokio::time::timeout(
            std::time::Duration::from_secs(600),
            orchestrator.send_message(&session_id, &input, &app_handle, &db_arc),
        )
        .await
        {
            Ok(res) => res.context("orchestrator send_message failed")?,
            Err(_) => r#"{"kind":"info","summary":"Runner timeout waiting for assistant response."}"#.to_string(),
        };
        {
            let conn = db_arc.lock().await;
            journal::save_message(&conn, &session_id, "assistant", &output)?;
            if let Some(session) = orchestrator.get_session(&session_id) {
                let count = session.messages.len() as i32;
                journal::update_session_message_count(&conn, &session_id, count)?;
            }
        }
        turns.push((input.clone(), output.clone()));

        // Check for done
        match parse_assistant_ui(&output) {
            Some(AssistantUiPayload::Done(_)) => {
                reached_done = true;
                break;
            }
            Some(AssistantUiPayload::Spa(ref spa)) => {
                // WAIT_FOR_USER: simulate user completing external action.
                if spa.action.action_type == AssistantActionType::WaitForUser {
                    input = "I've done this, let's continue.".to_string();
                } else {
                    input = "Go ahead".to_string();
                }
                continue;
            }
            Some(AssistantUiPayload::UserQuestion(ref uq)) => {
                // Generate a reasonable auto-answer based on question type.
                let q = &uq.questions[0];
                if q.secure_input.is_some() {
                    // For secure inputs, store a dummy secret value via orchestrator.
                    let secret_name = q.secure_input.as_ref().unwrap().secret_name.clone();
                    orchestrator.store_secret(&session_id, &secret_name, "test-secret-value-12345");
                    input = format!("[SECRET:{}] stored securely", secret_name);
                } else if q.text_input.is_some() {
                    // Provide a plausible test answer based on the question context.
                    let header = q.header.to_lowercase();
                    let question = q.question.to_lowercase();
                    let answer = if header.contains("email") || question.contains("email") {
                        "testuser@example.com".to_string()
                    } else if header.contains("ssid") || question.contains("ssid") || question.contains("wi-fi") || question.contains("wifi") || question.contains("network name") {
                        "TestNetwork".to_string()
                    } else if header.contains("server") || question.contains("server") {
                        "mail.example.com".to_string()
                    } else if header.contains("drive") || question.contains("drive") || question.contains("connect") {
                        "My Backup Drive, 1TB, appears in Finder".to_string()
                    } else if header.contains("username") || question.contains("username") {
                        "testuser".to_string()
                    } else if header.contains("path") || question.contains("path") || question.contains("folder") {
                        "/Users/test/Documents".to_string()
                    } else {
                        "test-input-value".to_string()
                    };
                    input = answer;
                } else if let Some(ref opts) = q.options {
                    // Pick the first option.
                    if let Some(first) = opts.first() {
                        input = first.label.clone();
                    } else {
                        input = "Yes".to_string();
                    }
                } else {
                    input = "Yes".to_string();
                }
                continue;
            }
            Some(AssistantUiPayload::Info(_)) => {
                // Info is terminal-ish, break
                break;
            }
            None => {
                // No UI payload — plain text response. Continue with a generic followup.
                input = "Continue. Please use the structured UI tools (ui_spa, ui_user_question, ui_info, or ui_done) to present your response.".to_string();
                continue;
            }
        }
    }

    approver.abort();

    Ok(PromptRunResult {
        session_id,
        turns,
        reached_done,
    })
}
