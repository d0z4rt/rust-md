use crate::config::{load_config, UserConfig};
use crate::service::find_all_notes;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::{net::SocketAddr, process};
use tokio::sync::Mutex;

use colored::Colorize;

mod app;
mod config;
mod parser;
mod service;
mod watcher;

const CONFIG_PATH: &str = "./packages/rust-md/config.yaml";

#[derive(Clone)]
struct AppState {
  notes: Arc<Mutex<HashMap<String, app::Note>>>,
  config: UserConfig,
}

/// ! NEED TO REWRITE EVERYTHING AND THINK OF THE LOGIC BEFORE....
#[tokio::main]
async fn main() {
  // ! DEBUG
  let app_start_time = std::time::Instant::now();

  println!(
    "{}{}Starting...\n",
    " rust-md ".on_purple().bold(),
    " v0.1 ".yellow().bold()
  );

  let config = load_config(CONFIG_PATH).unwrap();

  // Validate the root path
  if !std::path::Path::new(&config.root_path).is_dir() {
    eprintln!("Root path '{}' is not a valid directory", config.root_path);
    process::exit(1);
  }

  // Initial setup
  let notes = Arc::new(Mutex::new(HashMap::new()));
  let app_state = AppState {
    notes: notes.clone(),
    config: config.clone(),
  };

  // Perform initial conversion
  let root_path_buf = PathBuf::from(&config.root_path);

  println!("{}", "Indexing all files...".yellow().bold());

  let find_all_notes_start_time = std::time::Instant::now();

  find_all_notes(&root_path_buf, notes.clone(), &config)
    .await
    .map_err(|err| format!("Error while searching for files: {}", err))
    .unwrap();

  println!(
    "Indexed in: {}",
    format!("{:?}", find_all_notes_start_time.elapsed()).bold()
  );

  println!("{}", "Starting file watcher...".yellow().bold());
  // Start watching files for changes in a separate task
  tokio::spawn(async move {
    match watcher::watch_files(&config.root_path, notes.clone(), &config).await {
      Ok(_) => {}
      Err(e) => {
        eprintln!("Error while watching files: {}", e);
      }
    }
  });

  println!("{}", "Starting webserver...".yellow().bold());

  let app = app::create(app_state).await;

  let addr = SocketAddr::from(([127, 0, 0, 1], 4000));

  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

  println!(
    "\n{} {}",
    " rust-md ".on_bright_green().bold(),
    format!("Ready in: {:?}", app_start_time.elapsed())
      .green()
      .bold()
  );
  println!(
    "\n➜  Local: {}\n",
    format!("http://{:?}", listener.local_addr().unwrap()).cyan()
  );

  axum::serve(listener, app).await.unwrap();
}
