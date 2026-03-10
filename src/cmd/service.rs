use anyhow::{ensure, Context, Result};
use colored::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const LABEL: &str = "com.crewmux.web";

fn plist_path() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", LABEL))
}

fn binary_path() -> Result<PathBuf> {
    std::env::current_exe().context("Cannot determine binary path")
}

fn log_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".crewmux/service")
}

fn launchctl_domain() -> Result<String> {
    if let Ok(uid) = std::env::var("UID") {
        let uid = uid.trim();
        if !uid.is_empty() {
            return Ok(format!("gui/{}", uid));
        }
    }

    let output = Command::new("id")
        .arg("-u")
        .output()
        .context("Failed to determine current UID")?;
    ensure!(output.status.success(), "Failed to determine current UID");

    let uid = String::from_utf8(output.stdout).context("Current UID is not valid UTF-8")?;
    let uid = uid.trim();
    ensure!(!uid.is_empty(), "Current UID is empty");

    Ok(format!("gui/{}", uid))
}

pub fn install() -> Result<()> {
    let bin = binary_path()?;
    let plist = plist_path();
    let logs = log_dir();
    let domain = launchctl_domain()?;

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

    // Remove any previous instance before loading the new plist.
    let _ = Command::new("launchctl")
        .args(["bootout", &domain, &plist.to_string_lossy()])
        .output();

    let status = Command::new("launchctl")
        .args(["bootstrap", &domain, &plist.to_string_lossy()])
        .status()
        .context("Failed to run launchctl")?;

    if status.success() {
        println!(
            "{}",
            "CrewMux dashboard installed as background service."
                .green()
                .bold()
        );
        println!();
        println!("  Dashboard:  {}", "http://localhost:7700".cyan());
        println!("  Service:    {}", LABEL.dimmed());
        println!("  Plist:      {}", plist.display().to_string().dimmed());
        println!("  Logs:       {}", logs.display().to_string().dimmed());
        println!();
        println!("  Starts automatically on login.");
        println!("  Uninstall:  {}", "crewmux uninstall".bold());
    } else {
        println!("{}", "Failed to load service.".red());
    }

    Ok(())
}

pub fn uninstall() -> Result<()> {
    let plist = plist_path();
    let domain = launchctl_domain()?;

    if !plist.exists() {
        println!("{}", "Service not installed.".yellow());
        return Ok(());
    }

    let _ = Command::new("launchctl")
        .args(["bootout", &domain, &plist.to_string_lossy()])
        .status();

    fs::remove_file(&plist)?;

    println!("{}", "CrewMux service uninstalled.".green());
    println!("  Dashboard will no longer start automatically.");

    Ok(())
}
