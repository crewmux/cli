use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::PathBuf;

const LABEL: &str = "com.ai-ctl.web";

fn plist_path() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", LABEL))
}

fn ai_binary_path() -> Result<PathBuf> {
    std::env::current_exe().context("Cannot determine binary path")
}

fn log_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".ai-team/service")
}

pub fn install() -> Result<()> {
    let bin = ai_binary_path()?;
    let plist = plist_path();
    let logs = log_dir();

    fs::create_dir_all(&logs)?;
    fs::create_dir_all(plist.parent().unwrap())?;

    let content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>web</string>
        <string>--port</string>
        <string>7700</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{logs}/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{logs}/stderr.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin:{home}/.local/bin:{home}/.cargo/bin</string>
    </dict>
</dict>
</plist>"#,
        label = LABEL,
        bin = bin.display(),
        logs = logs.display(),
        home = dirs::home_dir().unwrap().display(),
    );

    fs::write(&plist, content)?;

    // Unload first if already loaded
    let _ = std::process::Command::new("launchctl")
        .args(["unload", &plist.to_string_lossy()])
        .output();

    let status = std::process::Command::new("launchctl")
        .args(["load", &plist.to_string_lossy()])
        .status()
        .context("Failed to run launchctl")?;

    if status.success() {
        println!("{}", "AI Team Web UI installed as background service.".green().bold());
        println!();
        println!("  Dashboard:  {}", "http://localhost:7700".cyan());
        println!("  Service:    {}", LABEL.dimmed());
        println!("  Plist:      {}", plist.display().to_string().dimmed());
        println!("  Logs:       {}", logs.display().to_string().dimmed());
        println!();
        println!("  Starts automatically on login.");
        println!("  Uninstall:  {}", "ai uninstall".bold());
    } else {
        println!("{}", "Failed to load service.".red());
    }

    Ok(())
}

pub fn uninstall() -> Result<()> {
    let plist = plist_path();

    if !plist.exists() {
        println!("{}", "Service not installed.".yellow());
        return Ok(());
    }

    let _ = std::process::Command::new("launchctl")
        .args(["unload", &plist.to_string_lossy()])
        .status();

    fs::remove_file(&plist)?;

    println!("{}", "AI Team Web UI service uninstalled.".green());
    println!("  Dashboard will no longer start automatically.");

    Ok(())
}
