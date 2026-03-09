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

pub fn current_pane_id(session: &str) -> Result<String> {
    tmux(&[
        "display-message",
        "-t",
        &format!("{}:1", session),
        "-p",
        "#{pane_id}",
    ])
}

pub fn select_pane_title(session: &str, pane: &str, title: &str) -> Result<()> {
    let target = pane_target(session, pane);
    tmux(&["select-pane", "-t", &target, "-T", title])?;
    Ok(())
}

pub fn send_keys(session: &str, pane: &str, text: &str) -> Result<()> {
    let target = pane_target(session, pane);
    tmux(&["send-keys", "-t", &target, "-l", text])?;
    tmux(&["send-keys", "-t", &target, "Enter"])?;
    Ok(())
}

pub fn send_ctrl_c(session: &str, pane: &str) -> Result<()> {
    let target = pane_target(session, pane);
    tmux(&["send-keys", "-t", &target, "C-c"])?;
    Ok(())
}

pub fn split_window_horizontal(session: &str, target_pane: &str, cwd: &str) -> Result<String> {
    let target = pane_target(session, target_pane);
    tmux(&[
        "split-window",
        "-h",
        "-P",
        "-F",
        "#{pane_id}",
        "-t",
        &target,
        "-c",
        cwd,
    ])
}

pub fn split_window_vertical(
    session: &str,
    target_pane: &str,
    cwd: &str,
    lines: u32,
) -> Result<String> {
    let target = pane_target(session, target_pane);
    tmux(&[
        "split-window",
        "-v",
        "-P",
        "-F",
        "#{pane_id}",
        "-t",
        &target,
        "-c",
        cwd,
        "-l",
        &lines.to_string(),
    ])
}

pub fn select_layout(session: &str, layout: &str) -> Result<()> {
    let _ = tmux(&["select-layout", "-t", &format!("{}:1", session), layout]);
    Ok(())
}

pub fn select_pane(session: &str, pane: &str) -> Result<()> {
    let target = pane_target(session, pane);
    tmux(&["select-pane", "-t", &target])?;
    Ok(())
}

pub fn open_in_iterm(session: &str) -> Result<()> {
    let attach_command = format!("exec tmux attach -t {}", shell_quote(session));
    let status = Command::new("/usr/bin/osascript")
        .args([
            "-e",
            r#"tell application "iTerm""#,
            "-e",
            r#"activate"#,
            "-e",
            r#"create window with default profile"#,
            "-e",
            &format!(
                r#"tell current session of current window to write text "{}""#,
                applescript_escape(&attach_command)
            ),
            "-e",
            r#"end tell"#,
        ])
        .status()
        .context("Failed to open iTerm")?;
    if !status.success() {
        bail!("Failed to open iTerm");
    }
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
    let target = pane_target(session, pane);
    let _ = tmux(&["kill-pane", "-t", &target]);
    Ok(())
}

pub fn capture_pane(session: &str, pane: &str, lines: u32) -> Result<String> {
    let target = pane_target(session, pane);
    tmux(&[
        "capture-pane",
        "-t",
        &target,
        "-p",
        "-S",
        &format!("-{}", lines),
    ])
}

pub fn list_sessions_raw() -> Result<Vec<String>> {
    match tmux(&["list-sessions", "-F", "#S"]) {
        Ok(out) => Ok(out
            .lines()
            .filter(|l| l.starts_with("cm-") || l.starts_with("ai-"))
            .map(String::from)
            .collect()),
        Err(_) => Ok(vec![]),
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn pane_target(session: &str, pane: &str) -> String {
    if pane.starts_with('%') || pane.contains(':') {
        pane.to_string()
    } else {
        format!("{}:{}", session, pane)
    }
}
