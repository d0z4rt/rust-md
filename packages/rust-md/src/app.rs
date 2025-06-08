use crate::{
  parser::{Link, Metadata},
  AppState,
};
use axum::{routing::get, Json, Router};
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Note {
  pub public: bool,
  pub name: String,
  pub slug: String,
  pub path: String,
  pub data: NoteData,
  pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NoteData {
  pub metadata: Metadata,
  pub links: Vec<Link>,
}

#[derive(Serialize, Deserialize)]
struct GraphResponse {
  nodes: Vec<NodeInfo>,
  links: Vec<Link>,
}

#[derive(Serialize, Deserialize)]
pub struct NodeInfo {
  pub id: String,
  pub public: bool,
  pub path: String,
  pub r#type: String,
}

pub async fn create(state: AppState) -> Router {
  // See https://docs.rs/tower-http/latest/tower_http/cors/index.html
  let cors = tower_http::cors::CorsLayer::new().allow_origin(tower_http::cors::Any);

  Router::new()
    .fallback(fallback)
    .route("/", get(get_default))
    .route("/files", get(get_note_list))
    .route("/files/{*file_path}", get(get_note))
    .with_state(state)
    .layer(cors)
}

/// axum handler for any request that fails to match the router routes.
/// This implementation returns HTTP status code Not Found (404).
#[derive(Serialize, Deserialize)]
struct ErrorResponse {
  status: String,
  code: u16,
  message: String,
}
async fn fallback(uri: axum::http::Uri) -> Json<ErrorResponse> {
  Json(ErrorResponse {
    status: "NOT_FOUND".to_string(),
    code: axum::http::StatusCode::NOT_FOUND.as_u16(),
    message: format!("No route {}", uri),
  })
}

#[derive(Serialize, Deserialize)]
struct DefaultResponse {
  status: String,
  code: i32,
  message: String,
}

async fn get_default() -> Result<Json<DefaultResponse>, String> {
  let default_response = DefaultResponse {
    status: "OK".to_string(),
    code: 200,
    message: String::from("Welcome to rust-md, you can use: '/files' - '/files/file_path'"),
  };
  Ok(Json(default_response))
}

#[derive(Serialize, Deserialize)]
struct TestResponse {
  id: String,
  html: String,
}

async fn get_note_list(
  axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<Json<GraphResponse>, String> {
  // ! DEBUG
  let start = std::time::Instant::now();

  let notes_guard = state.notes.lock().await;

  let mut nodes = Vec::new();
  let mut links = Vec::new();

  for (file_name, converted_note) in notes_guard.iter() {
    nodes.push(NodeInfo {
      id: file_name.clone(),
      public: converted_note.public,
      path: format!("/{}", file_name.replace("%20", " ")),
      r#type: "note".to_string(),
    });

    // ! Extract links from the note
    links.extend(converted_note.data.links.clone());
  }

  // ! DEBUG
  println!(
    "{} {} /files in {:?}",
    "[webserver]".cyan(),
    " GET ".on_blue(),
    start.elapsed(),
  );

  Ok(Json(GraphResponse { nodes, links }))
}

async fn get_note(
  axum::extract::Path(file_path): axum::extract::Path<String>,
  axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<Json<Note>, Json<ErrorResponse>> {
  // ! DEBUG
  let start = std::time::Instant::now();

  let notes_guard = state.notes.lock().await;

  if let Some(note) = notes_guard.get(&file_path) {
    // ! DEBUG

    println!(
      "{} {} /{} in {:?}",
      "[webserver]".cyan(),
      " GET ".on_blue(),
      note.name,
      start.elapsed(),
    );

    // ! HANDLE PRIVATE NOTES
    if !note.public && !state.config.private.include {
      return Err(Json(ErrorResponse {
        status: "FORBIDDEN".to_string(),
        code: axum::http::StatusCode::FORBIDDEN.as_u16(),
        message: format!("This file is private: {}", file_path),
      }));
    }
    Ok(Json(note.clone()))
  } else {
    Err(Json(ErrorResponse {
      status: "NOT_FOUND".to_string(),
      code: axum::http::StatusCode::NOT_FOUND.as_u16(),
      message: format!("This file does not exist: {}", file_path),
    }))
  }
}
