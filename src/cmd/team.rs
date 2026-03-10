use crate::{agent, meta, tmux};
use anyhow::Result;
use clap::Subcommand;
use colored::*;

#[derive(Subcommand)]
pub enum TeamAction {
    /// Start a new team session
    Start {
        /// Project directory (defaults to cwd)
        dir: Option<String>,
        /// Master agent type: claude or codex
        #[arg(short = 't', long = "master-type", default_value = "claude")]
        master_type: String,
        /// Master model
        #[arg(short = 'm', long = "master-model")]
        master_model: Option<String>,
    },
    /// Stop current project's team
    Stop {
        /// Project directory
        dir: Option<String>,
    },
    /// Stop all teams
    StopAll,
    /// List active sessions
    List,
    /// Attach to session
    Attach {
        /// Project directory
        dir: Option<String>,
    },
}

pub fn run(action: TeamAction) -> Result<()> {
    match action {
        TeamAction::Start {
            dir,
            master_type,
            master_model,
        } => cmd_start(dir, master_type, master_model),
        TeamAction::Stop { dir } => cmd_stop(dir),
        TeamAction::StopAll => cmd_stop_all(),
        TeamAction::List => cmd_list(),
        TeamAction::Attach { dir } => cmd_attach(dir),
    }
}

fn cmd_start(dir: Option<String>, master_type: String, master_model: Option<String>) -> Result<()> {
    let project_dir = match dir {
        Some(d) => std::fs::canonicalize(&d)?.to_string_lossy().to_string(),
        None => std::env::current_dir()?.to_string_lossy().to_string(),
    };
    let session = meta::resolve_session_name(&project_dir);

    if tmux::has_session(&session) {
        if meta::load_meta(&session).is_ok() {
            println!(
                "{}",
                format!("Session '{}' already running. Attaching...", session).blue()
            );
            return tmux::attach(&session);
        }

        println!(
            "{}",
            format!(
                "Session '{}' exists but has no recoverable metadata. Recreating...",
                session
            )
            .yellow()
        );
        tmux::kill_session(&session)?;
        let _ = std::fs::remove_dir_all(meta::session_task_dir(&session));
    }

    println!(
        "{} for {}",
        "Starting CrewMux".green().bold(),
        project_dir.cyan()
    );
    println!("Session: {}", session.bold());
    println!();

    // Create directories
    std::fs::create_dir_all(meta::logs_dir())?;
    std::fs::create_dir_all(meta::tasks_dir().join(&session))?;

    // Create tmux session
    tmux::new_session(&session, &project_dir)?;
    tmux::rename_window(&session, "team")?;
    tmux::set_option(&session, "pane-border-format", " #{pane_title} ")?;
    tmux::set_option(&session, "pane-border-status", "top")?;

    // Master pane
    let master_pane_id = tmux::current_pane_id(&session)?;
    tmux::select_pane_title(&session, &master_pane_id, "master")?;

    let master_cmd = agent::build_cli_command(&master_type, &master_model, &project_dir, true)?;
    tmux::send_keys(&session, &master_pane_id, &master_cmd)?;

    // Log pane
    let log_pane_id = tmux::split_window_vertical(&session, &master_pane_id, &project_dir, 6)?;
    tmux::select_pane_title(&session, &log_pane_id, "log")?;

    let log_file = meta::log_path(&session);
    tmux::send_keys(
        &session,
        &log_pane_id,
        &format!(
            "touch '{}' && tail -f '{}'",
            log_file.display(),
            log_file.display()
        ),
    )?;

    // Save metadata
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let team_meta = meta::TeamMeta {
        session: session.clone(),
        project: project_dir,
        started: now,
        master: meta::PaneMeta {
            pane: master_pane_id,
            r#type: Some(master_type.clone()),
            model: master_model.clone(),
        },
        workers: std::collections::HashMap::new(),
        log: meta::PaneMeta {
            pane: log_pane_id,
            r#type: None,
            model: None,
        },
        last_task: None,
        task_count: 0,
    };
    meta::save_meta(&session, &team_meta)?;

    let master_model_suffix = master_model
        .as_ref()
        .map(|model| format!("/{}", model))
        .unwrap_or_default();
    println!(
        "{}",
        format!("Master is ready ({}{}).", master_type, master_model_suffix).green()
    );
    println!();
    println!(
        "  {} \"your task\"    Spawn workers",
        "crewmux task spawn".bold()
    );
    println!(
        "  {}               Check team status",
        "crewmux ctl status".bold()
    );
    println!();

    tmux::select_pane(&session, &team_meta.master.pane)?;
    tmux::attach(&session)?;

    Ok(())
}

fn cmd_stop(dir: Option<String>) -> Result<()> {
    let project_dir = match dir {
        Some(d) => std::fs::canonicalize(&d)?.to_string_lossy().to_string(),
        None => std::env::current_dir()?.to_string_lossy().to_string(),
    };
    let session = meta::resolve_session_name(&project_dir);

    if tmux::has_session(&session) {
        tmux::kill_session(&session)?;
        let _ = std::fs::remove_dir_all(meta::session_task_dir(&session));
        println!("{}", format!("Stopped: {}", session).green());
    } else {
        println!("{}", format!("No active session: {}", session).red());
    }
    Ok(())
}

fn cmd_stop_all() -> Result<()> {
    let sessions = tmux::list_sessions_raw()?;
    if sessions.is_empty() {
        println!("{}", "No active CrewMux sessions.".red());
        return Ok(());
    }
    for s in sessions {
        tmux::kill_session(&s)?;
        let _ = std::fs::remove_dir_all(meta::session_task_dir(&s));
        println!("{}", format!("Stopped: {}", s).green());
    }
    Ok(())
}

fn cmd_list() -> Result<()> {
    let sessions = tmux::list_sessions_raw()?;
    if sessions.is_empty() {
        println!("{}", "No active CrewMux sessions.".red());
        return Ok(());
    }
    println!("{}", "Active CrewMux Sessions:".bold());
    println!();
    for s in sessions {
        if let Ok(m) = meta::load_meta(&s) {
            let wc = m.workers.len();
            println!("  {} -> {}  [master + {} workers]", s.cyan(), m.project, wc);
        } else {
            println!("  {}", s.cyan());
        }
    }
    Ok(())
}

fn cmd_attach(dir: Option<String>) -> Result<()> {
    let project_dir = match dir {
        Some(d) => std::fs::canonicalize(&d)?.to_string_lossy().to_string(),
        None => std::env::current_dir()?.to_string_lossy().to_string(),
    };
    let session = meta::resolve_session_name(&project_dir);

    if tmux::has_session(&session) {
        tmux::attach(&session)?;
    } else {
        println!(
            "{}",
            "No active session. Run 'crewmux team start' first.".red()
        );
    }
    Ok(())
}
