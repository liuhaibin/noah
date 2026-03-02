use anyhow::{Context, Result};
use async_trait::async_trait;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use itman_tools::{SafetyTier, Tool, ToolResult};

// ── Artifact struct ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub category: String,
    pub title: String,
    pub content: String,
    pub source: String,
    pub session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── CRUD functions ──────────────────────────────────────────────────────

pub fn save_artifact(
    conn: &Connection,
    category: &str,
    title: &str,
    content: &str,
    source: &str,
    session_id: Option<&str>,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO artifacts (id, category, title, content, source, session_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, category, title, content, source, session_id, now, now],
    )
    .context("Failed to insert artifact")?;

    Ok(id)
}

pub fn update_artifact(
    conn: &Connection,
    id: &str,
    title: Option<&str>,
    content: Option<&str>,
    category: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    // Build SET clause dynamically based on which fields are provided.
    let mut sets = vec!["updated_at = ?1"];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];

    if let Some(t) = title {
        params.push(Box::new(t.to_string()));
        sets.push("title = ?");
    }
    if let Some(c) = content {
        params.push(Box::new(c.to_string()));
        sets.push("content = ?");
    }
    if let Some(cat) = category {
        params.push(Box::new(cat.to_string()));
        sets.push("category = ?");
    }

    params.push(Box::new(id.to_string()));

    // Re-number placeholders.
    let numbered_sets: Vec<String> = sets
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i == 0 {
                s.to_string()
            } else {
                s.replace('?', &format!("?{}", i + 1))
            }
        })
        .collect();

    let sql = format!(
        "UPDATE artifacts SET {} WHERE id = ?{}",
        numbered_sets.join(", "),
        params.len()
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| &**p).collect();

    let rows = conn
        .execute(&sql, param_refs.as_slice())
        .context("Failed to update artifact")?;

    if rows == 0 {
        anyhow::bail!("Artifact not found: {}", id);
    }

    Ok(())
}

pub fn query_artifacts(
    conn: &Connection,
    search: Option<&str>,
    category: Option<&str>,
) -> Result<Vec<Artifact>> {
    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(cat) = category {
        params.push(Box::new(cat.to_string()));
        conditions.push(format!("category = ?{}", params.len()));
    }

    if let Some(q) = search {
        let like = format!("%{}%", q);
        params.push(Box::new(like.clone()));
        let idx1 = params.len();
        params.push(Box::new(like));
        let idx2 = params.len();
        conditions.push(format!("(title LIKE ?{} OR content LIKE ?{})", idx1, idx2));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, category, title, content, source, session_id, created_at, updated_at
         FROM artifacts {} ORDER BY updated_at DESC",
        where_clause
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| &**p).collect();

    let mut stmt = conn.prepare(&sql).context("Failed to prepare query_artifacts")?;
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(Artifact {
                id: row.get(0)?,
                category: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                source: row.get(4)?,
                session_id: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .context("Failed to execute query_artifacts")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to collect artifacts")?;

    Ok(rows)
}

pub fn delete_artifact(conn: &Connection, id: &str) -> Result<()> {
    let rows = conn
        .execute("DELETE FROM artifacts WHERE id = ?1", rusqlite::params![id])
        .context("Failed to delete artifact")?;

    if rows == 0 {
        anyhow::bail!("Artifact not found: {}", id);
    }

    Ok(())
}

// ── Prompt injection ────────────────────────────────────────────────────

/// Format all artifacts as a string block for inclusion in the system prompt.
/// Returns an empty string if there are no artifacts.
pub fn artifacts_for_prompt(conn: &Connection) -> Result<String> {
    let artifacts = query_artifacts(conn, None, None)?;

    if artifacts.is_empty() {
        return Ok(String::new());
    }

    let mut lines = vec!["## Known Facts About This System".to_string()];
    for a in &artifacts {
        lines.push(format!("- [{}] {}: {}", a.category, a.title, a.content));
    }
    Ok(lines.join("\n"))
}

// ── Contextual Suggestions ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub label: String,
    pub description: String,
}

/// Generate contextual suggestions based on recent knowledge artifacts.
/// Returns up to 2 suggestions from resolved issues / recurring patterns.
pub fn get_contextual_suggestions(conn: &Connection) -> Result<Vec<Suggestion>> {
    let mut suggestions = Vec::new();

    // Get recent resolved issues (max 2)
    let mut stmt = conn.prepare(
        "SELECT title, content FROM artifacts
         WHERE category IN ('resolved_issue', 'recurring_pattern')
         ORDER BY updated_at DESC LIMIT 2",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
        ))
    })?;

    for row in rows {
        let (title, _content) = row?;
        suggestions.push(Suggestion {
            label: format!("Check on: {}", title),
            description: "Follow up on a previous issue".to_string(),
        });
    }

    Ok(suggestions)
}

