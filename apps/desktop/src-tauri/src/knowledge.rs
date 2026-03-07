use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use itman_tools::{SafetyTier, Tool, ToolResult};

// ── Constants ───────────────────────────────────────────────────────────

const DEFAULT_CATEGORIES: &[&str] = &[
    "devices",
    "issues",
    "network",
    "playbooks",
    "preferences",
    "software",
];

// ── Init ────────────────────────────────────────────────────────────────

/// Create the `knowledge/` directory tree inside the app data dir.
pub fn init_knowledge_dir(app_dir: &Path) -> Result<PathBuf> {
    let knowledge_dir = app_dir.join("knowledge");
    for cat in DEFAULT_CATEGORIES {
        std::fs::create_dir_all(knowledge_dir.join(cat))
            .with_context(|| format!("Failed to create knowledge/{}", cat))?;
    }
    Ok(knowledge_dir)
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Turn a title/filename into a URL-safe slug.
pub fn slugify(input: &str) -> String {
    let lower = input.to_lowercase();
    let slug: String = lower
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse runs of dashes, trim leading/trailing dashes.
    let mut result = String::new();
    let mut prev_dash = true; // treat start as dash to trim leading
    for ch in slug.chars() {
        if ch == '-' {
            if !prev_dash {
                result.push('-');
            }
            prev_dash = true;
        } else {
            result.push(ch);
            prev_dash = false;
        }
    }
    // Trim trailing dash
    if result.ends_with('-') {
        result.pop();
    }
    if result.is_empty() {
        "untitled".to_string()
    } else {
        result
    }
}

/// Resolve a relative path inside `knowledge_dir`, rejecting traversal.
pub fn safe_resolve(knowledge_dir: &Path, relative: &str) -> Result<PathBuf> {
    let joined = knowledge_dir.join(relative);
    let canonical_base = knowledge_dir
        .canonicalize()
        .with_context(|| format!("Knowledge dir not found: {}", knowledge_dir.display()))?;

    // The file might not exist yet (for writes), so canonicalize the parent.
    let parent = joined.parent().unwrap_or(&joined);
    // Ensure parent exists for canonicalization.
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create parent: {}", parent.display()))?;

    let canonical_parent = parent
        .canonicalize()
        .with_context(|| format!("Cannot resolve: {}", parent.display()))?;

    if !canonical_parent.starts_with(&canonical_base) {
        anyhow::bail!("Path traversal rejected: {}", relative);
    }

    // Reconstruct the final path using the canonical parent + filename.
    let filename = joined.file_name().context("Missing filename")?;
    Ok(canonical_parent.join(filename))
}

// ── Knowledge entry ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub category: String,
    pub filename: String,
    pub path: String,
    pub title: String,
    pub playbook_type: Option<String>,
}

fn extract_playbook_type(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    let yaml_block = &after_first[..end];

    for line in yaml_block.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("type:") {
            let kind = value.trim();
            if !kind.is_empty() {
                return Some(kind.to_string());
            }
        }
    }

    None
}

