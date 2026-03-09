use anyhow::{bail, Context, Result};
use std::process::Command;

fn tmux(args: &[&str]) -> Result<String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .context("Failed to run tmux. Is it installed?")?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        bail!("tmux error: {}", err.trim());
    }
}

pub fn has_session(session: &str) -> bool {
    tmux(&["has-session", "-t", session]).is_ok()
}

pub fn new_session(session: &str, cwd: &str) -> Result<()> {
    tmux(&[
        "new-session",
        "-d",
        "-s",
        session,
        "-c",
        cwd,
        "-x",
        "200",
        "-y",
        "50",
    ])?;
    Ok(())
}

pub fn rename_window(session: &str, name: &str) -> Result<()> {
    tmux(&["rename-window", "-t", &format!("{}:1", session), name])?;
    Ok(())
}

pub fn set_option(session: &str, key: &str, value: &str) -> Result<()> {
    tmux(&["set", "-t", session, key, value])?;
    Ok(())
}

pub fn current_pane_index(session: &str) -> Result<String> {
    tmux(&[
        "display-message",
        "-t",
        &format!("{}:1", session),
        "-p",
        "#{pane_index}",
    ])
}

pub fn select_pane_title(session: &str, pane: &str, title: &str) -> Result<()> {
    tmux(&[
        "select-pane",
        "-t",
        &format!("{}:{}", session, pane),
        "-T",
        title,
    ])?;
    Ok(())
}

pub fn send_keys(session: &str, pane: &str, text: &str) -> Result<()> {
    tmux(&[
        "send-keys",
        "-t",
        &format!("{}:{}", session, pane),
        "-l",
        text,
    ])?;
    tmux(&[
        "send-keys",
        "-t",
        &format!("{}:{}", session, pane),
        "Enter",
    ])?;
    Ok(())
}

pub fn send_ctrl_c(session: &str, pane: &str) -> Result<()> {
    tmux(&[
        "send-keys",
        "-t",
        &format!("{}:{}", session, pane),
        "C-c",
    ])?;
    Ok(())
}

pub fn split_window_horizontal(session: &str, target_pane: &str, cwd: &str) -> Result<()> {
    tmux(&[
        "split-window",
        "-h",
        "-t",
        &format!("{}:{}", session, target_pane),
        "-c",
        cwd,
    ])?;
    Ok(())
}

pub fn split_window_vertical(
    session: &str,
    target_pane: &str,
    cwd: &str,
    lines: u32,
) -> Result<()> {
    tmux(&[
        "split-window",
        "-v",
        "-t",
        &format!("{}:{}", session, target_pane),
        "-c",
        cwd,
        "-l",
        &lines.to_string(),
    ])?;
    Ok(())
}

pub fn select_layout(session: &str, layout: &str) -> Result<()> {
    let _ = tmux(&["select-layout", "-t", &format!("{}:1", session), layout]);
    Ok(())
}

pub fn select_pane(session: &str, pane: &str) -> Result<()> {
    tmux(&[
        "select-pane",
        "-t",
        &format!("{}:{}", session, pane),
    ])?;
    Ok(())
}

pub fn attach(session: &str) -> Result<()> {
    // attach needs to replace the current process
    let status = Command::new("tmux")
        .args(["attach", "-t", session])
        .status()
        .context("Failed to attach to tmux session")?;
    if !status.success() {
        bail!("Failed to attach to session {}", session);
    }
    Ok(())
}

pub fn kill_session(session: &str) -> Result<()> {
    tmux(&["kill-session", "-t", session])?;
    Ok(())
}

pub fn kill_pane(session: &str, pane: &str) -> Result<()> {
    let _ = tmux(&["kill-pane", "-t", &format!("{}:{}", session, pane)]);
    Ok(())
}

pub fn capture_pane(session: &str, pane: &str, lines: u32) -> Result<String> {
    tmux(&[
        "capture-pane",
        "-t",
        &format!("{}:{}", session, pane),
        "-p",
        "-S",
        &format!("-{}", lines),
    ])
}

pub fn list_sessions_raw() -> Result<Vec<String>> {
    match tmux(&["list-sessions", "-F", "#S"]) {
        Ok(out) => Ok(out
            .lines()
            .filter(|l| l.starts_with("ai-"))
            .map(String::from)
            .collect()),
        Err(_) => Ok(vec![]),
    }
}
