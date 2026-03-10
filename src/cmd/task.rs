use crate::{agent, meta, tmux};
use anyhow::{bail, Result};
use clap::Subcommand;
use colored::*;

#[derive(Subcommand)]
pub enum TaskAction {
    /// Spawn worker(s) and dispatch a task
    Spawn {
        /// Worker type: claude or codex
        #[arg(short = 't', long, default_value = "claude")]
        r#type: String,

        /// Model to use (e.g. gpt-5.3-codex, gpt-5-codex, sonnet, opus)
        #[arg(short, long)]
        model: Option<String>,

        /// Number of workers to spawn
        #[arg(short = 'n', long, default_value = "1")]
        count: u32,

        /// Task to dispatch
        task: Vec<String>,
    },
    /// Send message to master
    Master {
        /// Message to send
        message: Vec<String>,
    },
    /// Send message to a specific worker
    Send {
        /// Worker name
        target: String,
        /// Message to send
        message: Vec<String>,
    },
    /// Kill all workers
    Clean,
}

pub fn run(action: TaskAction) -> Result<()> {
    match action {
        TaskAction::Spawn {
            r#type,
            model,
            count,
            task,
        } => cmd_spawn(r#type, model, count, task.join(" ")),
        TaskAction::Master { message } => cmd_master(message.join(" ")),
        TaskAction::Send { target, message } => cmd_send(target, message.join(" ")),
        TaskAction::Clean => cmd_clean(),
    }
}

fn ensure_session() -> Result<String> {
    let session = meta::resolve_session_name_cwd();
    if !tmux::has_session(&session) {
        bail!("No active team session. Run 'crewmux team start' first.");
    }
    Ok(session)
}

fn spawn_worker(
    session: &str,
    name: &str,
    worker_type: &str,
    model: &Option<String>,
    project_dir: &str,
) -> Result<String> {
    let m = meta::load_meta(session)?;
    let master_pane = &m.master.pane;

    let wpane_id = tmux::split_window_horizontal(session, master_pane, project_dir)?;

    tmux::select_pane_title(session, &wpane_id, name)?;

    let cmd = agent::build_cli_command(worker_type, model, project_dir, false)?;
    tmux::send_keys(session, &wpane_id, &cmd)?;

    // Re-tile layout
    let worker_count = m.workers.len();
    if worker_count < 2 {
        tmux::select_layout(session, "main-vertical")?;
    } else {
        tmux::select_layout(session, "tiled")?;
    }

    // Update meta
    let mut m = meta::load_meta(session)?;
    m.workers.insert(
        name.to_string(),
        meta::WorkerMeta {
            pane: wpane_id.clone(),
            r#type: worker_type.to_string(),
            model: model.clone(),
        },
    );
    meta::save_meta(session, &m)?;

    Ok(wpane_id)
}

fn cmd_spawn(worker_type: String, model: Option<String>, count: u32, task: String) -> Result<()> {
    if task.is_empty() {
        bail!("Task message is required. Usage: crewmux task spawn -t codex -m gpt-5.3-codex \"your task\"");
    }

    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;
    let project_dir = m.project.clone();

    meta::append_log(
        &session,
        &format!("TASK [{} x{}] {}", worker_type, count, task),
    )?;

    for _ in 0..count {
        let mut m = meta::load_meta(&session)?;
        let wname = meta::next_worker_name(&m, &worker_type);

        let color_name = match worker_type.as_str() {
            "claude" => wname.blue(),
            "codex" => wname.cyan(),
            _ => wname.normal(),
        };
        let model_str = model
            .as_ref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();
        println!("  {} spawning...{}", color_name, model_str.dimmed());

        let wpane = spawn_worker(&session, &wname, &worker_type, &model, &project_dir)?;
        meta::append_log(
            &session,
            &format!("SPAWN {} ({}{})", wname, worker_type, model_str),
        )?;

        // Wait for CLI to boot
        std::thread::sleep(std::time::Duration::from_secs(3));

        tmux::send_keys(&session, &wpane, &task)?;
        meta::append_log(&session, &format!("DISPATCH [{}] {}", wname, task))?;

        // Update task count
        m = meta::load_meta(&session)?;
        m.task_count += 1;
        m.last_task = Some(task.clone());
        meta::save_meta(&session, &m)?;
    }

    println!(
        "{}",
        format!("{} {} worker(s) dispatched.", count, worker_type).green()
    );
    Ok(())
}

fn cmd_master(message: String) -> Result<()> {
    if message.is_empty() {
        bail!("Message is required.");
    }
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;
    tmux::send_keys(&session, &m.master.pane, &message)?;
    meta::append_log(&session, &format!("DIRECT [master] {}", message))?;
    println!("{}", "Sent to master.".blue());
    Ok(())
}

fn cmd_send(target: String, message: String) -> Result<()> {
    if message.is_empty() {
        bail!("Message is required.");
    }
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;
    let pane = meta::resolve_pane(&m, &target);
    match pane {
        Some(p) => {
            tmux::send_keys(&session, &p, &message)?;
            meta::append_log(&session, &format!("DIRECT [{}] {}", target, message))?;
            println!("{}", format!("Sent to {}.", target).green());
        }
        None => bail!("Unknown target: {}. Use 'crewmux ctl roles'.", target),
    }
    Ok(())
}

fn cmd_clean() -> Result<()> {
    let session = ensure_session()?;
    let m = meta::load_meta(&session)?;

    for w in m.workers.values() {
        tmux::kill_pane(&session, &w.pane)?;
    }

    let mut m = meta::load_meta(&session)?;
    m.workers.clear();
    meta::save_meta(&session, &m)?;
    meta::append_log(&session, "CLEANUP workers")?;

    println!("{}", "All workers killed.".green());
    Ok(())
}
