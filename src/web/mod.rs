use axum::{
    body::Body,
    extract::Query,
    http::{
        header::{self, HeaderValue},
        HeaderMap, StatusCode,
    },
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

use crate::{agent, meta, tmux};

#[derive(RustEmbed)]
#[folder = "assets/brand"]
struct BrandAssets;

pub async fn serve(port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .route("/brand/:asset", get(brand_asset))
        // Session management
        .route("/api/sessions", get(api_sessions))
        .route("/api/sessions/create", post(api_create_session))
        .route("/api/sessions/:session", delete(api_stop_session))
        .route("/api/sessions/stop-all", post(api_stop_all))
        // Status & peek
        .route("/api/status/:session", get(api_status))
        .route("/api/peek/:session/:target", get(api_peek))
        // Agent control
        .route("/api/send", post(api_send))
        .route("/api/spawn", post(api_spawn))
        .route("/api/interrupt", post(api_interrupt))
        .route("/api/kill-workers", post(api_kill_workers))
        .route("/api/kill-agent", post(api_kill_agent))
        .route("/api/open-terminal", post(api_open_terminal))
        // Directory browser
        .route("/api/browse", get(api_browse))
        .route("/api/recents", get(api_recents));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    eprintln!("CrewMux dashboard: http://localhost:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<String> {
    Html(include_str!("../../static/index.html").to_string())
}

async fn brand_asset(
    axum::extract::Path(asset): axum::extract::Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let file = BrandAssets::get(&asset).ok_or(StatusCode::NOT_FOUND)?;
    let mime = mime_guess::from_path(&asset).first_or_octet_stream();

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok((headers, Body::from(file.data.into_owned())))
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
    master_type: Option<String>,
    master_model: Option<String>,
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
struct OpenTerminalRequest {
    session: String,
    target: Option<String>,
}

#[derive(Deserialize)]
struct PeekQuery {
    lines: Option<u32>,
}

#[derive(Deserialize)]
struct BrowseQuery {
    path: Option<String>,
}

#[derive(Serialize)]
struct BrowseResult {
    current: String,
    parent: Option<String>,
    dirs: Vec<DirEntry>,
    is_git: bool,
}

#[derive(Serialize)]
struct DirEntry {
    name: String,
    path: String,
    is_git: bool,
}

#[derive(Serialize)]
struct ApiResult {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<String>,
}

type ApiError = (StatusCode, String);

fn err500(e: impl std::fmt::Display) -> ApiError {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn err404(e: impl std::fmt::Display) -> ApiError {
    (StatusCode::NOT_FOUND, e.to_string())
}

fn err400(e: impl std::fmt::Display) -> ApiError {
    (StatusCode::BAD_REQUEST, e.to_string())
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

    sessions.sort_by(|a, b| a.name.cmp(&b.name));

    Json(sessions)
}

async fn api_create_session(
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<ApiResult>, ApiError> {
    let project_dir = std::fs::canonicalize(&req.project_dir)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid path: {}", e)))?
        .to_string_lossy()
        .to_string();
    let master_type = req.master_type.clone().unwrap_or_else(|| "claude".into());
    let master_model = req.master_model.clone();

    let session = meta::resolve_session_name(&project_dir);

    if tmux::has_session(&session) {
        return Ok(Json(ApiResult {
            ok: true,
            message: format!("Session '{}' already exists", session),
            session: Some(session),
        }));
    }

    // Create directories
    std::fs::create_dir_all(meta::logs_dir()).map_err(err500)?;
    std::fs::create_dir_all(meta::session_task_dir(&session)).map_err(err500)?;

    // Create tmux session
    tmux::new_session(&session, &project_dir).map_err(err500)?;
    tmux::rename_window(&session, "team").map_err(err500)?;
    tmux::set_option(&session, "pane-border-format", " #{pane_title} ").map_err(err500)?;
    tmux::set_option(&session, "pane-border-status", "top").map_err(err500)?;

    // Master pane
    let master_pane_id = tmux::current_pane_id(&session).map_err(err500)?;
    tmux::select_pane_title(&session, &master_pane_id, "master").map_err(err500)?;

    let master_cmd = agent::build_cli_command(&master_type, &master_model, &project_dir, true)
        .map_err(err400)?;
    tmux::send_keys(&session, &master_pane_id, &master_cmd).map_err(err500)?;

    // Log pane
    let log_pane_id =
        tmux::split_window_vertical(&session, &master_pane_id, &project_dir, 6).map_err(err500)?;
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
            r#type: Some(master_type),
            model: master_model,
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
    meta::save_meta(&session, &team_meta).map_err(err500)?;

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Session '{}' created", session),
        session: Some(session),
    }))
}

async fn api_stop_session(
    axum::extract::Path(session): axum::extract::Path<String>,
) -> Result<Json<ApiResult>, ApiError> {
    if tmux::has_session(&session) {
        tmux::kill_session(&session).map_err(err500)?;
    }
    let _ = std::fs::remove_dir_all(meta::session_task_dir(&session));

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Session '{}' stopped", session),
        session: None,
    }))
}

