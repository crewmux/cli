use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::{meta, tmux};

pub async fn serve(port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        // Session management
        .route("/api/sessions", get(api_sessions))
        .route("/api/sessions/create", post(api_create_session))
        .route("/api/sessions/{session}", delete(api_stop_session))
        .route("/api/sessions/stop-all", post(api_stop_all))
        // Status & peek
        .route("/api/status/{session}", get(api_status))
        .route("/api/peek/{session}/{target}", get(api_peek))
        // Agent control
        .route("/api/send", post(api_send))
        .route("/api/spawn", post(api_spawn))
        .route("/api/interrupt", post(api_interrupt))
        .route("/api/kill-workers", post(api_kill_workers))
        .route("/api/kill-agent", post(api_kill_agent));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    eprintln!("AI Team Web UI: http://localhost:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<String> {
    Html(include_str!("../../static/index.html").to_string())
}

// --- API types ---

#[derive(Serialize)]
struct SessionInfo {
    name: String,
    project: Option<String>,
    worker_count: usize,
    started: Option<String>,
    active: bool,
}

#[derive(Serialize)]
struct StatusResponse {
    session: String,
    project: String,
    started: String,
    last_task: Option<String>,
    task_count: u32,
    master: AgentInfo,
    workers: Vec<AgentInfo>,
}

#[derive(Serialize)]
struct AgentInfo {
    name: String,
    agent_type: String,
    model: Option<String>,
    pane: String,
}

#[derive(Deserialize)]
struct CreateSessionRequest {
    project_dir: String,
}

#[derive(Deserialize)]
struct SendRequest {
    session: String,
    target: String,
    message: String,
}

#[derive(Deserialize)]
struct SpawnRequest {
    session: String,
    worker_type: String,
    model: Option<String>,
    count: Option<u32>,
    task: String,
}

#[derive(Deserialize)]
struct InterruptRequest {
    session: String,
    target: String,
}

#[derive(Deserialize)]
struct KillWorkersRequest {
    session: String,
}

#[derive(Deserialize)]
struct KillAgentRequest {
    session: String,
    target: String,
}

#[derive(Deserialize)]
struct PeekQuery {
    lines: Option<u32>,
}

#[derive(Serialize)]
struct ApiResult {
    ok: bool,
    message: String,
}

type ApiError = (StatusCode, String);

fn err500(e: impl std::fmt::Display) -> ApiError {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn err404(e: impl std::fmt::Display) -> ApiError {
    (StatusCode::NOT_FOUND, e.to_string())
}

// --- Session management ---

async fn api_sessions() -> Json<Vec<SessionInfo>> {
    let live = tmux::list_sessions_raw().unwrap_or_default();
    let mut sessions = vec![];

    for name in &live {
        let info = if let Ok(m) = meta::load_meta(name) {
            SessionInfo {
                name: name.clone(),
                project: Some(m.project),
                worker_count: m.workers.len(),
                started: Some(m.started),
                active: true,
            }
        } else {
            SessionInfo {
                name: name.clone(),
                project: None,
                worker_count: 0,
                started: None,
                active: true,
            }
        };
        sessions.push(info);
    }

    Json(sessions)
}

async fn api_create_session(
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<ApiResult>, ApiError> {
    let project_dir = std::fs::canonicalize(&req.project_dir)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid path: {}", e)))?
        .to_string_lossy()
        .to_string();

    let session = meta::session_name(&project_dir);

    if tmux::has_session(&session) {
        return Ok(Json(ApiResult {
            ok: true,
            message: format!("Session '{}' already exists", session),
        }));
    }

    // Create directories
    std::fs::create_dir_all(meta::logs_dir()).map_err(err500)?;
    std::fs::create_dir_all(meta::tasks_dir().join(&session)).map_err(err500)?;

    // Create tmux session
    tmux::new_session(&session, &project_dir).map_err(err500)?;
    tmux::rename_window(&session, "team").map_err(err500)?;
    tmux::set_option(&session, "pane-border-format", " #{pane_title} ").map_err(err500)?;
    tmux::set_option(&session, "pane-border-status", "top").map_err(err500)?;

    // Master pane
    let master_pane = tmux::current_pane_index(&session).map_err(err500)?;
    let master_pane_id = format!("1.{}", master_pane);
    tmux::select_pane_title(&session, &master_pane_id, "master").map_err(err500)?;

    // Launch claude on master
    let master_prompt_path = meta::team_dir().join("master-prompt.md");
    let claude_cmd = if master_prompt_path.exists() {
        format!(
            "claude --disallowedTools Agent,TeamCreate,TeamDelete,SendMessage --append-system-prompt \"$(cat {})\"",
            master_prompt_path.display()
        )
    } else {
        "claude".to_string()
    };
    tmux::send_keys(&session, &master_pane_id, &claude_cmd).map_err(err500)?;

    // Log pane
    tmux::split_window_vertical(&session, &master_pane_id, &project_dir, 6).map_err(err500)?;
    let log_pane = tmux::current_pane_index(&session).map_err(err500)?;
    let log_pane_id = format!("1.{}", log_pane);
    tmux::select_pane_title(&session, &log_pane_id, "log").map_err(err500)?;

    let log_file = meta::log_path(&session);
    tmux::send_keys(
        &session,
        &log_pane_id,
        &format!(
            "touch '{}' && tail -f '{}'",
            log_file.display(),
            log_file.display()
        ),
    )
    .map_err(err500)?;

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
    meta::save_meta(&session, &team_meta).map_err(err500)?;

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Session '{}' created", session),
    }))
}

