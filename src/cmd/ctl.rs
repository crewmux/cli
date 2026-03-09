use crate::{meta, tmux};
use anyhow::{bail, Result};
use clap::Subcommand;
use colored::*;

#[derive(Subcommand)]
pub enum CtlAction {
    /// Show team status overview
    Status,
    /// List all agents
    Roles,
    /// Send message to an agent
    Send {
        /// Agent name (master, worker name)
        target: String,
        /// Message
        message: Vec<String>,
    },
    /// View agent output
    Peek {
        /// Agent name
        #[arg(default_value = "master")]
        target: String,
        /// Number of lines
        #[arg(short, long, default_value = "50")]
        lines: u32,
    },
    /// View or follow task log
    Log {
        /// Follow mode (like tail -f)
        #[arg(short, long)]
        follow: bool,
    },
    /// Send message to all agents
    Broadcast {
        /// Message
        message: Vec<String>,
    },
    /// Send Ctrl+C to agent(s)
    Interrupt {
        /// Agent name or "all"
        #[arg(default_value = "all")]
        target: String,
    },
    /// Kill all worker panes
    KillWorkers,
}

pub fn run(action: CtlAction) -> Result<()> {
    match action {
        CtlAction::Status => cmd_status(),
        CtlAction::Roles => cmd_roles(),
        CtlAction::Send { target, message } => cmd_send(target, message.join(" ")),
        CtlAction::Peek { target, lines } => cmd_peek(target, lines),
        CtlAction::Log { follow } => cmd_log(follow),
        CtlAction::Broadcast { message } => cmd_broadcast(message.join(" ")),
        CtlAction::Interrupt { target } => cmd_interrupt(target),
        CtlAction::KillWorkers => cmd_kill_workers(),
    }
}

fn ensure_session() -> Result<String> {
    let session = meta::session_name_cwd();
    if !tmux::has_session(&session) {
        bail!("No active team session. Run 'ai team start' first.");
    }
    Ok(session)
}

fn cmd_status() -> Result<()> {
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;

    println!("{}", "AI Team Status".bold());
    println!("{}", "─".repeat(50).dimmed());
    println!("  Session:  {}", m.session.cyan());
    println!("  Project:  {}", m.project);
    if let Some(ref task) = m.last_task {
        let display: String = task.chars().take(80).collect();
        println!("  Task:     {}", display);
    }
    println!();

    let master_type = m.master.r#type.as_deref().unwrap_or("claude");
    let master_model = m
        .master
        .model
        .as_ref()
        .map(|model| format!("/{}", model))
        .unwrap_or_default();
    println!(
        "  {} ({}{})",
        "master".blue().bold(),
        master_type,
        master_model
    );

    if m.workers.is_empty() {
        println!();
        println!(
            "  {}",
            "No workers spawned yet. Use 'ai task spawn' to dispatch.".dimmed()
        );
    } else {
        println!();
        for (name, w) in &m.workers {
            let color_name = match w.r#type.as_str() {
                "claude" => name.blue().bold(),
                "codex" => name.cyan().bold(),
                _ => name.normal().bold(),
            };
            let model_str = w
                .model
                .as_ref()
                .map(|m| format!("/{}", m))
                .unwrap_or_default();
            println!("  {} ({}{})", color_name, w.r#type, model_str);
        }
    }

    println!();
    println!("{}", "─".repeat(50).dimmed());
    println!("  {} <name> \"msg\"   Send to agent", "ai ctl send".bold());
    println!("  {} <name>          View output", "ai ctl peek".bold());
    println!("  {}                List all agents", "ai ctl roles".bold());

    Ok(())
}

fn cmd_roles() -> Result<()> {
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;

    println!("{}", "Available agents:".bold());
    println!();
    let master_type = m.master.r#type.as_deref().unwrap_or("claude");
    let master_model = m
        .master
        .model
        .as_ref()
        .map(|model| format!("/{}", model))
        .unwrap_or_default();
    println!("  {}  ({}{})", "master".blue(), master_type, master_model);
    for (name, w) in &m.workers {
        let color_name = match w.r#type.as_str() {
            "claude" => name.blue(),
            "codex" => name.cyan(),
            _ => name.normal(),
        };
        let model_str = w
            .model
            .as_ref()
            .map(|m| format!("/{}", m))
            .unwrap_or_default();
        println!("  {}  ({}{})", color_name, w.r#type, model_str);
    }
    println!("  {}  (task log)", "log".yellow());

    Ok(())
}

fn cmd_send(target: String, message: String) -> Result<()> {
    if message.is_empty() {
        bail!("Usage: ai ctl send <name> \"message\"");
    }
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;

    match meta::resolve_pane(&m, &target) {
        Some(pane) => {
            tmux::send_keys(&session, &pane, &message)?;
            meta::append_log(&session, &format!("REMOTE [{}] {}", target, message))?;
            println!("{}", format!("Sent to {}.", target).green());
        }
        None => bail!("Unknown: {}. Use 'ai ctl roles'.", target),
    }
    Ok(())
}

fn cmd_peek(target: String, lines: u32) -> Result<()> {
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;

    match meta::resolve_pane(&m, &target) {
        Some(pane) => {
            println!("{}", format!("=== {} ===", target).bold());
            println!();
            let output = tmux::capture_pane(&session, &pane, lines)?;
            println!("{}", output);
        }
        None => bail!("Unknown: {}", target),
    }
    Ok(())
}

fn cmd_log(follow: bool) -> Result<()> {
    let session = meta::session_name_cwd();
    let log_file = meta::log_path(&session);

    if !log_file.exists() {
        bail!("No log file.");
    }

    if follow {
        let status = std::process::Command::new("tail")
            .args(["-f", &log_file.to_string_lossy()])
            .status()?;
        if !status.success() {
            bail!("tail failed");
        }
    } else {
        let content = std::fs::read_to_string(&log_file)?;
        print!("{}", content);
    }
    Ok(())
}

fn cmd_broadcast(message: String) -> Result<()> {
    if message.is_empty() {
        bail!("Usage: ai ctl broadcast \"msg\"");
    }
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;

    // Send to master
    tmux::send_keys(&session, &m.master.pane, &message)?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Send to all workers
    for w in m.workers.values() {
        tmux::send_keys(&session, &w.pane, &message)?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    println!("{}", "Broadcast sent.".green());
    Ok(())
}

fn cmd_interrupt(target: String) -> Result<()> {
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;

    if target == "all" {
        tmux::send_ctrl_c(&session, &m.master.pane)?;
        for w in m.workers.values() {
            tmux::send_ctrl_c(&session, &w.pane)?;
        }
    } else {
        match meta::resolve_pane(&m, &target) {
            Some(pane) => tmux::send_ctrl_c(&session, &pane)?,
            None => bail!("Unknown: {}", target),
        }
    }

    println!("{}", format!("Interrupted {}.", target).yellow());
    Ok(())
}

fn cmd_kill_workers() -> Result<()> {
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;

    for w in m.workers.values() {
        tmux::kill_pane(&session, &w.pane)?;
    }

    let mut m = meta::load_meta(&session)?;
    m.workers.clear();
    meta::save_meta(&session, &m)?;

    println!("{}", "All workers killed.".green());
    Ok(())
}