/// Extract the title from the first `# ` heading line, or derive from filename.
fn extract_title(content: &str, filename: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            let title = heading.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }
    // Derive from filename: replace dashes with spaces, title-case first letter.
    let derived = filename.trim_end_matches(".md").replace('-', " ");
    let mut chars = derived.chars();
    match chars.next() {
        None => "Untitled".to_string(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

/// List all knowledge files, optionally filtered by category.
pub fn list_knowledge_tree(
    knowledge_dir: &Path,
    category: Option<&str>,
) -> Result<Vec<KnowledgeEntry>> {
    let mut entries = Vec::new();

    let dirs_to_scan: Vec<PathBuf> = if let Some(cat) = category {
        let cat_dir = knowledge_dir.join(cat);
        if cat_dir.is_dir() {
            vec![cat_dir]
        } else {
            return Ok(entries);
        }
    } else {
        // Scan all subdirectories.
        let mut dirs = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(knowledge_dir) {
            for entry in read_dir.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    dirs.push(entry.path());
                }
            }
        }
        dirs.sort();
        dirs
    };

    for dir in dirs_to_scan {
        let cat_name = dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if let Ok(read_dir) = std::fs::read_dir(&dir) {
            let mut files: Vec<_> = read_dir.flatten().collect();
            files.sort_by_key(|e| e.file_name());

            for file_entry in files {
                let fname = file_entry.file_name().to_string_lossy().to_string();
                if !fname.ends_with(".md") {
                    continue;
                }
                let rel_path = format!("{}/{}", cat_name, fname);
                let content = std::fs::read_to_string(file_entry.path()).unwrap_or_default();
                let title = extract_title(&content, &fname);
                let playbook_type = if cat_name == "playbooks" {
                    extract_playbook_type(&content).or_else(|| Some("user".to_string()))
                } else {
                    None
                };
                entries.push(KnowledgeEntry {
                    category: cat_name.clone(),
                    filename: fname,
                    path: rel_path,
                    title,
                    playbook_type,
                });
            }
        }
    }

    Ok(entries)
}

/// Build a table-of-contents string for the system prompt.
pub fn knowledge_toc(knowledge_dir: &Path) -> Result<String> {
    let entries = list_knowledge_tree(knowledge_dir, None)?;
    if entries.is_empty() {
        return Ok(String::new());
    }

    let mut lines = vec![
        "## Knowledge Base".to_string(),
        "Use `search_knowledge` or `read_knowledge` to access details. Files under `playbooks` are diagnostic protocols — use `activate_playbook` to load them.".to_string(),
        String::new(),
    ];

    let mut current_cat = String::new();
    for entry in &entries {
        if entry.category != current_cat {
            current_cat = entry.category.clone();
            lines.push(format!("### {}", current_cat));
        }
        lines.push(format!("- {} (`{}`)", entry.title, entry.path));
    }

    Ok(lines.join("\n"))
}

// ── LLM Tools ───────────────────────────────────────────────────────────

// -- WriteKnowledge --

pub struct WriteKnowledgeTool {
    knowledge_dir: PathBuf,
}

impl WriteKnowledgeTool {
    pub fn new(knowledge_dir: PathBuf) -> Self {
        Self { knowledge_dir }
    }
}

#[async_trait]
impl Tool for WriteKnowledgeTool {
    fn name(&self) -> &str {
        "write_knowledge"
    }

    fn description(&self) -> &str {
        "Create or update a markdown knowledge file. Use to remember device details, resolved issues, user preferences, or system configuration. The file is stored in a category folder."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "description": "Folder name: devices, issues, network, playbooks, preferences, software, or a new category name."
                },
                "filename": {
                    "type": "string",
                    "description": "Slug for the file (without .md). E.g. 'hp-laserjet-pro-m404n'."
                },
                "content": {
                    "type": "string",
                    "description": "Full markdown content. Start with '# Title'."
                }
            },
            "required": ["category", "filename", "content"]
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
        let filename = input
            .get("filename")
            .and_then(|v| v.as_str())
            .context("Missing 'filename'")?;
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .context("Missing 'content'")?;

        let slug = slugify(filename);
        let rel_path = format!("{}/{}.md", slugify(category), slug);
        let full_path = safe_resolve(&self.knowledge_dir, &rel_path)?;

        std::fs::write(&full_path, content)
            .with_context(|| format!("Failed to write {}", rel_path))?;

        Ok(ToolResult::read_only(
            format!("Saved knowledge file: {}", rel_path),
            json!({ "path": rel_path }),
        ))
    }
}

// -- SearchKnowledge --

pub struct SearchKnowledgeTool {
    knowledge_dir: PathBuf,
}

impl SearchKnowledgeTool {
    pub fn new(knowledge_dir: PathBuf) -> Self {
        Self { knowledge_dir }
    }
}