async fn api_stop_session(
    axum::extract::Path(session): axum::extract::Path<String>,
) -> Result<Json<ApiResult>, ApiError> {
    if tmux::has_session(&session) {
        tmux::kill_session(&session).map_err(err500)?;
    }
    let _ = std::fs::remove_dir_all(meta::tasks_dir().join(&session));

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Session '{}' stopped", session),
    }))
}

async fn api_stop_all() -> Result<Json<ApiResult>, ApiError> {
    let sessions = tmux::list_sessions_raw().unwrap_or_default();
    for s in &sessions {
        let _ = tmux::kill_session(s);
        let _ = std::fs::remove_dir_all(meta::tasks_dir().join(s));
    }
    Ok(Json(ApiResult {
        ok: true,
        message: format!("{} session(s) stopped", sessions.len()),
    }))
}

// --- Status & peek ---

async fn api_status(
    axum::extract::Path(session): axum::extract::Path<String>,
) -> Result<Json<StatusResponse>, ApiError> {
    let m = meta::load_meta(&session).map_err(err404)?;

    let workers: Vec<AgentInfo> = m
        .workers
        .iter()
        .map(|(name, w)| AgentInfo {
            name: name.clone(),
            agent_type: w.r#type.clone(),
            model: w.model.clone(),
            pane: w.pane.clone(),
        })
        .collect();

    Ok(Json(StatusResponse {
        session: m.session,
        project: m.project,
        started: m.started,
        last_task: m.last_task,
        task_count: m.task_count,
        master: AgentInfo {
            name: "master".into(),
            agent_type: m.master.r#type.unwrap_or("claude".into()),
            model: None,
            pane: m.master.pane,
        },
        workers,
    }))
}

async fn api_peek(
    axum::extract::Path((session, target)): axum::extract::Path<(String, String)>,
    Query(query): Query<PeekQuery>,
) -> Result<String, ApiError> {
    let m = meta::load_meta(&session).map_err(err404)?;
    let lines = query.lines.unwrap_or(80);

    let pane = meta::resolve_pane(&m, &target).ok_or((StatusCode::NOT_FOUND, "Unknown target".into()))?;

    tmux::capture_pane(&session, &pane, lines).map_err(err500)
}

// --- Agent control ---

async fn api_send(Json(req): Json<SendRequest>) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;

    let pane = meta::resolve_pane(&m, &req.target)
        .ok_or((StatusCode::NOT_FOUND, "Unknown target".into()))?;

    tmux::send_keys(&req.session, &pane, &req.message).map_err(err500)?;
    let _ = meta::append_log(&req.session, &format!("WEB [{}] {}", req.target, req.message));

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Sent to {}", req.target),
    }))
}