// ── LLM Tools ───────────────────────────────────────────────────────────

pub struct SaveArtifactTool {
    db: Arc<Mutex<Connection>>,
}

impl SaveArtifactTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for SaveArtifactTool {
    fn name(&self) -> &str {
        "save_artifact"
    }

    fn description(&self) -> &str {
        "Save a knowledge artifact about this system. Use this to remember device details, resolved issues, user preferences, or recurring patterns for future sessions."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "enum": ["device_fact", "resolved_issue", "config_note", "recurring_pattern", "preference", "general"],
                    "description": "The category of knowledge being saved."
                },
                "title": {
                    "type": "string",
                    "description": "A short, searchable title. Good: 'Slow WiFi fixed by DNS change to 8.8.8.8'. Bad: 'Network issue'."
                },
                "content": {
                    "type": "string",
                    "description": "The detailed knowledge to save."
                }
            },
            "required": ["category", "title", "content"]
        })
    }

    fn safety_tier(&self) -> SafetyTier {
        SafetyTier::SafeAction
    }

    async fn execute(&self, input: &Value) -> Result<ToolResult> {
        let category = input
            .get("category")
            .and_then(|v| v.as_str())
            .context("Missing 'category'")?;
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .context("Missing 'title'")?;
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .context("Missing 'content'")?;

        let conn = self.db.lock().await;
        let id = save_artifact(&conn, category, title, content, "agent", None)?;

        Ok(ToolResult::read_only(
            format!("Saved artifact '{}' (id: {})", title, id),
            json!({ "id": id }),
        ))
    }
}

pub struct QueryArtifactsTool {
    db: Arc<Mutex<Connection>>,
}

impl QueryArtifactsTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for QueryArtifactsTool {
    fn name(&self) -> &str {
        "query_artifacts"
    }

