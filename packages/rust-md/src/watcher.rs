use crate::{
  app::{Note, NoteData},
  config::UserConfig,
  parser,
};
use anyhow::Result;
use colored::Colorize;
use notify::{Event, RecursiveMode, Watcher};
use std::{
  collections::HashMap,
  fs,
  path::Path,
  sync::{mpsc, Arc},
};
use tokio::sync::Mutex;

pub async fn watch_files(
  root_path: &str,
  notes: Arc<Mutex<HashMap<String, Note>>>,
  config: &UserConfig,
) -> Result<()> {
  let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

  // Use recommended_watcher() to automatically select the best implementation
  // for your platform. The `EventHandler` passed to this constructor can be a
  // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
  // another type the trait is implemented for.
  let mut watcher = notify::recommended_watcher(tx)?;

  // Add a path to be watched. All files and directories at that path and
  // below will be monitored for changes.
  watcher.watch(Path::new(root_path), RecursiveMode::Recursive)?;
  // Block forever, printing out events as they come in
  for res in rx {
    match res {
      Ok(event) => {
        // println!("event: {:?}", event);
        if let notify::Event {
          kind: notify::EventKind::Modify(notify::event::ModifyKind::Any),
          paths,
          ..
        } = &event
        {
          for path in paths {
            // Check if the path contain a directory that is in the ignored list
            if check_ignore_patterns(path, config) {
              continue; // Skip
            }

            if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
              println!(
                "{} » Updating: {}",
                "[watcher]".purple(),
                format!("{}", path.display()).bold()
              );

              let content = fs::read_to_string(path)?;
              let relative_path = path.strip_prefix(root_path)?;
              let file_path = relative_path.to_string_lossy().replace('\\', "/");
              let file_name = relative_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap()
                .replace(" ", "%20");

              let (html_output, metadata, links) =
                parser::markdown_to_html(&file_path, &file_name, &content, false, config)?;

              let converted_note = Note {
                public: metadata.public.unwrap_or(false),
                name: file_name.clone(),
                slug: file_name.replace(" ", "%20"),
                path: file_path.clone(),
                data: NoteData { metadata, links },
                content: html_output,
              };

              let mut notes_guard = notes.lock().await;
              notes_guard.insert(file_path, converted_note);
            }
          }
        }
      }
      Err(e) => println!("watch error: {:?}", e),
    }
  }

  Ok(())
}

fn check_ignore_patterns(path: &Path, config: &UserConfig) -> bool {
  if let Some(dir_name) = path.parent() {
    // println!("dir_name: {}", dir_name.display());
    let dir_name = dir_name.to_string_lossy().to_string();
    for ignore_pattern in &config.ignore {
      if dir_name.contains(ignore_pattern) {
        println!(
          "{} × Ignoring: {}",
          "[watcher]".purple(),
          format!("{}", path.display()).bright_black().bold()
        );
        return true; // Skip this folder
      }
    }
  }
  false // Do not skip this folder
}
