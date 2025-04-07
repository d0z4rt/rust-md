use std::{env, net::SocketAddr, process};

mod app;

/// ! NEED TO REWRITE EVERYTHING AND THINK OF THE LOGIC BEFORE....
#[tokio::main]
async fn main() {
  // env variables
  dotenvy::dotenv().ok();

  let root_path = env::var("ROOT_PATH").expect("ROOT_PATH is not set in .env file");

  // Validate the root path
  if !std::path::Path::new(&root_path).is_dir() {
    eprintln!("Root path '{}' is not a valid directory", root_path);
    process::exit(1);
  }

  let app = app::create().await;

  let addr = SocketAddr::from(([127, 0, 0, 1], 4000));

  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  println!("listening on {}", listener.local_addr().unwrap());

  axum::serve(listener, app).await.unwrap();
}
