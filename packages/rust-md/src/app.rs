use crate::{
  parser::{self, Link, Metadata},
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
    .route("/test", get(get_test))
    .route("/files", get(get_note_list))
    .route("/files/{*file_path}", get(get_note))
    .with_state(state)
    .layer(cors)
}

/// axum handler for any request that fails to match the router routes.
/// This implementation returns HTTP status code Not Found (404).
pub async fn fallback(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
  (
    axum::http::StatusCode::NOT_FOUND,
    format!("No route {}", uri),
  )
}

#[derive(Serialize, Deserialize)]
struct DefaultResponse {
  status: String,
  code: i32,
  infos: String,
}

async fn get_default() -> Result<Json<DefaultResponse>, String> {
  let default_response = DefaultResponse {
    status: "OK".to_string(),
    code: 200,
    infos: String::from("Welcome to rust-md, you can use: '/test' - '/files' - '/files/file_path'"),
  };
  Ok(Json(default_response))
}

#[derive(Serialize, Deserialize)]
struct TestResponse {
  id: String,
  html: String,
}

// Test using pulldown-cmark
async fn get_test() -> Result<Json<TestResponse>, String> {
  let md=
  "---\ntitle: test\npublic: false\nauthor: me\n---\n# Hello\n\nHere's a [link](https://example.com).\n and an internal [link](./example.md)\n> [!WARNING]\n> Blockquote test";

  let (html_output, _, _) =
    parser::markdown_to_html(&String::from("/"), &String::from("file"), md, false).unwrap();
  println!("\nHTML output:\n{}", html_output);
  let metadata = parser::markdown_to_metadata(md).expect("Failed to extract meta");

  println!("\nMETADATA output:\n{:?}", metadata);

  let test_response = TestResponse {
    id: "hello".to_string(),
    html: html_output,
  };
  Ok(Json(test_response))
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
) -> Result<Json<Note>, String> {
  // ! DEBUG
  let start = std::time::Instant::now();

  let notes_guard = state.notes.lock().await;

  // Keep only the file name
  let file_name = file_path
    .split('/')
    .last()
    .unwrap_or(&file_path)
    .to_string();

  if let Some(note) = notes_guard.get(&file_name) {
    // ! DEBUG

    println!(
      "{} {} /{} in {:?}",
      "[webserver]".cyan(),
      " GET ".on_blue(),
      note.name,
      start.elapsed(),
    );

    // ! HANDLE PRIVATE NOTES
    if !note.public {
      return Err("This file is private".to_string());
    }
    Ok(Json(note.clone()))
  } else {
    Err("Note not found".to_string())
  }
}