async fn api_spawn(
    Json(req): Json<SpawnRequest>,
) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;
    let project_dir = m.project.clone();
    let count = req.count.unwrap_or(1);

    let _ = meta::append_log(
        &req.session,
        &format!("WEB SPAWN [{} x{}] {}", req.worker_type, count, req.task),
    );

    for _ in 0..count {
        let m = meta::load_meta(&req.session).map_err(err500)?;
        let wname = meta::next_worker_name(&m, &req.worker_type);

        let cmd = match req.worker_type.as_str() {
            "claude" => {
                let mut c = "claude".to_string();
                if let Some(ref model) = req.model {
                    c.push_str(&format!(" --model {}", model));
                }
                c
            }
            "codex" => {
                let mut c = "codex".to_string();
                if let Some(ref model) = req.model {
                    c.push_str(&format!(" -m {}", model));
                }
                c
            }
            _ => req.worker_type.clone(),
        };

        tmux::split_window_horizontal(&req.session, &m.master.pane, &project_dir).map_err(err500)?;
        let wpane = tmux::current_pane_index(&req.session).map_err(err500)?;
        let wpane_id = format!("1.{}", wpane);

        tmux::select_pane_title(&req.session, &wpane_id, &wname).ok();
        tmux::send_keys(&req.session, &wpane_id, &cmd).ok();

        let mut m = meta::load_meta(&req.session).map_err(err500)?;
        m.workers.insert(
            wname.clone(),
            meta::WorkerMeta {
                pane: wpane_id.clone(),
                r#type: req.worker_type.clone(),
                model: req.model.clone(),
            },
        );
        m.task_count += 1;
        m.last_task = Some(req.task.clone());
        meta::save_meta(&req.session, &m).map_err(err500)?;

        // Wait for CLI boot, then dispatch
        if !req.task.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            tmux::send_keys(&req.session, &wpane_id, &req.task).ok();
        }

        let _ = meta::append_log(&req.session, &format!("SPAWN [{}] {}", wname, req.task));
    }

    // Re-tile
    if let Ok(m) = meta::load_meta(&req.session) {
        let layout = if m.workers.len() <= 2 { "main-vertical" } else { "tiled" };
        tmux::select_layout(&req.session, layout).ok();
    }

    Ok(Json(ApiResult {
        ok: true,
        message: format!("{} {} worker(s) spawned", count, req.worker_type),
    }))
}

async fn api_interrupt(
    Json(req): Json<InterruptRequest>,
) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;

    if req.target == "all" {
        for (_, w) in &m.workers {
            tmux::send_ctrl_c(&req.session, &w.pane).ok();
        }
    } else {
        let pane = meta::resolve_pane(&m, &req.target)
            .ok_or((StatusCode::NOT_FOUND, "Unknown target".into()))?;
        tmux::send_ctrl_c(&req.session, &pane).map_err(err500)?;
    }

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Interrupted {}", req.target),
    }))
}

async fn api_kill_workers(
    Json(req): Json<KillWorkersRequest>,
) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;

    for (_, w) in &m.workers {
        tmux::kill_pane(&req.session, &w.pane).ok();
    }

    let mut m = meta::load_meta(&req.session).map_err(err500)?;
    m.workers.clear();
    meta::save_meta(&req.session, &m).map_err(err500)?;

    Ok(Json(ApiResult {
        ok: true,
        message: "All workers killed".into(),
    }))
}

async fn api_kill_agent(
    Json(req): Json<KillAgentRequest>,
) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;

    if req.target == "master" {
        return Err((StatusCode::BAD_REQUEST, "Cannot kill master".into()));
    }

    let pane = meta::resolve_pane(&m, &req.target)
        .ok_or((StatusCode::NOT_FOUND, "Unknown target".into()))?;

    tmux::kill_pane(&req.session, &pane).ok();

    let mut m = meta::load_meta(&req.session).map_err(err500)?;
    m.workers.remove(&req.target);
    meta::save_meta(&req.session, &m).map_err(err500)?;

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Killed {}", req.target),
    }))
}
