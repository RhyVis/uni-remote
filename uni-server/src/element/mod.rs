use anyhow::{anyhow, Ok, Result};
use sc::{create_sc_info, SugarCubeInfo};
use std::{collections::HashMap, fs, path::PathBuf};
use tracing::{error, info, warn};

use crate::util::config::{config_ref, ManageInfo, ManageType, ReadConfig};

pub(crate) mod sc;

#[derive(Debug, Default)]
pub struct LoadedMapping {
    map: HashMap<String, LoadedType>,
}

impl LoadedMapping {
    pub fn get(&self, id: &str) -> Option<&LoadedType> {
        self.map.get(id)
    }

    pub fn insert(&mut self, id: String, loaded_type: LoadedType) {
        self.map.insert(id, loaded_type);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &LoadedType)> {
        self.map.iter()
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum LoadedType {
    Plain {
        root_path: PathBuf,
        enter_path: PathBuf,
        original_ref: &'static ManageInfo,
    },
    SugarCube {
        info: SugarCubeInfo,
        original_ref: &'static ManageInfo,
    },
}

pub fn load_data_dir() -> Result<LoadedMapping> {
    let config = config_ref();
    let mut mapping = LoadedMapping::default();

    let data_dir = config.data_dir();
    if !data_dir.exists() {
        warn!(
            "Data directory does not exist, creating: {}",
            data_dir.display()
        );
        fs::create_dir_all(&data_dir)?;
    }
    if config.manage_empty() {
        error!("At least one valid manage in 'config.toml' is required");
        return Err(anyhow!("No manage found"));
    }

    for (id, manage_info) in config.manage_iter() {
        info!(
            "Loading data dir for {}: {}",
            id,
            manage_info.name.clone().unwrap_or("No name?".to_string())
        );
        let path = data_dir.join(id);
        if !path.exists() {
            warn!(
                "Data directory for {} does not exist, creating: {}",
                id,
                path.display()
            );
            fs::create_dir_all(&path)?;
        }

        match &manage_info.mode {
            ManageType::Plain { enter_path } => {
                let actual_path = &path;
                let loaded_type = LoadedType::Plain {
                    root_path: actual_path.clone(),
                    enter_path: actual_path.join(enter_path),
                    original_ref: &manage_info,
                };

                mapping.insert(id.clone(), loaded_type);
            }

            ManageType::SugarCube {
                use_mods,
                use_save_sync,
            } => {
                if *use_save_sync {
                    let save_path = path.join("save");
                    if !save_path.exists() {
                        info!(
                            "Creating save directory for {}: {}",
                            id,
                            save_path.display()
                        );
                        fs::create_dir(path.join("save"))?;
                    }
                }
                let loaded_type = LoadedType::SugarCube {
                    info: create_sc_info(id, manage_info.name.clone(), *use_mods, *use_save_sync)?,
                    original_ref: &manage_info,
                };

                mapping.insert(id.clone(), loaded_type);
            }
        }
    }

    Ok(mapping)
}
