use crate::prompt;
use anyhow::{bail, Result};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const TRUSTED_LEVEL: &str = r#"trust_level = "trusted""#;

pub fn build_cli_command(
    agent_type: &str,
    model: &Option<String>,
    project_dir: &str,
    use_master_prompt: bool,
) -> Result<String> {
    if agent_type == "claude" {
        ensure_claude_project_trusted(project_dir)?;
    }
    if agent_type == "codex" {
        ensure_codex_project_trusted(project_dir)?;
    }

    let command = match agent_type {
        "claude" => build_claude_command(model, use_master_prompt)?,
        "codex" => build_codex_command(model, use_master_prompt)?,
        other => bail!("Unsupported agent type: {}", other),
    };

    Ok(command)
}

fn build_claude_command(model: &Option<String>, use_master_prompt: bool) -> Result<String> {
    let mut command = "claude".to_string();
    if let Some(model_name) = model.as_deref() {
        command.push_str(" --model ");
        command.push_str(&shell_quote(model_name));
    }

    command.push_str(" --allowedTools ");
    command.push_str(&shell_quote("Bash(*),Edit,MultiEdit,Write,NotebookEdit"));

    if use_master_prompt {
        let master_prompt_path = prompt::ensure_default_master_prompt()?;
        command.push_str(" --disallowedTools Agent,TeamCreate,TeamDelete,SendMessage");
        command.push_str(" --append-system-prompt ");
        command.push_str(&shell_command_substitution_cat(&master_prompt_path));
    }

    Ok(command)
}

fn build_codex_command(model: &Option<String>, use_master_prompt: bool) -> Result<String> {
    let mut command = "codex -a never -s danger-full-access".to_string();
    if let Some(model_name) = model.as_deref() {
        command.push_str(" -m ");
        command.push_str(&shell_quote(model_name));
    }

    if use_master_prompt {
        let master_prompt_path = prompt::ensure_default_master_prompt()?;
        command.push(' ');
        command.push_str(&shell_command_substitution_cat(&master_prompt_path));
    }

    Ok(command)
}

fn shell_command_substitution_cat(path: &Path) -> String {
    format!("\"$(cat {})\"", shell_quote(&path.to_string_lossy()))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

fn ensure_codex_project_trusted(project_dir: &str) -> Result<()> {
    let config_path = codex_config_path();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let header = project_header(project_dir);
    let content = fs::read_to_string(&config_path).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();

    if let Some(header_index) = lines.iter().position(|line| line.trim() == header) {
        let mut cursor = header_index + 1;
        let mut trust_index = None;

        while cursor < lines.len() && !lines[cursor].trim_start().starts_with('[') {
            if lines[cursor].trim_start().starts_with("trust_level") {
                trust_index = Some(cursor);
                break;
            }
            cursor += 1;
        }

        if let Some(index) = trust_index {
            lines[index] = TRUSTED_LEVEL.to_string();
        } else {
            lines.insert(cursor, TRUSTED_LEVEL.to_string());
        }
    } else {
        if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(header);
        lines.push(TRUSTED_LEVEL.to_string());
    }

    let mut updated = lines.join("\n");
    updated.push('\n');
    fs::write(config_path, updated)?;

    Ok(())
}

fn codex_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot determine home directory. Is $HOME set?")
        .join(".codex/config.toml")
}

fn project_header(project_dir: &str) -> String {
    format!(
        r#"[projects."{}"]"#,
        project_dir.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

fn ensure_claude_project_trusted(project_dir: &str) -> Result<()> {
    let config_path = claude_config_path();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut root = fs::read_to_string(&config_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .unwrap_or_else(|| Value::Object(Map::new()));

    if !root.is_object() {
        root = Value::Object(Map::new());
    }

    let obj = root.as_object_mut().unwrap();
    obj.insert("hasTrustDialogAccepted".into(), Value::Bool(true));
    obj.remove(project_dir);

    let projects = obj
        .entry("projects")
        .or_insert_with(|| Value::Object(Map::new()));
    if !projects.is_object() {
        *projects = Value::Object(Map::new());
    }

    let project_entry = projects
        .as_object_mut()
        .unwrap()
        .entry(project_dir.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !project_entry.is_object() {
        *project_entry = Value::Object(Map::new());
    }

    let project_obj = project_entry.as_object_mut().unwrap();
    project_obj
        .entry("allowedTools")
        .or_insert_with(|| Value::Array(vec![]));
    project_obj
        .entry("mcpContextUris")
        .or_insert_with(|| Value::Array(vec![]));
    project_obj
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()));
    project_obj
        .entry("enabledMcpjsonServers")
        .or_insert_with(|| Value::Array(vec![]));
    project_obj
        .entry("disabledMcpjsonServers")
        .or_insert_with(|| Value::Array(vec![]));
    project_obj.insert("hasTrustDialogAccepted".into(), Value::Bool(true));
    project_obj.insert("hasCompletedProjectOnboarding".into(), Value::Bool(true));

    fs::write(config_path, serde_json::to_string_pretty(&root)?)?;
    Ok(())
}

fn claude_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot determine home directory. Is $HOME set?")
        .join(".claude.json")
}