async fn api_stop_all() -> Result<Json<ApiResult>, ApiError> {
    let sessions = tmux::list_sessions_raw().unwrap_or_default();
    for s in &sessions {
        let _ = tmux::kill_session(s);
        let _ = std::fs::remove_dir_all(meta::session_task_dir(s));
    }
    Ok(Json(ApiResult {
        ok: true,
        message: format!("{} session(s) stopped", sessions.len()),
        session: None,
    }))
}

// --- Status & peek ---

async fn api_status(
    axum::extract::Path(session): axum::extract::Path<String>,
) -> Result<Json<StatusResponse>, ApiError> {
    let m = meta::load_meta(&session).map_err(err404)?;

    let mut workers: Vec<AgentInfo> = m
        .workers
        .iter()
        .map(|(name, w)| AgentInfo {
            name: name.clone(),
            agent_type: w.r#type.clone(),
            model: w.model.clone(),
            pane: w.pane.clone(),
        })
        .collect();
    workers.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Json(StatusResponse {
        session: m.session,
        project: m.project,
        started: m.started,
        last_task: m.last_task,
        task_count: m.task_count,
        master: AgentInfo {
            name: "master".into(),
            agent_type: m.master.r#type.unwrap_or("claude".into()),
            model: m.master.model,
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

    let pane =
        meta::resolve_pane(&m, &target).ok_or((StatusCode::NOT_FOUND, "Unknown target".into()))?;

    tmux::capture_pane(&session, &pane, lines).map_err(err500)
}

// --- Agent control ---

async fn api_send(Json(req): Json<SendRequest>) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;

    let pane = meta::resolve_pane(&m, &req.target)
        .ok_or((StatusCode::NOT_FOUND, "Unknown target".into()))?;

    tmux::send_keys(&req.session, &pane, &req.message).map_err(err500)?;
    let _ = meta::append_log(
        &req.session,
        &format!("WEB [{}] {}", req.target, req.message),
    );

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Sent to {}", req.target),
        session: None,
    }))
}

async fn api_spawn(Json(req): Json<SpawnRequest>) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;
    let project_dir = m.project.clone();
    let count = req.count.unwrap_or(1);
    let has_task = !req.task.trim().is_empty();

    let _ = meta::append_log(
        &req.session,
        &if has_task {
            format!("WEB SPAWN [{} x{}] {}", req.worker_type, count, req.task)
        } else {
            format!("WEB SPAWN [{} x{}] (idle)", req.worker_type, count)
        },
    );

    for _ in 0..count {
        let m = meta::load_meta(&req.session).map_err(err500)?;
        let wname = meta::next_worker_name(&m, &req.worker_type);

        let cmd = agent::build_cli_command(&req.worker_type, &req.model, &project_dir, false)
            .map_err(err400)?;

        let wpane_id = tmux::split_window_horizontal(&req.session, &m.master.pane, &project_dir)
            .map_err(err500)?;

        tmux::select_pane_title(&req.session, &wpane_id, &wname).map_err(err500)?;
        tmux::send_keys(&req.session, &wpane_id, &cmd).map_err(err500)?;

        let mut m = meta::load_meta(&req.session).map_err(err500)?;
        m.workers.insert(
            wname.clone(),
            meta::WorkerMeta {
                pane: wpane_id.clone(),
                r#type: req.worker_type.clone(),
                model: req.model.clone(),
            },
        );
        if has_task {
            m.task_count += 1;
            m.last_task = Some(req.task.clone());
        }
        meta::save_meta(&req.session, &m).map_err(err500)?;

        // Wait for CLI boot, then dispatch
        if has_task {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            tmux::send_keys(&req.session, &wpane_id, &req.task).map_err(err500)?;
        }

        let _ = meta::append_log(
            &req.session,
            &if has_task {
                format!("SPAWN [{}] {}", wname, req.task)
            } else {
                format!("SPAWN [{}] (idle)", wname)
            },
        );
    }

    // Re-tile
    if let Ok(m) = meta::load_meta(&req.session) {
        let layout = if m.workers.len() <= 2 {
            "main-vertical"
        } else {
            "tiled"
        };
        tmux::select_layout(&req.session, layout).ok();
    }

    Ok(Json(ApiResult {
        ok: true,
        message: if has_task {
            format!(
                "{} {} worker(s) spawned and dispatched",
                count, req.worker_type
            )
        } else {
            format!("{} {} worker(s) spawned", count, req.worker_type)
        },
        session: None,
    }))
}

