use pulldown_cmark::{html::push_html, CowStr, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::{
  collections::HashMap,
  fs,
  path::{Component, Path, PathBuf},
};

use crate::config::UserConfig;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
  pub r#type: Option<String>,
  pub public: Option<bool>,
  pub title: Option<String>,
  pub created: Option<String>,
  pub updated: Option<String>,
  pub aliases: Option<Vec<String>>,
  pub tags: Option<Vec<String>>,
  pub summary: Option<String>,
  // Catch-all for unknown fields
  #[serde(flatten)]
  pub extra: Option<HashMap<String, serde_yaml_ng::Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Link {
  pub source: String,
  pub target: String,
  pub target_path: String,
  pub target_public: Option<bool>,
}

/// Transform markdown to html and extract links
pub fn markdown_to_html(
  file_path: &str,
  file_name: &str,
  source_markdown: &str,
  _private_links: bool,
  config: &UserConfig,
) -> anyhow::Result<(String, Metadata, Vec<Link>)> {
  // Extract metadata
  let metadata = markdown_to_metadata(source_markdown)?;

  // Parser options
  let mut options = Options::empty();
  options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
  options.insert(Options::ENABLE_STRIKETHROUGH);
  // Disabled since it only enable blockquote but doesn't support blockquote title and as limited types of callouts
  // options.insert(Options::ENABLE_GFM);

  // File links
  let mut links: Vec<Link> = Vec::new();

  // used to check if we're in a link tag (inside the parser loop)
  let mut in_link_tag = false; // used to check if we're in a link tag

  // Parse the Markdown content
  let parser = Parser::new_ext(source_markdown, options).map(|event| match event {
    Event::Start(tag) => match tag {
      Tag::Link {
        id,
        link_type,
        dest_url,
        title,
      } => {
        if !dest_url.starts_with("http") && dest_url.ends_with(".md") {
          // Used to rewrite link text
          in_link_tag = true;

          let stripped_url = dest_url.trim_end_matches(".md").to_string();
          let dest_file_path = resolve_relative_path(file_path, &dest_url)
            .to_string_lossy()
            .replace('\\', "/");

          // Check if the file is a Markdown file
          let dest_content = fs::read_to_string(&dest_file_path)
            .map_err(|err| format!("Failed to read file '{}': {}", dest_file_path, err))
            .unwrap_or("".to_string());

          // Extract metadata
          let dest_metadata = markdown_to_metadata(&dest_content).unwrap();
          let dest_public = dest_metadata.public.unwrap_or(false);
          let dest_name = Path::new(&stripped_url)
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .replace(" ", "%20");

          links.push(Link {
            source: file_name.to_owned(),
            target: dest_name,
            target_path: dest_file_path,
            target_public: Some(dest_public),
          });

          if !dest_public && !config.private.include {
            return Event::Start(Tag::Link {
              id,
              link_type,
              dest_url: CowStr::from("#"),
              title: CowStr::from("private file"),
            });
          }
          return Event::Start(Tag::Link {
            id,
            link_type,
            dest_url: CowStr::from(stripped_url),
            title,
          });
        }
        Event::Start(Tag::Link {
          id,
          link_type,
          dest_url,
          title,
        })
      }
      _ => Event::Start(tag),
    },
    Event::End(tag) => match tag {
      TagEnd::Link {} => {
        in_link_tag = false;
        Event::End(tag)
      }
      _ => Event::End(tag),
    },
    Event::Text(s) => {
      // If the last link was to a private file,
      // prepend a lock icon to the text.
      if in_link_tag
        && !config.private.include
        && !links
          .last()
          .map_or(false, |link| link.target_public.unwrap_or(false))
      {
        return Event::Text(CowStr::from(format!("{} {}", config.private.icon, s)));
      }

      Event::Text(s)
    }
    _ => event,
  });
  // .map(|event| {
  //   match &event {
  //     Event::Start(tag) => println!("Start: {:?}", tag),
  //     Event::End(tag) => println!("End: {:?}", tag),
  //     Event::Html(s) => println!("Html: {:?}", s),
  //     Event::InlineHtml(s) => println!("InlineHtml: {:?}", s),
  //     Event::Text(s) => println!("Text: {:?}", s),
  //     Event::Code(s) => println!("Code: {:?}", s),
  //     Event::DisplayMath(s) => println!("DisplayMath: {:?}", s),
  //     Event::InlineMath(s) => println!("Math: {:?}", s),
  //     Event::FootnoteReference(s) => println!("FootnoteReference: {:?}", s),
  //     Event::TaskListMarker(b) => println!("TaskListMarker: {:?}", b),
  //     Event::SoftBreak => println!("SoftBreak"),
  //     Event::HardBreak => println!("HardBreak"),
  //     Event::Rule => println!("Rule"),
  //   };
  //   event
  // });

  // Write to a new String buffer.
  let mut html_output = String::new();
  push_html(&mut html_output, parser);

  Ok((html_output, metadata, links))
}

fn resolve_relative_path(base_file: &str, relative_path: &str) -> PathBuf {
  // Normalize paths to forward slashes
  let base_path = PathBuf::from(base_file.replace('\\', "/"));
  let relative_path = relative_path.replace('\\', "/");

  // Get the parent directory of the base file
  let mut parent_dir = base_path.clone();
  parent_dir.pop(); // Remove filename to get parent directory

  // Start building the result path
  let mut result = parent_dir;

  // Process each component of the relative path
  for component in Path::new(&relative_path).components() {
    match component {
      std::path::Component::ParentDir => {
        result.pop(); // Move up one directory level
      }
      std::path::Component::Normal(part) => {
        result.push(part); // Add path component
      }
      _ => {} // Ignore current directory (.) and prefix components
    }
  }

  // Ensure consistent forward slashes in output
  result
}

fn _normalize_combined_path(base: &str, relative: &str) -> PathBuf {
  // Create absolute base path
  let base_path = Path::new("/").join(base.strip_prefix('/').unwrap_or(base));

  // Get parent directory of base path
  let base_parent = base_path.parent().unwrap_or_else(|| Path::new("/"));

  // Join with relative path and normalize
  let mut normalized = PathBuf::new();
  for component in base_parent.join(relative).components() {
    match component {
      Component::Prefix(_) | Component::RootDir => {
        normalized.push("/");
      }
      Component::CurDir => {} // Ignore ./
      Component::ParentDir => {
        // Only pop if we're not at root
        if normalized.components().count() > 1 {
          normalized.pop();
        }
      }
      Component::Normal(name) => {
        normalized.push(name);
      }
    }
  }

  // Ensure .md extension and clean path
  normalized.with_extension("md")
}

fn _clean_path(path: PathBuf) -> String {
  let mut cleaned = path
    .to_string_lossy()
    .replace('\\', "/")
    .replace("/./", "/") // Remove any ./ references
    .replace("//", "/"); // Remove duplicate slashes

  // Ensure path starts with /
  if !cleaned.starts_with('/') {
    cleaned.insert(0, '/');
  }

  String::from(cleaned.trim_end_matches(".md"))
}

/// Extract the metadata aka frontmatter of a markdown file
pub fn markdown_to_metadata(
  source_markdown: &str,
) -> anyhow::Result<Metadata, serde_yaml_ng::Error> {
  // Default metadata
  let default_metadata = Metadata {
    r#type: None,
    public: Some(false),
    title: None,
    created: None,
    updated: None,
    aliases: None,
    tags: None,
    summary: None,
    extra: None,
  };

  // Trim leading whitespace
  let trimmed = source_markdown.trim_start();

  // Check if the document starts with "---"
  if !trimmed.starts_with("---") {
    return Ok(default_metadata);
  }

  // Find the end of the front matter (second "---")
  let after_first_delim = &trimmed[3..]; // Skip first "---"
  if let Some(end_pos) = after_first_delim.find("---") {
    let metadata_raw = &after_first_delim[..end_pos].trim();

    serde_yaml_ng::from_str(metadata_raw)
  } else {
    Ok(default_metadata)
  }
}
