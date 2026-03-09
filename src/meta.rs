use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMeta {
    pub session: String,
    pub project: String,
    pub started: String,
    pub master: PaneMeta,
    pub workers: HashMap<String, WorkerMeta>,
    pub log: PaneMeta,
    #[serde(default)]
    pub last_task: Option<String>,
    #[serde(default)]
    pub task_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneMeta {
    pub pane: String,
    #[serde(default)]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMeta {
    pub pane: String,
    pub r#type: String,
    #[serde(default)]
    pub model: Option<String>,
}

/// ~/.ai-team
pub fn team_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".ai-team")
}

pub fn logs_dir() -> PathBuf {
    team_dir().join("logs")
}

pub fn tasks_dir() -> PathBuf {
    team_dir().join("tasks")
}

pub fn session_name(dir: &str) -> String {
    let base = std::path::Path::new(dir)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());
    format!("ai-{}", base.replace([' ', '.'], "-"))
}

pub fn session_name_cwd() -> String {
    let cwd = std::env::current_dir().unwrap();
    session_name(&cwd.to_string_lossy())
}

pub fn meta_path(session: &str) -> PathBuf {
    tasks_dir().join(session).join("meta.json")
}

pub fn log_path(session: &str) -> PathBuf {
    logs_dir().join(format!("{}.log", session))
}

pub fn load_meta(session: &str) -> Result<TeamMeta> {
    let path = meta_path(session);
    let data = fs::read_to_string(&path)
        .with_context(|| format!("No metadata at {}", path.display()))?;
    serde_json::from_str(&data).context("Failed to parse meta.json")
}

pub fn save_meta(session: &str, meta: &TeamMeta) -> Result<()> {
    let path = meta_path(session);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(meta)?;
    fs::write(&path, data)?;
    Ok(())
}

pub fn append_log(session: &str, msg: &str) -> Result<()> {
    use std::io::Write;
    let path = log_path(session);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let ts = chrono::Local::now().format("%H:%M:%S");
    writeln!(file, "[{}] {}", ts, msg)?;
    Ok(())
}

/// Resolve a target name (master, worker name, or partial match) to a pane id
pub fn resolve_pane(meta: &TeamMeta, target: &str) -> Option<String> {
    if target == "master" {
        return Some(meta.master.pane.clone());
    }
    if target == "log" {
        return Some(meta.log.pane.clone());
    }
    if let Some(w) = meta.workers.get(target) {
        return Some(w.pane.clone());
    }
    // partial match
    for (name, w) in &meta.workers {
        if name.contains(target) {
            return Some(w.pane.clone());
        }
    }
    None
}

/// Get next worker name for a type (e.g. claude-1, claude-2, codex-1)
pub fn next_worker_name(meta: &TeamMeta, worker_type: &str) -> String {
    let prefix = format!("{}-", worker_type);
    let max_num = meta
        .workers
        .keys()
        .filter(|k| k.starts_with(&prefix))
        .filter_map(|k| k[prefix.len()..].parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("{}-{}", worker_type, max_num + 1)
}

/// List all sessions (directories in tasks_dir that have meta.json)
pub fn list_sessions() -> Result<Vec<(String, Option<TeamMeta>)>> {
    let dir = tasks_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut sessions = vec![];
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = load_meta(&name).ok();
            sessions.push((name, meta));
        }
    }
    Ok(sessions)
}
