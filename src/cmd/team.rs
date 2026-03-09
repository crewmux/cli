use crate::{meta, tmux};
use anyhow::Result;
use clap::Subcommand;
use colored::*;

#[derive(Subcommand)]
pub enum TeamAction {
    /// Start a new team session
    Start {
        /// Project directory (defaults to cwd)
        dir: Option<String>,
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
        TeamAction::Start { dir } => cmd_start(dir),
        TeamAction::Stop { dir } => cmd_stop(dir),
        TeamAction::StopAll => cmd_stop_all(),
        TeamAction::List => cmd_list(),
        TeamAction::Attach { dir } => cmd_attach(dir),
    }
}

fn cmd_start(dir: Option<String>) -> Result<()> {
    let project_dir = match dir {
        Some(d) => std::fs::canonicalize(&d)?.to_string_lossy().to_string(),
        None => std::env::current_dir()?.to_string_lossy().to_string(),
    };
    let session = meta::session_name(&project_dir);

    if tmux::has_session(&session) {
        println!("{}", format!("Session '{}' already running. Attaching...", session).blue());
        return tmux::attach(&session);
    }

    println!(
        "{} for {}",
        "Starting AI Team".green().bold(),
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
    let master_pane = tmux::current_pane_index(&session)?;
    let master_pane_id = format!("1.{}", master_pane);
    tmux::select_pane_title(&session, &master_pane_id, "master")?;

    // Check for master prompt
    let master_prompt_path = meta::team_dir().join("master-prompt.md");
    let claude_cmd = if master_prompt_path.exists() {
        format!(
            "claude --disallowedTools Agent,TeamCreate,TeamDelete,SendMessage --append-system-prompt \"$(cat {})\"",
            master_prompt_path.display()
        )
    } else {
        "claude".to_string()
    };
    tmux::send_keys(&session, &master_pane_id, &claude_cmd)?;

    // Log pane
    tmux::split_window_vertical(&session, &master_pane_id, &project_dir, 6)?;
    let log_pane = tmux::current_pane_index(&session)?;
    let log_pane_id = format!("1.{}", log_pane);
    tmux::select_pane_title(&session, &log_pane_id, "log")?;

    let log_file = meta::log_path(&session);
    tmux::send_keys(
        &session,
        &log_pane_id,
        &format!("touch '{}' && tail -f '{}'", log_file.display(), log_file.display()),
    )?;

    // Save metadata
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let team_meta = meta::TeamMeta {
        session: session.clone(),
        project: project_dir,
        started: now,
        master: meta::PaneMeta {
            pane: master_pane_id,
            r#type: Some("claude".into()),
        },
        workers: std::collections::HashMap::new(),
        log: meta::PaneMeta {
            pane: log_pane_id,
            r#type: None,
        },
        last_task: None,
        task_count: 0,
    };
    meta::save_meta(&session, &team_meta)?;

    println!("{}", "Master is ready.".green());
    println!();
    println!("  {} \"your task\"    Spawn workers", "ai task spawn".bold());
    println!("  {}               Check team status", "ai ctl status".bold());
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
    let session = meta::session_name(&project_dir);

    if tmux::has_session(&session) {
        tmux::kill_session(&session)?;
        let _ = std::fs::remove_dir_all(meta::tasks_dir().join(&session));
        println!("{}", format!("Stopped: {}", session).green());
    } else {
        println!("{}", format!("No active session: {}", session).red());
    }
    Ok(())
}

fn cmd_stop_all() -> Result<()> {
    let sessions = tmux::list_sessions_raw()?;
    if sessions.is_empty() {
        println!("{}", "No active AI team sessions.".red());
        return Ok(());
    }
    for s in sessions {
        tmux::kill_session(&s)?;
        let _ = std::fs::remove_dir_all(meta::tasks_dir().join(&s));
        println!("{}", format!("Stopped: {}", s).green());
    }
    Ok(())
}

fn cmd_list() -> Result<()> {
    let sessions = tmux::list_sessions_raw()?;
    if sessions.is_empty() {
        println!("{}", "No active AI team sessions.".red());
        return Ok(());
    }
    println!("{}", "Active AI Teams:".bold());
    println!();
    for s in sessions {
        if let Ok(m) = meta::load_meta(&s) {
            let wc = m.workers.len();
            println!(
                "  {} -> {}  [master + {} workers]",
                s.cyan(),
                m.project,
                wc
            );
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
    let session = meta::session_name(&project_dir);

    if tmux::has_session(&session) {
        tmux::attach(&session)?;
    } else {
        println!("{}", "No active session. Run 'ai team start' first.".red());
    }
    Ok(())
}