    fn description(&self) -> &str {
        "Search saved knowledge artifacts about this system. Use this to recall device details, past fixes, user preferences, or recurring patterns."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "search": {
                    "type": "string",
                    "description": "Text to search for in titles and content."
                },
                "category": {
                    "type": "string",
                    "enum": ["device_fact", "resolved_issue", "config_note", "recurring_pattern", "preference", "general"],
                    "description": "Filter by category."
                }
            },
            "required": []
        })
    }

    fn safety_tier(&self) -> SafetyTier {
        SafetyTier::ReadOnly
    }

    async fn execute(&self, input: &Value) -> Result<ToolResult> {
        let search = input.get("search").and_then(|v| v.as_str());
        let category = input.get("category").and_then(|v| v.as_str());

        let conn = self.db.lock().await;
        let artifacts = query_artifacts(&conn, search, category)?;

        if artifacts.is_empty() {
            return Ok(ToolResult::read_only(
                "No matching artifacts found.".to_string(),
                json!({ "artifacts": [] }),
            ));
        }

        let mut lines = Vec::new();
        for a in &artifacts {
            lines.push(format!("- [{}] {}: {}", a.category, a.title, a.content));
        }

        let data: Vec<Value> = artifacts
            .iter()
            .map(|a| {
                json!({
                    "id": a.id,
                    "category": a.category,
                    "title": a.title,
                    "content": a.content,
                })
            })
            .collect();

        Ok(ToolResult::read_only(
            format!("Found {} artifact(s):\n{}", artifacts.len(), lines.join("\n")),
            json!({ "artifacts": data }),
        ))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::journal;

    fn test_db() -> Connection {
        journal::init_db(":memory:").expect("Failed to init in-memory DB")
    }

    #[test]
    fn test_save_and_query_artifact() {
        let conn = test_db();
        let id = save_artifact(&conn, "device_fact", "Printer model", "HP LaserJet Pro M404n", "agent", None).unwrap();
        assert!(!id.is_empty());

        let results = query_artifacts(&conn, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
        assert_eq!(results[0].category, "device_fact");
        assert_eq!(results[0].title, "Printer model");
        assert_eq!(results[0].content, "HP LaserJet Pro M404n");
        assert_eq!(results[0].source, "agent");
    }

    #[test]
    fn test_query_by_category() {
        let conn = test_db();
        save_artifact(&conn, "device_fact", "Printer", "HP", "agent", None).unwrap();
        save_artifact(&conn, "preference", "Browser", "Chrome", "agent", None).unwrap();

        let results = query_artifacts(&conn, None, Some("device_fact")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Printer");
    }

    #[test]
    fn test_query_by_search() {
        let conn = test_db();
        save_artifact(&conn, "resolved_issue", "Slow WiFi fixed", "Changed DNS to 8.8.8.8", "agent", None).unwrap();
        save_artifact(&conn, "device_fact", "Printer", "HP LaserJet", "agent", None).unwrap();

        let results = query_artifacts(&conn, Some("DNS"), None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Slow WiFi fixed");
    }

    #[test]
    fn test_query_empty_results() {
        let conn = test_db();
        let results = query_artifacts(&conn, Some("nonexistent"), None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_update_artifact() {
        let conn = test_db();
        let id = save_artifact(&conn, "preference", "Browser", "Safari", "agent", None).unwrap();

        update_artifact(&conn, &id, None, Some("Chrome"), None).unwrap();

        let results = query_artifacts(&conn, None, None).unwrap();
        assert_eq!(results[0].content, "Chrome");
        assert_eq!(results[0].title, "Browser"); // unchanged
    }

    #[test]
    fn test_update_nonexistent_artifact() {
        let conn = test_db();
        let result = update_artifact(&conn, "does-not-exist", Some("new title"), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_artifact() {
        let conn = test_db();
        let id = save_artifact(&conn, "general", "Test", "content", "agent", None).unwrap();

        delete_artifact(&conn, &id).unwrap();

        let results = query_artifacts(&conn, None, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_delete_nonexistent_artifact() {
        let conn = test_db();
        let result = delete_artifact(&conn, "does-not-exist");
        assert!(result.is_err());
    }

    #[test]
    fn test_artifacts_for_prompt_empty() {
        let conn = test_db();
        let prompt = artifacts_for_prompt(&conn).unwrap();
        assert!(prompt.is_empty());
    }

    #[test]
    fn test_artifacts_for_prompt_with_data() {
        let conn = test_db();
        save_artifact(&conn, "device_fact", "Printer", "HP LaserJet Pro M404n", "agent", None).unwrap();
        save_artifact(&conn, "preference", "Browser", "Chrome over Safari", "agent", None).unwrap();

        let prompt = artifacts_for_prompt(&conn).unwrap();
        assert!(prompt.contains("## Known Facts About This System"));
        assert!(prompt.contains("[device_fact]"));
        assert!(prompt.contains("[preference]"));
        assert!(prompt.contains("HP LaserJet Pro M404n"));
    }

    #[tokio::test]
    async fn test_save_artifact_tool() {
        let conn = test_db();
        let db = Arc::new(Mutex::new(conn));
        let tool = SaveArtifactTool::new(db.clone());

        let input = json!({
            "category": "device_fact",
            "title": "Router model",
            "content": "Netgear Nighthawk R7000"
        });

        let result = tool.execute(&input).await.unwrap();
        assert!(result.output.contains("Saved artifact"));

        let conn = db.lock().await;
        let artifacts = query_artifacts(&conn, None, None).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].title, "Router model");
    }

    #[tokio::test]
    async fn test_query_artifacts_tool() {
        let conn = test_db();
        save_artifact(&conn, "resolved_issue", "DNS fix", "Changed to 8.8.8.8", "agent", None).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let tool = QueryArtifactsTool::new(db);

        let input = json!({ "search": "DNS" });
        let result = tool.execute(&input).await.unwrap();
        assert!(result.output.contains("1 artifact(s)"));
        assert!(result.output.contains("DNS fix"));
    }

    #[tokio::test]
    async fn test_query_artifacts_tool_no_results() {
        let conn = test_db();
        let db = Arc::new(Mutex::new(conn));
        let tool = QueryArtifactsTool::new(db);

        let input = json!({});
        let result = tool.execute(&input).await.unwrap();
        assert!(result.output.contains("No matching artifacts"));
    }
}
