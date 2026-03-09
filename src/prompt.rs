use crate::meta;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MASTER_PROMPT: &str = include_str!("../assets/master-prompt.md");
const LEGACY_MASTER_PROMPT_PREFIX: &str =
    "You are the master coordinator of an AI team. You ANALYZE and DELEGATE. You do NOT implement.";

pub fn master_prompt_path() -> PathBuf {
    meta::team_dir().join("master-prompt.md")
}

pub fn ensure_default_master_prompt() -> Result<PathBuf> {
    let path = master_prompt_path();
    let legacy_path = meta::legacy_team_dir().join("master-prompt.md");

    if !path.exists() && legacy_path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&legacy_path, &path)?;
    }

    let needs_bootstrap = match fs::read_to_string(&path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                true
            } else if trimmed.starts_with(LEGACY_MASTER_PROMPT_PREFIX) {
                backup_legacy_prompt(&path, &content)?;
                true
            } else {
                false
            }
        }
        Err(_) => true,
    };

    if needs_bootstrap {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, DEFAULT_MASTER_PROMPT)?;
    }

    Ok(path)
}

fn backup_legacy_prompt(path: &Path, content: &str) -> Result<()> {
    let backup = path.with_extension("legacy.bak");
    if !backup.exists() {
        fs::write(backup, content)?;
    }
    Ok(())
}
