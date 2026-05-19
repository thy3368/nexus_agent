use chrono::Utc;
use std::path::{Path, PathBuf};

fn sanitize(value: &str) -> String {
    let s: String = value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '-',
        })
        .collect();
    let trimmed = s.trim_matches('-');
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn build_log_prefix(kind: &str, name: &str, session_id: &str) -> String {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    format!("{}-{}-{}-{}", sanitize(kind), sanitize(name), session_id, timestamp)
}

pub fn log_file_path(dir: &Path, prefix: &str, event: &str) -> PathBuf {
    dir.join(format!("{}-{}.json", prefix, event))
}

pub async fn write_json_log(path: PathBuf, payload: serde_json::Value) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let content = serde_json::to_string_pretty(&payload)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}
