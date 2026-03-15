use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse},
    routing::{delete, get, patch, post},
};
use serde::Deserialize;

use crate::storage::{JsonStorage, Storage};
use crate::task::Task;

static TODO_HTML: &str = include_str!("todo.html");

type SharedStorage = Arc<Mutex<JsonStorage>>;

#[derive(Clone)]
struct AppState {
    storage: SharedStorage,
}

pub async fn run(port: u16) -> Result<()> {
    let storage = Arc::new(Mutex::new(JsonStorage::default()));
    let state = AppState { storage };

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(|| async { "ok" }))
        .route("/tasks", post(add_task))
        .route("/tasks/{id}/complete", patch(complete_task))
        .route("/tasks/{id}", delete(delete_task))
        .with_state(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr = format!("{host}:{port}");
    println!("[todo] listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index(State(s): State<AppState>) -> impl IntoResponse {
    let tasks = s.storage.lock().unwrap().load().unwrap_or_default();
    let tasks_json = serde_json::to_string(&tasks).unwrap_or_else(|_| "[]".to_string());
    let html = TODO_HTML.replace("__TASKS_JSON__", &tasks_json);
    Html(html)
}

#[derive(Deserialize)]
struct AddTask {
    text: String,
}

async fn add_task(
    State(s): State<AppState>,
    axum::Form(body): axum::Form<AddTask>,
) -> impl IntoResponse {
    if body.text.trim().is_empty() {
        return redirect("/");
    }
    let storage = s.storage.lock().unwrap();
    let mut tasks = storage.load().unwrap_or_default();
    let id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let text = body.text.trim().to_string();
    tasks.push(Task::new(id, text.clone()));
    storage.save(&tasks).ok();
    drop(storage);
    crate::hooks::run("on_add", &[&id.to_string(), &text]).ok();
    redirect("/")
}

async fn complete_task(
    State(s): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let storage = s.storage.lock().unwrap();
    let mut tasks = storage.load().unwrap_or_default();
    let text = if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
        t.done = true;
        t.text.clone()
    } else {
        return (StatusCode::NOT_FOUND, HeaderMap::new(), "not found".to_string())
            .into_response();
    };
    storage.save(&tasks).ok();
    drop(storage);
    crate::hooks::run("on_complete", &[&id.to_string(), &text]).ok();
    redirect("/")
}

async fn delete_task(
    State(s): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let storage = s.storage.lock().unwrap();
    let mut tasks = storage.load().unwrap_or_default();
    if let Some(pos) = tasks.iter().position(|t| t.id == id) {
        let text = tasks[pos].text.clone();
        tasks.remove(pos);
        storage.save(&tasks).ok();
        drop(storage);
        crate::hooks::run("on_delete", &[&id.to_string(), &text]).ok();
    }
    redirect("/")
}

fn redirect(to: &str) -> axum::response::Response {
    let mut headers = HeaderMap::new();
    headers.insert(header::LOCATION, to.parse().unwrap());
    (StatusCode::SEE_OTHER, headers, "").into_response()
}