#[async_trait]
impl Tool for SearchKnowledgeTool {
    fn name(&self) -> &str {
        "search_knowledge"
    }

    fn description(&self) -> &str {
        "Search across all knowledge files for a keyword or phrase. Returns matching files with context snippets."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Text to search for (case-insensitive)."
                },
                "category": {
                    "type": "string",
                    "description": "Optional category folder to limit the search."
                }
            },
            "required": ["query"]
        })
    }

    fn safety_tier(&self) -> SafetyTier {
        SafetyTier::ReadOnly
    }

    async fn execute(&self, input: &Value) -> Result<ToolResult> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .context("Missing 'query'")?;
        let category = input.get("category").and_then(|v| v.as_str());

        let query_lower = query.to_lowercase();
        let entries = list_knowledge_tree(&self.knowledge_dir, category)?;
        let mut results: Vec<Value> = Vec::new();

        for entry in &entries {
            let full_path = self.knowledge_dir.join(&entry.path);
            let content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let lines: Vec<&str> = content.lines().collect();
            let mut snippets: Vec<String> = Vec::new();

            for (i, line) in lines.iter().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    // Grab line before, matching line, and line after.
                    let start = if i > 0 { i - 1 } else { i };
                    let end = (i + 2).min(lines.len());
                    let snippet: String = lines[start..end].join("\n");
                    snippets.push(snippet);
                    if snippets.len() >= 2 {
                        break; // Max 2 snippets per file
                    }
                }
            }

            if !snippets.is_empty() {
                results.push(json!({
                    "path": entry.path,
                    "title": entry.title,
                    "snippets": snippets,
                }));
            }

            if results.len() >= 10 {
                break;
            }
        }

        if results.is_empty() {
            return Ok(ToolResult::read_only(
                format!("No knowledge files match '{}'.", query),
                json!({ "results": [] }),
            ));
        }

        let mut lines = vec![format!("Found {} matching file(s):", results.len())];
        for r in &results {
            let path = r["path"].as_str().unwrap_or("");
            let title = r["title"].as_str().unwrap_or("");
            lines.push(format!("\n### {} (`{}`)", title, path));
            if let Some(snippets) = r["snippets"].as_array() {
                for s in snippets {
                    lines.push(format!("  {}", s.as_str().unwrap_or("")));
                }
            }
        }

        Ok(ToolResult::read_only(
            lines.join("\n"),
            json!({ "results": results }),
        ))
    }
}

// -- ReadKnowledge --

pub struct ReadKnowledgeTool {
    knowledge_dir: PathBuf,
}

impl ReadKnowledgeTool {
    pub fn new(knowledge_dir: PathBuf) -> Self {
        Self { knowledge_dir }
    }
}

#[async_trait]
impl Tool for ReadKnowledgeTool {
    fn name(&self) -> &str {
        "read_knowledge"
    }

    fn description(&self) -> &str {
        "Read the full content of a knowledge file by its relative path."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path, e.g. 'devices/hp-laserjet-pro-m404n.md'."
                }
            },
            "required": ["path"]
        })
    }

    fn safety_tier(&self) -> SafetyTier {
        SafetyTier::ReadOnly
    }

    async fn execute(&self, input: &Value) -> Result<ToolResult> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .context("Missing 'path'")?;
        let full_path = safe_resolve(&self.knowledge_dir, path)?;

        let content = std::fs::read_to_string(&full_path)
            .with_context(|| format!("File not found: {}", path))?;

        Ok(ToolResult::read_only(
            content.clone(),
            json!({ "path": path, "content": content }),
        ))
    }
}

// -- ListKnowledge --

pub struct ListKnowledgeTool {
    knowledge_dir: PathBuf,
}

impl ListKnowledgeTool {
    pub fn new(knowledge_dir: PathBuf) -> Self {
        Self { knowledge_dir }
    }
}

#[async_trait]
impl Tool for ListKnowledgeTool {
    fn name(&self) -> &str {
        "list_knowledge"
    }

