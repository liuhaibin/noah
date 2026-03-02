use tauri::State;

use crate::artifacts::{self, Artifact};
use crate::AppState;

#[tauri::command]
pub async fn list_artifacts(
    state: State<'_, AppState>,
    search: Option<String>,
    category: Option<String>,
) -> Result<Vec<Artifact>, String> {
    let conn = state.db.lock().await;
    artifacts::query_artifacts(&conn, search.as_deref(), category.as_deref())
        .map_err(|e| format!("Failed to list artifacts: {}", e))
}

#[tauri::command]
pub async fn delete_artifact(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<(), String> {
    let conn = state.db.lock().await;
    artifacts::delete_artifact(&conn, &artifact_id)
        .map_err(|e| format!("Failed to delete artifact: {}", e))
}
