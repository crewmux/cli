use crate::tmux;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const TEAM_DIR_NAME: &str = ".crewmux";
const LEGACY_TEAM_DIR_NAME: &str = ".ai-team";
const SESSION_PREFIX: &str = "crewmux";
const LEGACY_SESSION_PREFIXES: &[&str] = &["cm", "ai"];

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
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMeta {
    pub pane: String,
    pub r#type: String,
    #[serde(default)]
    pub model: Option<String>,
}

/// ~/.crewmux
pub fn team_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(TEAM_DIR_NAME)
}

pub fn legacy_team_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(LEGACY_TEAM_DIR_NAME)
}

pub fn logs_dir() -> PathBuf {
    team_dir().join("logs")
}

pub fn tasks_dir() -> PathBuf {
    team_dir().join("tasks")
}

pub fn session_task_dir(session: &str) -> PathBuf {
    session_storage_root(session).join("tasks").join(session)
}

pub fn session_name(dir: &str) -> String {
    format!("{}-{}", SESSION_PREFIX, session_basename(dir))
}

pub fn resolve_session_name(dir: &str) -> String {
    for session in session_candidates(dir) {
        if tmux::has_session(&session) {
            return session;
        }
    }
    session_name(dir)
}

pub fn resolve_session_name_cwd() -> String {
    let cwd = std::env::current_dir().unwrap();
    resolve_session_name(&cwd.to_string_lossy())
}

pub fn meta_path(session: &str) -> PathBuf {
    session_task_dir(session).join("meta.json")
}

pub fn log_path(session: &str) -> PathBuf {
    session_storage_root(session)
        .join("logs")
        .join(format!("{}.log", session))
}

fn session_basename(dir: &str) -> String {
    let base = std::path::Path::new(dir)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());
    base.replace([' ', '.'], "-")
}

fn session_candidates(dir: &str) -> Vec<String> {
    let basename = session_basename(dir);
    let mut candidates = Vec::with_capacity(1 + LEGACY_SESSION_PREFIXES.len());
    candidates.push(format!("{}-{}", SESSION_PREFIX, basename));
    candidates.extend(
        LEGACY_SESSION_PREFIXES
            .iter()
            .map(|prefix| format!("{}-{}", prefix, basename)),
    );
    candidates
}

fn session_storage_root(session: &str) -> PathBuf {
    let current = team_dir();
    if current
        .join("tasks")
        .join(session)
        .join("meta.json")
        .exists()
        || current
            .join("logs")
            .join(format!("{}.log", session))
            .exists()
    {
        return current;
    }

    let legacy = legacy_team_dir();
    if legacy
        .join("tasks")
        .join(session)
        .join("meta.json")
        .exists()
        || legacy
            .join("logs")
            .join(format!("{}.log", session))
            .exists()
    {
        return legacy;
    }

    current
}

pub fn load_meta(session: &str) -> Result<TeamMeta> {
    let path = meta_path(session);
    match fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).context("Failed to parse meta.json"),
        Err(err) => {
            if tmux::has_session(session) {
                return recover_meta_from_tmux(session).with_context(|| {
                    format!(
                        "No metadata at {} and failed to recover session metadata",
                        path.display()
                    )
                });
            }

            Err(err).with_context(|| format!("No metadata at {}", path.display()))
        }
    }
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

/// Resolve a worker target to its canonical name and metadata.
/// Returns None when there is no match or the partial match is ambiguous.
pub fn resolve_worker(meta: &TeamMeta, target: &str) -> Option<(String, WorkerMeta)> {
    if let Some((name, worker)) = meta.workers.get_key_value(target) {
        return Some((name.clone(), worker.clone()));
    }

    let mut matches = meta
        .workers
        .iter()
        .filter(|(name, _)| name.contains(target));
    let (name, worker) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }

    Some((name.clone(), worker.clone()))
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
    let mut sessions = vec![];
    let mut seen = std::collections::HashSet::new();

    for dir in [tasks_dir(), legacy_team_dir().join("tasks")] {
        if !dir.exists() {
            continue;
        }

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if seen.insert(name.clone()) {
                    let meta = load_meta(&name).ok();
                    sessions.push((name, meta));
                }
            }
        }
    }

    Ok(sessions)
}

fn recover_meta_from_tmux(session: &str) -> Result<TeamMeta> {
    let panes = tmux::list_panes(session)?;
    if panes.is_empty() {
        bail!("tmux session has no panes");
    }

    let recognized_layout = panes.iter().any(|pane| {
        pane.title == "master"
            || pane.title == "log"
            || pane.title.starts_with("claude-")
            || pane.title.starts_with("codex-")
    });
    if !recognized_layout {
        bail!("tmux session does not look like a CrewMux session");
    }

    let project = panes[0].current_path.clone();
    let master_pane = panes
        .iter()
        .find(|pane| pane.title == "master")
        .cloned()
        .unwrap_or_else(|| panes[0].clone());
    let log_pane = panes
        .iter()
        .find(|pane| pane.title == "log")
        .cloned()
        .or_else(|| panes.iter().find(|pane| pane.id != master_pane.id).cloned())
        .ok_or_else(|| anyhow::anyhow!("tmux session is missing a log pane"))?;

    let mut workers = HashMap::new();
    for pane in panes {
        if pane.id == master_pane.id || pane.id == log_pane.id {
            continue;
        }

        let title = pane.title.trim();
        if title.is_empty() {
            continue;
        }

        let worker_type = title.split('-').next().unwrap_or(title).to_string();
        workers.insert(
            title.to_string(),
            WorkerMeta {
                pane: pane.id,
                r#type: worker_type,
                model: None,
            },
        );
    }

    let meta = TeamMeta {
        session: session.to_string(),
        project,
        started: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        master: PaneMeta {
            pane: master_pane.id,
            r#type: None,
            model: None,
        },
        workers,
        log: PaneMeta {
            pane: log_pane.id,
            r#type: None,
            model: None,
        },
        last_task: None,
        task_count: 0,
    };
    save_meta(session, &meta)?;
    Ok(meta)
}