async fn api_interrupt(Json(req): Json<InterruptRequest>) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;

    if req.target == "all" {
        tmux::send_ctrl_c(&req.session, &m.master.pane).ok();
        for w in m.workers.values() {
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
        session: None,
    }))
}

async fn api_kill_workers(
    Json(req): Json<KillWorkersRequest>,
) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;

    for w in m.workers.values() {
        tmux::kill_pane(&req.session, &w.pane).ok();
    }

    let mut m = meta::load_meta(&req.session).map_err(err500)?;
    m.workers.clear();
    meta::save_meta(&req.session, &m).map_err(err500)?;

    Ok(Json(ApiResult {
        ok: true,
        message: "All workers killed".into(),
        session: None,
    }))
}

async fn api_kill_agent(Json(req): Json<KillAgentRequest>) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;

    if req.target == "master" || req.target == "log" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Only worker agents can be killed".into(),
        ));
    }

    let (worker_name, worker) = meta::resolve_worker(&m, &req.target)
        .ok_or((StatusCode::NOT_FOUND, "Unknown worker".into()))?;

    tmux::kill_pane(&req.session, &worker.pane).ok();

    let mut m = meta::load_meta(&req.session).map_err(err500)?;
    m.workers.remove(&worker_name);
    meta::save_meta(&req.session, &m).map_err(err500)?;

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Killed {}", worker_name),
        session: None,
    }))
}

async fn api_open_terminal(
    Json(req): Json<OpenTerminalRequest>,
) -> Result<Json<ApiResult>, ApiError> {
    let m = meta::load_meta(&req.session).map_err(err404)?;
    let target = req.target.unwrap_or_else(|| "master".into());
    let pane =
        meta::resolve_pane(&m, &target).ok_or((StatusCode::NOT_FOUND, "Unknown target".into()))?;

    tmux::select_pane(&req.session, &pane).map_err(err500)?;
    tmux::open_in_iterm(&req.session).map_err(err500)?;

    Ok(Json(ApiResult {
        ok: true,
        message: format!("Opened {} in iTerm", target),
        session: None,
    }))
}

// --- Directory browser ---

async fn api_browse(Query(q): Query<BrowseQuery>) -> Result<Json<BrowseResult>, ApiError> {
    let home = dirs::home_dir().unwrap().to_string_lossy().to_string();
    let path = q.path.unwrap_or_else(|| home.clone());

    let canonical = std::fs::canonicalize(&path)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid path: {}", e)))?;
    let current = canonical.to_string_lossy().to_string();

    let parent = canonical.parent().map(|p| p.to_string_lossy().to_string());

    let is_git = canonical.join(".git").is_dir();

    let mut dirs = vec![];
    if let Ok(entries) = std::fs::read_dir(&canonical) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    // Skip hidden dirs except common ones
                    if name.starts_with('.') {
                        continue;
                    }
                    let full = entry.path().to_string_lossy().to_string();
                    let entry_is_git = entry.path().join(".git").is_dir();
                    dirs.push(DirEntry {
                        name,
                        path: full,
                        is_git: entry_is_git,
                    });
                }
            }
        }
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(Json(BrowseResult {
        current,
        parent,
        dirs,
        is_git,
    }))
}

async fn api_recents() -> Json<Vec<String>> {
    // Return project dirs from previous sessions
    let mut recents = vec![];
    if let Ok(entries) = meta::list_sessions() {
        for (_, team) in entries {
            if let Some(m) = team {
                if std::path::Path::new(&m.project).exists() && !recents.contains(&m.project) {
                    recents.push(m.project);
                }
            }
        }
    }
    recents.sort();
    Json(recents)
}
