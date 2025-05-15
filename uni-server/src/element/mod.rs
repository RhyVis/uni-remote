use anyhow::{Ok, Result};
use sc::{SugarCubeInfo, create_sc_info};
use std::{collections::HashMap, path::PathBuf};
use tracing::info;

use crate::util::config::{ManageType, ReadConfig, config_ref};

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
}

#[derive(Debug)]
pub enum LoadedType {
    Plain { root_path: PathBuf },
    SugarCube { info: SugarCubeInfo },
}

pub fn load_data_dir() -> Result<LoadedMapping> {
    let config = config_ref();
    let mut mapping = LoadedMapping::default();

    for (id, manage_info) in config.manage_iter() {
        info!(
            "Loading data dir for {}: {}",
            id,
            manage_info.name.clone().unwrap_or("No name?".to_string())
        );

        match manage_info.mode {
            ManageType::Plain => {
                let actual_path = config.data_dir().join(id);
                let loaded_type = LoadedType::Plain {
                    root_path: actual_path,
                };

                mapping.insert(id.clone(), loaded_type);
            }

            ManageType::SugarCube {
                use_mods,
                use_save_sync_mod,
            } => {
                let loaded_type = LoadedType::SugarCube {
                    info: create_sc_info(
                        id,
                        manage_info.name.clone(),
                        use_mods,
                        use_save_sync_mod,
                    )?,
                };

                mapping.insert(id.clone(), loaded_type);
            }
        }
    }

    Ok(mapping)
}
