use axum::{routing::get, Json, Router};
use markdown::mdast::Node;
use serde::{Deserialize, Serialize};
use std::{
  collections::{HashSet, VecDeque},
  env, fs,
  path::{PathBuf, MAIN_SEPARATOR},
};

#[derive(Serialize, Deserialize)]
struct Note {
  public: bool,
  name: String,
  slug: String,
  path: String,
  data: NoteData,
  content: String,
}

#[derive(Serialize, Deserialize)]
struct NoteData {
  matter: Matter,
  links: Vec<Link>,
}

#[derive(Serialize, Deserialize)]
struct Matter {
  r#type: String,
  public: Option<bool>,
  created: String,
  updated: String,
  aliases: Vec<String>,
  tags: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Link {
  source: String,
  target: String,
}

#[derive(Serialize, Deserialize)]
struct GraphResponse {
  nodes: Vec<NodeInfo>,
  links: Vec<Link>,
}

#[derive(Serialize, Deserialize)]
struct NodeInfo {
  id: String,
  public: bool,
  path: String,
  r#type: String,
}

pub async fn create() -> Router {
  // See https://docs.rs/tower-http/latest/tower_http/cors/index.html
  let cors = tower_http::cors::CorsLayer::new().allow_origin(tower_http::cors::Any);

  Router::new()
    .fallback(fallback)
    .route("/test", get(get_test))
    .route("/brain", get(get_note_list))
    .route("/brain/{*file_path}", get(get_note))
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
struct TestResponse {
  id: String,
}

// Test modifying inner markdown
async fn get_test() -> Result<Json<TestResponse>, String> {
  let test_response = TestResponse {
    id: "hello".to_string(),
  };
  let md = "# Hello\n\nHere's a [link](https://example.com).\n";
  println!("Original:\n{md}");

  let mut ast = markdown::to_mdast(md, &Default::default()).unwrap();
  println!("\nAST before:\n{ast:#?}");

  let mut queue = VecDeque::from([&mut ast]);
  while let Some(node) = queue.pop_front() {
    match node {
      Node::Link(link) => link.url = "https://modified.com".into(),
      Node::Root(root) => queue.extend(root.children.iter_mut()),
      Node::Paragraph(p) => queue.extend(p.children.iter_mut()),
      _ => {}
    }
  }

  let md = mdast_util_to_markdown::to_markdown(&ast).expect("error when converting MD to AST");

  println!("\nAST after:\n{ast:#?}");
  println!(
    "\nHTML output:\n{}",
    markdown::to_html_with_options(&md, &markdown::Options::default()).unwrap()
  );
  Ok(Json(test_response))
}

async fn get_note_list() -> Result<Json<GraphResponse>, String> {
  // ! DEBUG
  let start = std::time::Instant::now();

  let root_path = env::var("ROOT_PATH").expect("ROOT_PATH is not set in .env file");

  let mut nodes = Vec::new();
  let mut links = Vec::new();
  let mut public_files_number = 0;
  let mut private_files_number = 0;

  // Start recursive traversal from the root path
  let root_path_buf = PathBuf::from(&root_path);

  find_all_notes(
    &root_path_buf,
    &mut nodes,
    &mut links,
    &mut public_files_number,
    &mut private_files_number,
  )
  .map_err(|err| format!("Error while searching for notes: {}", err))?;

  // ! DEBUG
  println!("find_all_notes took: {:?}", start.elapsed());

  // Filter out nodes where public is not true
  nodes.retain(|node| node.public);

  // Collect the IDs of the remaining nodes
  let public_node_ids: HashSet<String> = nodes.iter().map(|node| node.id.clone()).collect();

  // Remove links that reference non-public nodes
  links.retain(|link| {
    public_node_ids.contains(&link.source) && public_node_ids.contains(&link.target)
  });

  // ! DEBUG
  let duration = start.elapsed();
  println!(
    "Function took: {:?} | {} Private files | {} Public files ",
    duration, private_files_number, public_files_number
  );

  Ok(Json(GraphResponse { nodes, links }))
}

async fn get_note(
  axum::extract::Path(file_path): axum::extract::Path<String>,
) -> Result<Json<Note>, String> {
  // ! DEBUG
  let start = std::time::Instant::now();

  let root_path = env::var("ROOT_PATH").expect("ROOT_PATH is not set in .env file");
  let file_disk_path = format!("{}/{}.md", root_path, file_path);
  let content = fs::read_to_string(file_disk_path).map_err(|_| "File not found".to_string())?;

  // Parse the Markdown content
  let parse_options = markdown::ParseOptions {
    constructs: markdown::Constructs {
      frontmatter: true,
      ..markdown::Constructs::gfm()
    },
    ..markdown::ParseOptions::gfm()
  };

  // Split the content to extract frontmatter
  let (frontmatter, content) = extract_frontmatter(&content);

  let matter: Matter =
    serde_yaml_ng::from_str(frontmatter).map_err(|_| "Error parsing frontmatter".to_string())?;

  // Parse the Markdown content into an AST
  let ast = markdown::to_mdast(content, &parse_options).expect("error when converting MD to AST");
  // ? There is no AST -> to html yet and is not planned
  // ? Afaik you can do MDAST -> to HAST then HAST to HTML but for now I will just try to manipulate the html direclty

  // Keep only the file name
  let file_name = file_path
    .split('/')
    .last()
    .unwrap_or(&file_path)
    .to_string();

  // Remove underscore to get the name

  let file_title = file_name.replace("_", " ");

  // Extract links from the AST
  let links = extract_links(&ast, &file_path);

  // Parse the Markdown content into HTML
  let html_content = markdown::to_html_with_options(
    content,
    &markdown::Options {
      parse: parse_options,
      ..markdown::Options::gfm()
    },
  )
  .expect("error when converting MD to HTML");

  // Remove `.md` from links in the HTML
  let html_content = remove_md_from_html_links(&html_content);
  let html_content = rewrite_src_from_html_img(&html_content);
  let html_content = transform_callout(&html_content);

  let note = Note {
    public: matter.public.unwrap_or(false),
    name: file_title.clone(),
    slug: file_name.replace(" ", "%20"),
    path: file_path.clone(),
    data: NoteData { matter, links },
    content: html_content,
  };

  // ! DEBUG
  let duration = start.elapsed();
  println!("Function took: {:?} | {} ", duration, file_title);

  if !note.public {
    return Err("This file is private".to_string());
  }

  Ok(Json(note))
}

/// Recursively find all notes in the directory and its subdirectories
fn find_all_notes(
  dir: &PathBuf,
  nodes: &mut Vec<NodeInfo>,
  links: &mut Vec<Link>,
  public_files_number: &mut i32,
  private_files_number: &mut i32,
) -> Result<(), String> {
  // ! List of folders to ignore
  let ignore_folders = [
    ".obsidian",
    ".mobile",
    ".output",
    ".trash",
    ".trash",
    "templates",
  ];

  // ! DEBUG
  let start = std::time::Instant::now();

  let entries = fs::read_dir(dir)
    .map_err(|err| format!("Failed to read directory '{}': {}", dir.display(), err))?;

  // ! DEBUG
  println!("fs::read_dir took: {:?}", start.elapsed());

  for entry in entries {
    let entry = entry.map_err(|err| format!("Failed to read directory entry: {}", err))?;

    let path = entry.path();

    if path.is_dir() {
      // Skip ignored folders
      if let Some(dir_name) = path.file_name().and_then(|name| name.to_str()) {
        if ignore_folders.contains(&dir_name) {
          println!("Ignoring folder: {}", path.display());
          continue; // Skip this folder
        }
      }

      // Recursively search subdirectories
      find_all_notes(
        &path,
        nodes,
        links,
        public_files_number,
        private_files_number,
      )?;
    } else if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md") {
      // ! DEBUG
      let start_file_read = std::time::Instant::now();

      // Check if the file is a Markdown file
      let content = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read file '{}': {}", path.display(), err))?;

      // ! DEBUG
      println!("fs::read_to_string took: {:?}", start_file_read.elapsed());

      // Extract the frontmatter
      let (frontmatter, content) = extract_frontmatter(&content);

      let mut file_public = false;

      // Parse the frontmatter to check if the note is public
      if let Ok(matter) = serde_yaml_ng::from_str::<Matter>(frontmatter) {
        if matter.public == Some(true) {
          file_public = true;
          *public_files_number += 1;
        } else {
          *private_files_number += 1;
        }
      }

      // Add the node to the list
      let relative_path = path
        .strip_prefix(env::var("ROOT_PATH").unwrap())
        .map_err(|err| {
          format!(
            "Failed to strip prefix from path '{}': {}",
            path.display(),
            err
          )
        })?;

      if let Some(filename) = relative_path.with_extension("").to_str() {
        // Extract the note name (last part of the path)
        let note_name = filename
          .split(MAIN_SEPARATOR)
          .last()
          .unwrap_or(filename)
          .to_string();

        // Normalize the path to use Unix-style separators
        let unix_path = &filename.replace('\\', "/");

        nodes.push(NodeInfo {
          id: note_name.replace(" ", "%20"),
          public: file_public,
          path: unix_path.clone(),
          r#type: "note".to_string(), // You can customize this based on frontmatter
        });

        // Extract links from the note
        let parse_options = markdown::ParseOptions {
          constructs: markdown::Constructs {
            frontmatter: true,
            ..markdown::Constructs::gfm()
          },
          ..markdown::ParseOptions::gfm()
        };

        let ast =
          markdown::to_mdast(content, &parse_options).expect("error when converting MD to AST");

        // Keep only the file name
        //  let file_name = unix_path.split('/').last().unwrap_or(unix_path).to_string();

        let full_path = path.to_str().unwrap_or(unix_path);
        let unix_full_path = &full_path.replace('\\', "/");
        let note_links = extract_links(&ast, unix_full_path);

        links.extend(note_links);
      }
    }
  }

  Ok(())
}

fn extract_frontmatter(content: &str) -> (&str, &str) {
  // Split the content at the first occurrence of '---'
  let parts: Vec<&str> = content.splitn(3, "---").collect();
  if parts.len() == 3 {
    // Return the frontmatter and the rest of the content
    (parts[1].trim(), parts[2].trim())
  } else {
    // Return empty frontmatter if not found
    ("", content.trim())
  }
}

fn extract_links(ast: &Node, source: &str) -> Vec<Link> {
  let mut links = Vec::new();
  traverse_nodes(ast, &mut links, source);
  links
}

fn traverse_nodes(node: &Node, links: &mut Vec<Link>, source: &str) {
  match node {
    Node::Link(link) => {
      let url = &link.url;
      if url.ends_with(".md") && !url.starts_with("http") {
        let target = url.trim_end_matches(".md");
        let target_name = target.split('/').last().unwrap_or(target).to_string();
        let source = source.trim_end_matches(".md");
        let source_name = source.split('/').last().unwrap_or(source).to_string();
        links.push(Link {
          source: source_name.replace(" ", "%20"),
          target: target_name.replace(" ", "%20"),
        });
      }
    }
    Node::Root(root) => {
      for child in &root.children {
        traverse_nodes(child, links, source);
      }
    }
    Node::Paragraph(root) => {
      for child in &root.children {
        traverse_nodes(child, links, source);
      }
    }
    Node::List(root) => {
      for child in &root.children {
        traverse_nodes(child, links, source);
      }
    }
    Node::ListItem(root) => {
      for child in &root.children {
        traverse_nodes(child, links, source);
      }
    }
    _ => {}
  }
}

/// Remove `.md` from links in the HTML content
fn remove_md_from_html_links(html_content: &str) -> String {
  // Use a regular expression to find and replace `.md` in links
  let re = regex::Regex::new(r#"href="([^"]+\.md)""#).unwrap();
  re.replace_all(html_content, |caps: &regex::Captures| {
    let link = caps.get(1).unwrap().as_str();
    let new_link = link.replace(".md", "");
    format!(r#"href="{}""#, new_link)
  })
  .to_string()
}

/// Rewrite img src in HTML
fn rewrite_src_from_html_img(html_content: &str) -> String {
  // Use a regular expression to find and replace `.md` in links
  let re = regex::Regex::new(r#"src="([^"]+\.webp)""#).unwrap();

  re.replace_all(html_content, |caps: &regex::Captures| {
    let link = caps.get(1).unwrap().as_str();
    let new_link = link.replace("../", "/");
    format!(r#"src="{}""#, new_link)
  })
  .to_string()
}

fn transform_callout(html: &str) -> String {
  // Regex to extract the callout type and content
  let re = regex::Regex::new(r"(?s)<blockquote>\s<p>\[!(.*?)\](.*?)</blockquote>").unwrap();
  
  // Replace the blockquote with the desired HTML structure
  re.replace_all(html, |caps: &regex::Captures| {
    
    
    let callout_type = caps.get(1).unwrap().as_str().to_lowercase(); // e.g., "info", "warning"
     let content = caps.get(2).unwrap().as_str();
    
    println!("Type {:?} Content : {:?}", callout_type, content);
    
    let svg_path = match callout_type.as_str() {
        "note" => {
          r#"M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"#
        }
        "tip" => {
          r#"M8 1.5c-2.363 0-4 1.69-4 3.75 0 .984.424 1.625.984 2.304l.214.253c.223.264.47.556.673.848.284.411.537.896.621 1.49a.75.75 0 0 1-1.484.211c-.04-.282-.163-.547-.37-.847a8.456 8.456 0 0 0-.542-.68c-.084-.1-.173-.205-.268-.32C3.201 7.75 2.5 6.766 2.5 5.25 2.5 2.31 4.863 0 8 0s5.5 2.31 5.5 5.25c0 1.516-.701 2.5-1.328 3.259-.095.115-.184.22-.268.319-.207.245-.383.453-.541.681-.208.3-.33.565-.37.847a.751.751 0 0 1-1.485-.212c.084-.593.337-1.078.621-1.489.203-.292.45-.584.673-.848.075-.088.147-.173.213-.253.561-.679.985-1.32.985-2.304 0-2.06-1.637-3.75-4-3.75ZM5.75 12h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1 0-1.5ZM6 15.25a.75.75 0 0 1 .75-.75h2.5a.75.75 0 0 1 0 1.5h-2.5a.75.75 0 0 1-.75-.75Z"#
        }
        "important" => {
          r#"M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm7 2.25v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 9a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"#
        }
        "caution" => {
          r#"M4.47.22A.749.749 0 0 1 5 0h6c.199 0 .389.079.53.22l4.25 4.25c.141.14.22.331.22.53v6a.749.749 0 0 1-.22.53l-4.25 4.25A.749.749 0 0 1 11 16H5a.749.749 0 0 1-.53-.22L.22 11.53A.749.749 0 0 1 0 11V5c0-.199.079-.389.22-.53Zm.84 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5ZM8 4a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"#
        }
        "question" => {
          r#"M8,0C3.6,0,0,3.6,0,8s3.6,8,8,8,8-3.6,8-8S12.4,0,8,0ZM8,14.5c-3.6,0-6.5-2.9-6.5-6.5S4.4,1.5,8,1.5s6.5,2.9,6.5,6.5-2.9,6.5-6.5,6.5ZM10.8,6.5c0,1.9-2.4,2.8-2.7,2.8h-.2c-.3,0-.6-.2-.7-.5-.1-.4,0-.8.4-.9.4-.1,1.7-.7,1.7-1.5s-.4-1.2-.9-1.4c-.7-.3-1.6.1-1.9.9-.1.4-.6.6-.9.4s-.6-.6-.4-.9c.5-1.5,2.2-2.3,3.7-1.7,1.2.4,2,1.5,2,2.8h0ZM8.7,11.6c0,.4-.3.7-.7.7s-.7-.3-.7-.7.3-.7.7-.7h0c.4,0,.7.3.7.7Z"#
        }
        "warning" => {
          r#"M8.22 1.754a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368L8.22 1.754ZM7 9a1 1 0 1 1 2 0v2a1 1 0 1 1-2 0V9Zm1-5a.75.75 0 0 1 .75.75v3a.75.75 0 0 1-1.5 0v-3A.75.75 0 0 1 8 4Z"#
        }
        _ => {
          r#"M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"#
        } // Default to info
      };

      let callout_type_name = callout_type.to_uppercase();

      let svg = format!(
        r#"<svg class="octicon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="{}"></path></svg>"#,
        svg_path
      );

      // Generate the new HTML structure
      format!(
        r#"<div class="markdown-callout markdown-callout-{callout_type}" dir="auto">
<p class="markdown-callout-title" dir="auto">
{svg}{callout_type_name}
</p>
{content}
</div>"#
    )
  })
  .to_string()
}