    fn description(&self) -> &str {
        "List all knowledge files, optionally filtered by category."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "description": "Optional category folder to list."
                }
            },
            "required": []
        })
    }

    fn safety_tier(&self) -> SafetyTier {
        SafetyTier::ReadOnly
    }

    async fn execute(&self, input: &Value) -> Result<ToolResult> {
        let category = input.get("category").and_then(|v| v.as_str());
        let entries = list_knowledge_tree(&self.knowledge_dir, category)?;

        if entries.is_empty() {
            return Ok(ToolResult::read_only(
                "No knowledge files found.".to_string(),
                json!({ "entries": [] }),
            ));
        }

        let mut lines = Vec::new();
        let mut current_cat = String::new();
        for entry in &entries {
            if entry.category != current_cat {
                current_cat = entry.category.clone();
                lines.push(format!("\n### {}", current_cat));
            }
            lines.push(format!("- {} (`{}`)", entry.title, entry.path));
        }

        let data: Vec<Value> = entries
            .iter()
            .map(|e| {
                json!({
                    "category": e.category,
                    "filename": e.filename,
                    "path": e.path,
                    "title": e.title,
                })
            })
            .collect();

        Ok(ToolResult::read_only(
            format!("{} knowledge file(s):{}", entries.len(), lines.join("\n")),
            json!({ "entries": data }),
        ))
    }
}

// ── Migration ───────────────────────────────────────────────────────────

/// Migrate existing artifacts from SQLite to markdown files.
/// Called by journal migration 4.
pub fn migrate_artifacts_to_files(conn: &rusqlite::Connection, knowledge_dir: &Path) -> Result<()> {
    // Check if the artifacts table exists.
    let table_exists: bool = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='artifacts'")
        .and_then(|mut stmt| stmt.exists([]))
        .unwrap_or(false);

    if !table_exists {
        return Ok(());
    }

    let mut stmt =
        conn.prepare("SELECT category, title, content FROM artifacts ORDER BY updated_at ASC")?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    for row in rows {
        let (old_category, title, content) = row?;

        // Map old category to new folder.
        let new_folder = match old_category.as_str() {
            "device_fact" => "devices",
            "resolved_issue" => "issues",
            "config_note" => "software",
            "recurring_pattern" => "issues",
            "preference" => "preferences",
            "general" => "software",
            other => other, // Pass-through for any unknown
        };

        let slug = slugify(&title);
        let rel_path = format!("{}/{}.md", new_folder, slug);

        // Ensure the category dir exists.
        let cat_dir = knowledge_dir.join(new_folder);
        std::fs::create_dir_all(&cat_dir)?;

        let full_path = knowledge_dir.join(&rel_path);

        // Build markdown content.
        let md = format!("# {}\n\n{}", title, content);
        std::fs::write(&full_path, md)
            .with_context(|| format!("Failed to write migrated file: {}", rel_path))?;
    }

    Ok(())
}

// ── Delete ──────────────────────────────────────────────────────────────

