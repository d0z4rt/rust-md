use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserConfig {
  pub root_path: String,
  pub ignore: Vec<String>,
}

pub fn load_config(path: &str) -> Result<UserConfig, Box<dyn std::error::Error>> {
  let file = std::fs::File::open(path)?;
  let file_size = file.metadata()?.len();

  if file_size == 0 {
    return Err("The config file is empty.".into());
  }

  let reader = std::io::BufReader::new(file);

  let config: UserConfig = serde_yaml_ng::from_reader(reader)?;
  Ok(config)
}
