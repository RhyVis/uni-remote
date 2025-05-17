use super::cd_in;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, sync::OnceLock};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    port: u16,
    #[serde(default)]
    root: String,
    #[serde(default)]
    manage: HashMap<String, ManageInfo>,
}

pub trait ReadConfig {
    fn port(&self) -> u16;
    fn data_dir(&self) -> PathBuf;
    fn manage_iter(&self) -> impl Iterator<Item = (&String, &ManageInfo)>;
    fn manage_size(&self) -> usize;
    fn manage_empty(&self) -> bool {
        self.manage_size() == 0
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 3500,
            root: String::from("data"),
            manage: HashMap::new(),
        }
    }
}

impl ReadConfig for Config {
    fn port(&self) -> u16 {
        self.port
    }

    fn data_dir(&self) -> PathBuf {
        cd_in(&self.root)
    }

    fn manage_iter(&self) -> impl Iterator<Item = (&String, &ManageInfo)> {
        self.manage.iter()
    }

    fn manage_size(&self) -> usize {
        self.manage.len()
    }
}

pub fn config_ref() -> &'static Config {
    const CONFIG_FILE_NAME: &str = "config.toml";
    static CONFIG: OnceLock<Config> = OnceLock::new();

    fn load() -> Result<Config> {
        let config_path = cd_in(CONFIG_FILE_NAME);
        let content = match fs::read_to_string(&config_path) {
            Ok(content) => content,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    let default = Config::default();
                    let default_content = toml::to_string_pretty(&default)?;
                    fs::create_dir_all(&config_path.parent().unwrap_or(&config_path))?;
                    fs::write(&config_path, default_content)?;
                    info!(
                        "Config file not found, created default config file at: {}",
                        config_path.display()
                    );
                    return Ok(default);
                }
                return Err(err.into());
            }
        };
        let config = toml::from_str::<Config>(&content)?;

        info!("Loaded config file from: {}", config_path.display());

        Ok(config)
    }

    CONFIG.get_or_init(|| load().expect("Cannot load config file at all!"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManageInfo {
    #[serde(default)]
    pub name: Option<String>,
    pub mode: ManageType,
}

fn default_enter_path() -> String {
    const DEFAULT_ENTER_PATH: &str = "index.html";
    DEFAULT_ENTER_PATH.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub enum ManageType {
    Plain {
        #[serde(default = "default_enter_path")]
        enter_path: String,
    },
    SugarCube {
        #[serde(default)]
        use_mods: bool,
        #[serde(default)]
        use_save_sync: bool,
    },
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ser() {
        let manage_type = ManageType::Plain {
            enter_path: "index.html".to_string(),
        };
        let info1 = ManageInfo {
            name: Some("Test".to_string()),
            mode: manage_type.clone(),
        };
        let ser1 = toml::to_string_pretty(&info1).unwrap();
        println!("ser1: ");
        println!("{}", ser1);

        let manage_type = ManageType::SugarCube {
            use_mods: true,
            use_save_sync: false,
        };
        let info2 = ManageInfo {
            name: Some("Test2".to_string()),
            mode: manage_type.clone(),
        };
        let ser2 = toml::to_string_pretty(&info2).unwrap();
        println!("ser2: ");
        println!("{}", ser2);

        let mut manage = HashMap::new();
        manage.insert("test".to_string(), info1);
        manage.insert("test2".to_string(), info2);

        let config = Config {
            manage,
            ..Default::default()
        };
        let ser3 = toml::to_string_pretty(&config).unwrap();
        println!("ser3: ");
        println!("{}", ser3);
    }
}
