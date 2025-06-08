use crate::{
  app::{Note, NoteData},
  config::UserConfig,
  parser,
};
use colored::Colorize;
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

/// Recursively find all notes in the directory and its subdirectories
pub async fn find_all_notes(
  dir: &PathBuf,
  notes: Arc<Mutex<HashMap<String, Note>>>,
  config: &UserConfig,
) -> Result<(), String> {
  let entries = fs::read_dir(dir)
    .map_err(|err| format!("Failed to read directory '{}': {}", dir.display(), err))?;

  for entry in entries {
    let entry = entry.map_err(|err| format!("Failed to read directory entry: {}", err))?;

    let path = entry.path();

    // * If the entry is a directory, recursively search it
    if path.is_dir() {
      // Check if the directory is in the ignored list, if so skip it
      if let Some(dir_name) = path.file_name() {
        let dir_name = dir_name.to_string_lossy().to_string();
        if config.ignore.contains(&dir_name) {
          println!(
            "{} {}",
            "× Ignoring folder:".bright_black(),
            format!("{}", path.display()).bright_black().bold()
          );
          continue; // Skip this folder
        }
      }

      println!(
        "{} {}",
        "» Scanning folder:".bright_black(),
        format!("{}", path.display()).bright_black().bold()
      );

      // Recursively search subdirectories
      let recursive_call = find_all_notes(&path, notes.clone(), config);
      Box::pin(recursive_call).await?;

    // *  If the entry is a file, process it
    } else if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md") {
      // Check if the file is a Markdown file
      let content = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read file '{}': {}", path.display(), err))?;

      let relative_path = path
        .strip_prefix(&config.root_path)
        .map_err(|err| format!("Failed to get relative path: {}", err))?;

      // Get clean path components
      let path_str = relative_path
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/");
      let absolute_path = format!("/{}", path_str);
      let full_path = path.to_str().unwrap_or("");

      let file_name = relative_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .replace(" ", "%20");

      // Parse Markdown content and extract links
      let (html_output, metadata, note_links) = parser::markdown_to_html(
        full_path, &file_name, &content, false, config,
      )
      .map_err(|err| {
        format!(
          "Failed to convert to markdown '{}': {}",
          path.display(),
          err
        )
      })?;

      // Store the converted note in the HashMap
      let converted_note = Note {
        public: metadata.public.unwrap_or(false),
        name: file_name.clone(),
        slug: file_name.replace(" ", "%20"),
        path: absolute_path,
        data: NoteData {
          metadata,
          links: note_links,
        },
        content: html_output,
      };

      let mut notes_guard = notes.lock().await;
      notes_guard.insert(path_str, converted_note);
    }
  }

  Ok(())
}