/// Delete a knowledge file by relative path.
pub fn delete_knowledge_file(knowledge_dir: &Path, relative: &str) -> Result<()> {
    let full_path = safe_resolve(knowledge_dir, relative)?;
    if !full_path.exists() {
        anyhow::bail!("File not found: {}", relative);
    }
    std::fs::remove_file(&full_path).with_context(|| format!("Failed to delete: {}", relative))?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let kdir = init_knowledge_dir(tmp.path()).unwrap();
        (tmp, kdir)
    }

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("HP LaserJet Pro M404n"), "hp-laserjet-pro-m404n");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(
            slugify("Slow WiFi fixed (DNS change)"),
            "slow-wifi-fixed-dns-change"
        );
    }

    #[test]
    fn test_slugify_leading_trailing() {
        assert_eq!(slugify("---hello---"), "hello");
    }

    #[test]
    fn test_slugify_empty() {
        assert_eq!(slugify(""), "untitled");
        assert_eq!(slugify("---"), "untitled");
    }

    #[test]
    fn test_safe_resolve_valid() {
        let (_tmp, kdir) = setup();
        let path = safe_resolve(&kdir, "devices/test.md").unwrap();
        // Canonicalize kdir too (on macOS /var -> /private/var).
        let canonical_kdir = kdir.canonicalize().unwrap();
        assert!(path.starts_with(&canonical_kdir));
    }

    #[test]
    fn test_safe_resolve_traversal() {
        let (_tmp, kdir) = setup();
        let result = safe_resolve(&kdir, "../../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_init_creates_dirs() {
        let (_tmp, kdir) = setup();
        for cat in DEFAULT_CATEGORIES {
            assert!(kdir.join(cat).is_dir(), "Missing category dir: {}", cat);
        }
    }

    #[test]
    fn test_list_knowledge_tree_empty() {
        let (_tmp, kdir) = setup();
        let entries = list_knowledge_tree(&kdir, None).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_list_knowledge_tree_with_files() {
        let (_tmp, kdir) = setup();
        std::fs::write(kdir.join("devices/printer.md"), "# HP LaserJet\n\nDetails").unwrap();
        std::fs::write(
            kdir.join("issues/slow-wifi.md"),
            "# Slow WiFi Fixed\n\nChanged DNS",
        )
        .unwrap();

        let entries = list_knowledge_tree(&kdir, None).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].category, "devices");
        assert_eq!(entries[0].title, "HP LaserJet");
        assert_eq!(entries[1].category, "issues");
    }

    #[test]
    fn test_list_knowledge_tree_filter_category() {
        let (_tmp, kdir) = setup();
        std::fs::write(kdir.join("devices/printer.md"), "# Printer").unwrap();
        std::fs::write(kdir.join("issues/bug.md"), "# Bug").unwrap();

        let entries = list_knowledge_tree(&kdir, Some("devices")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Printer");
    }

    #[test]
    fn test_list_knowledge_tree_sets_playbook_type() {
        let (_tmp, kdir) = setup();
        let content = "---
name: Network Diagnostics
description: Diagnose network issues
type: system
---
# Network";
        std::fs::write(kdir.join("playbooks/network.md"), content).unwrap();

        let entries = list_knowledge_tree(&kdir, Some("playbooks")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].playbook_type.as_deref(), Some("system"));
    }

    #[test]
    fn test_knowledge_toc_empty() {
        let (_tmp, kdir) = setup();
        let toc = knowledge_toc(&kdir).unwrap();
        assert!(toc.is_empty());
    }

    #[test]
    fn test_knowledge_toc_with_files() {
        let (_tmp, kdir) = setup();
        std::fs::write(kdir.join("devices/printer.md"), "# HP LaserJet\n\nDetails").unwrap();

        let toc = knowledge_toc(&kdir).unwrap();
        assert!(toc.contains("## Knowledge Base"));
        assert!(toc.contains("### devices"));
        assert!(toc.contains("HP LaserJet"));
        assert!(toc.contains("`devices/printer.md`"));
    }

    #[test]
    fn test_extract_title_from_heading() {
        assert_eq!(
            extract_title("# My Title\n\nContent", "file.md"),
            "My Title"
        );
    }

    #[test]
    fn test_extract_title_from_filename() {
        assert_eq!(
            extract_title("No heading here", "my-cool-file.md"),
            "My cool file"
        );
    }

    #[test]
    fn test_delete_knowledge_file() {
        let (_tmp, kdir) = setup();
        let path = kdir.join("devices/test.md");
        std::fs::write(&path, "content").unwrap();
        assert!(path.exists());

        delete_knowledge_file(&kdir, "devices/test.md").unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_delete_knowledge_file_not_found() {
        let (_tmp, kdir) = setup();
        let result = delete_knowledge_file(&kdir, "devices/nonexistent.md");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_knowledge_tool() {
        let (_tmp, kdir) = setup();
        let tool = WriteKnowledgeTool::new(kdir.clone());

        let input = json!({
            "category": "devices",
            "filename": "hp-printer",
            "content": "# HP Printer\n\nModel: M404n"
        });

        let result = tool.execute(&input).await.unwrap();
        assert!(result.output.contains("devices/hp-printer.md"));

        let content = std::fs::read_to_string(kdir.join("devices/hp-printer.md")).unwrap();
        assert!(content.contains("HP Printer"));
    }

    #[tokio::test]
    async fn test_search_knowledge_tool() {
        let (_tmp, kdir) = setup();
        std::fs::write(
            kdir.join("devices/printer.md"),
            "# HP LaserJet\n\nModel M404n",
        )
        .unwrap();
        std::fs::write(
            kdir.join("network/wifi.md"),
            "# WiFi Config\n\nDNS: 8.8.8.8",
        )
        .unwrap();

        let tool = SearchKnowledgeTool::new(kdir);
        let input = json!({ "query": "DNS" });
        let result = tool.execute(&input).await.unwrap();
        assert!(result.output.contains("1 matching file"));
        assert!(result.output.contains("WiFi Config"));
    }

    #[tokio::test]
    async fn test_search_knowledge_tool_no_results() {
        let (_tmp, kdir) = setup();
        let tool = SearchKnowledgeTool::new(kdir);
        let input = json!({ "query": "nonexistent" });
        let result = tool.execute(&input).await.unwrap();
        assert!(result.output.contains("No knowledge files"));
    }

    #[tokio::test]
    async fn test_read_knowledge_tool() {
        let (_tmp, kdir) = setup();
        std::fs::write(
            kdir.join("devices/printer.md"),
            "# HP LaserJet\n\nDetails here",
        )
        .unwrap();

        let tool = ReadKnowledgeTool::new(kdir);
        let input = json!({ "path": "devices/printer.md" });
        let result = tool.execute(&input).await.unwrap();
        assert!(result.output.contains("HP LaserJet"));
    }

    #[tokio::test]
    async fn test_list_knowledge_tool() {
        let (_tmp, kdir) = setup();
        std::fs::write(kdir.join("devices/printer.md"), "# Printer").unwrap();

        let tool = ListKnowledgeTool::new(kdir);
        let input = json!({});
        let result = tool.execute(&input).await.unwrap();
        assert!(result.output.contains("1 knowledge file"));
        assert!(result.output.contains("Printer"));
    }

    #[test]
    fn test_migrate_artifacts_to_files() {
        let (_tmp, kdir) = setup();
        let conn = crate::safety::journal::init_db(":memory:").unwrap();

        // Seed some artifacts.
        conn.execute(
            "INSERT INTO artifacts (id, category, title, content, source, created_at, updated_at)
             VALUES ('1', 'device_fact', 'HP Printer Model', 'LaserJet Pro M404n', 'agent', '2026-01-01', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO artifacts (id, category, title, content, source, created_at, updated_at)
             VALUES ('2', 'resolved_issue', 'Slow WiFi Fixed', 'Changed DNS to 8.8.8.8', 'agent', '2026-01-02', '2026-01-02')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO artifacts (id, category, title, content, source, created_at, updated_at)
             VALUES ('3', 'preference', 'Prefers Chrome', 'User likes Chrome over Safari', 'agent', '2026-01-03', '2026-01-03')",
            [],
        ).unwrap();

        migrate_artifacts_to_files(&conn, &kdir).unwrap();

        // Check files were created.
        assert!(kdir.join("devices/hp-printer-model.md").exists());
        assert!(kdir.join("issues/slow-wifi-fixed.md").exists());
        assert!(kdir.join("preferences/prefers-chrome.md").exists());

        // Check content.
        let content = std::fs::read_to_string(kdir.join("devices/hp-printer-model.md")).unwrap();
        assert!(content.contains("# HP Printer Model"));
        assert!(content.contains("LaserJet Pro M404n"));
    }
}
