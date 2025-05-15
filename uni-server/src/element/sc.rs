use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{Instant, SystemTime},
};

use anyhow::Result;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bincode::config::{Configuration, standard};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use walkdir::WalkDir;

use crate::{
    constants::SSI_MOD_ID,
    util::{
        config::{Config, ReadConfig, config_ref},
        mfs::MapFileSystem,
        path_ext::PathHelper,
    },
};

const INSTANCE_DIR_NAME: &'static str = "instance";
const INDEX_DIR_NAME: &'static str = "index";
const LAYER_DIR_NAME: &'static str = "layer";
const MOD_DIR_NAME: &'static str = "mod";

trait ReadConfigSugarCube: ReadConfig {
    fn instance_dir(&self, id: &str) -> PathBuf {
        self.data_dir().join(id).join(INSTANCE_DIR_NAME)
    }
    fn index_dir(&self, id: &str) -> PathBuf {
        self.data_dir().join(id).join(INDEX_DIR_NAME)
    }
    fn layer_dir(&self, id: &str) -> PathBuf {
        self.data_dir().join(id).join(LAYER_DIR_NAME)
    }
    fn mod_dir(&self, id: &str) -> PathBuf {
        self.data_dir().join(id).join(MOD_DIR_NAME)
    }
}

impl ReadConfigSugarCube for Config {}

type InstanceMap = HashMap<String, SugarCubeInstance>;
type IndexMap = HashMap<String, PathBuf>;
type LayerMap = HashMap<String, MapFileSystem>;
type ModMap = HashMap<String, HashMap<String, PathBuf>>;
type ModRefMap = HashMap<(String, String), PathBuf>;

#[derive(Debug)]
#[allow(dead_code)]
pub struct SugarCubeInfo {
    pub name: Option<String>,
    pub instances: InstanceMap,
    pub mods: ModMap,

    pub use_mods: bool,
    pub use_save_sync_mod: bool,
}

impl SugarCubeInfo {
    pub fn get_instance(&self, id: &str) -> Option<&SugarCubeInstance> {
        self.instances.get(id)
    }
    pub fn get_mod(&self, mod_id: &str, mod_sub_id: &str) -> Option<&PathBuf> {
        self.mods.get(mod_id).and_then(|m| m.get(mod_sub_id))
    }

    pub fn generate_mod_list(
        &self,
        instance_id: &str,
        manage_id: &str,
    ) -> Result<Vec<String>, Response> {
        if !self.use_mods {
            warn!("Mod list generation is disabled");
            return Err(
                (StatusCode::BAD_REQUEST, "Mod list generation is disabled").into_response()
            );
        }
        let instance = self.get_instance(instance_id).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Instance ID {instance_id} not found"),
            )
                .into_response()
        })?;
        let mut mod_list = instance
            .mods_ref
            .keys()
            .map(|(mod_id, mod_sub_id)| format!("/repo/sc/mod/{manage_id}/{mod_id}/{mod_sub_id}"))
            .collect::<Vec<_>>();

        if self.use_save_sync_mod {
            mod_list.push(format!("/sc/mod/{manage_id}/{SSI_MOD_ID}/0").to_string());
        }

        Ok(mod_list)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct SugarCubeInstance {
    pub id: String,
    pub name: Option<String>,
    pub index_path: PathBuf,
    pub layer_merged: MapFileSystem,
    pub mods_ref: ModRefMap,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SugarCubeInstanceConfig {
    pub id: String,
    pub name: Option<String>,
    pub index: String,
    pub layers: Vec<String>,
    pub mods: Vec<(String, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LayerCache {
    last_modified: SystemTime,
    layer_map: LayerMap,
}

pub(super) fn create_sc_info(
    id: &str,
    name: Option<String>,
    use_mods: bool,
    use_save_sync_mod: bool,
) -> Result<SugarCubeInfo> {
    let indexes = create_indexes(id)?;
    let layers = create_layers(id)?;
    let mods = if use_mods {
        create_mods(id)?
    } else {
        HashMap::new()
    };

    let instances = create_instances(id, &indexes, &layers, &mods)?;

    Ok(SugarCubeInfo {
        name,
        instances,
        mods,
        use_mods,
        use_save_sync_mod,
    })
}

fn create_instances(
    id: &str,
    index_map: &IndexMap,
    layer_map: &LayerMap,
    mod_map: &ModMap,
) -> Result<InstanceMap> {
    let instance_dir = config_ref().instance_dir(id);
    if !instance_dir.exists() {
        warn!(
            "Instance directory {} does not exist, initialized",
            instance_dir.display()
        );
        fs::create_dir(&instance_dir)?;

        let example = SugarCubeInstanceConfig {
            id: "example".to_string(),
            name: Some("Example Instance".to_string()),
            index: "index".to_string(),
            layers: vec!["layer1".to_string(), "layer2".to_string()],
            mods: vec![
                ("mod1".to_string(), "1.0".to_string()),
                ("mod2".to_string(), "0.3.9-test".to_string()),
            ],
        };
        let example_path = instance_dir.join("_example.yaml");
        let example_str = format!(
            "{}\n{}",
            "# This is an example file for instance config, always ignored",
            serde_yaml::to_string(&example)?
        );
        fs::write(example_path, example_str)?;
        info!(
            "Created example instance config at {}",
            instance_dir.join("_example.yaml").display()
        );

        return Ok(HashMap::new());
    }

    let walker = WalkDir::new(&instance_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.file_name() != "_example.yaml")
        .filter(|e| e.path().extension_eqs(&["json", "toml", "yaml", "yml"]));

    let mut map = HashMap::new();
    let start = Instant::now();

    for entry in walker {
        let path = entry.path();
        let lowercase_ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let ext = lowercase_ext.as_str();

        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                error!("Error reading file {}: {}", path.display(), e);
                continue;
            }
        };

        let instance_config: SugarCubeInstanceConfig = match ext {
            "json" => serde_json::from_str(content.as_str())?,
            "toml" => toml::from_str(content.as_str())?,
            "yaml" | "yml" => serde_yaml::from_str(content.as_str())?,
            _ => {
                warn!(
                    "Unsupported file extension {} for instance config, why are you here?? : {}",
                    ext,
                    path.display()
                );
                continue;
            }
        };

        // Resolving references
        let index_ref = match index_map.get(&instance_config.index) {
            Some(r) => r,
            None => {
                warn!(
                    "Index {} referenced by {} not found, skipping",
                    instance_config.index, instance_config.id
                );
                continue;
            }
        };

        let mut merged_layer_map = HashMap::new();
        for layer_id in instance_config.layers {
            if let Some(mfs) = layer_map.get(&layer_id) {
                for (k, v) in mfs.iter() {
                    merged_layer_map.insert(k.clone(), v.clone());
                }
            }
        }
        let merged_mfs = MapFileSystem::new(merged_layer_map);

        let mut mod_ref_map = HashMap::new();

        for (mod_id, mod_sub_id) in instance_config.mods {
            if let Some(mod_subs) = mod_map.get(&mod_id) {
                if let Some(mod_path) = mod_subs.get(&mod_sub_id) {
                    mod_ref_map.insert((mod_id, mod_sub_id), mod_path.clone());
                } else {
                    warn!(
                        "Mod {} with sub_id {} referenced by {} not found, skipping",
                        mod_id, mod_sub_id, instance_config.id
                    );
                    continue;
                }
            } else {
                warn!(
                    "Mod {} referenced by {} not found, skipping",
                    mod_id, instance_config.id
                );
                continue;
            }
        }

        let instance = SugarCubeInstance {
            id: instance_config.id.clone(),
            name: instance_config.name,
            index_path: index_ref.clone(),
            layer_merged: merged_mfs,
            mods_ref: mod_ref_map,
        };

        map.insert(instance_config.id.clone(), instance);
    }

    if map.is_empty() {
        warn!(
            "No valid instances found for {} in {}",
            id,
            instance_dir.display()
        );
    } else {
        info!(
            "Created {} instances for {} in {}ms",
            map.len(),
            id,
            start.elapsed().as_millis()
        );
    }
    Ok(map)
}

fn create_indexes(id: &str) -> Result<IndexMap> {
    let index_dir = config_ref().index_dir(id);
    if !index_dir.exists() {
        warn!(
            "Index directory {} does not exist, initialized",
            index_dir.display()
        );
        fs::create_dir(&index_dir)?;
        return Ok(HashMap::new());
    }

    let walker = WalkDir::new(&index_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension_eq("html"));

    let mut map = HashMap::new();
    let start = Instant::now();

    for entry in walker {
        let path = entry.path();
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        map.insert(name, path.to_path_buf());
    }

    if map.is_empty() {
        warn!(
            "No valid indexes found for {} in {}",
            id,
            index_dir.display()
        );
    } else {
        info!(
            "Created {} indexes for {} in {}ms",
            map.len(),
            id,
            start.elapsed().as_millis()
        );
    }
    Ok(map)
}

fn create_layers(id: &str) -> Result<LayerMap> {
    let layer_dir = config_ref().layer_dir(id);
    if !layer_dir.exists() {
        warn!(
            "Layer directory {} does not exist, initialized",
            layer_dir.display()
        );
        fs::create_dir(&layer_dir)?;
        return Ok(HashMap::new());
    }

    let start = Instant::now();

    fn get_latest_modified_time(dir: impl AsRef<Path>) -> SystemTime {
        let mut latest_time = fs::metadata(dir.as_ref())
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() != "cache.bin")
        {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified > latest_time {
                        latest_time = modified;
                    }
                }
            }
        }

        latest_time
    }

    let layer_cache_path = layer_dir.join("cache.bin");
    let current_modified = get_latest_modified_time(&layer_dir);

    if let Ok(cache_file) = fs::read(&layer_cache_path) {
        if let Ok((cache, _)) = bincode::serde::decode_from_slice::<LayerCache, Configuration>(
            &cache_file,
            bincode::config::standard(),
        ) {
            if current_modified <= cache.last_modified {
                info!(
                    "Using cached layer map for {} with {} items, created on '{}' ({}ms)",
                    id,
                    cache.layer_map.len(),
                    chrono::DateTime::<chrono::Local>::from(cache.last_modified)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string(),
                    start.elapsed().as_millis()
                );
                return Ok(cache.layer_map);
            } else {
                info!(
                    "Cache for {} is outdated, last modified on '{}', current modified on '{}'",
                    id,
                    chrono::DateTime::<chrono::Local>::from(cache.last_modified)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string(),
                    chrono::DateTime::<chrono::Local>::from(current_modified)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                );
            }
        }
    }
    info!("No valid cache found for {}, creating new layer map", id);

    let layer_roots = layer_dir
        .read_dir()
        .map_err(|e| {
            error!("Error reading mod directory {}: {}", layer_dir.display(), e);
            e
        })?
        .filter_map(|entry_result| {
            entry_result
                .map_err(|e| {
                    error!("Error reading mod directory entry: {}", e);
                })
                .ok()
        })
        .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_dir()));

    let mut map = HashMap::new();

    for entry in layer_roots {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let now = Instant::now();
        let mfs = match MapFileSystem::new_dir(&path) {
            Ok(mfs) => {
                info!(
                    "Initialized MFS by dir '{}' in {}ms",
                    name,
                    now.elapsed().as_millis()
                );
                mfs
            }
            Err(e) => {
                error!(
                    "Error creating MapFileSystem in {}, skipping: {}",
                    path.display(),
                    e
                );
                continue;
            }
        };

        map.insert(name, mfs);
    }

    if map.is_empty() {
        warn!(
            "No valid layers found for {} in {}",
            id,
            layer_dir.display()
        );
    } else {
        info!(
            "Created {} layers for {} in {}ms",
            map.len(),
            id,
            start.elapsed().as_millis()
        );

        let cache = LayerCache {
            last_modified: current_modified,
            layer_map: map.clone(),
        };

        if let Ok(content) = bincode::serde::encode_to_vec(cache, standard()) {
            fs::write(layer_cache_path, content)?;
        } else {
            error!(
                "Error writing layer cache to {}",
                layer_cache_path.display()
            );
        }
    }
    Ok(map)
}

fn create_mods(id: &str) -> Result<ModMap> {
    let mod_dir = config_ref().mod_dir(id);
    if !mod_dir.exists() {
        warn!(
            "Mod directory {} does not exist, initialized",
            mod_dir.display()
        );
        fs::create_dir(&mod_dir)?;
        return Ok(HashMap::new());
    }

    let mut repo = HashMap::new();
    let start = Instant::now();

    let mod_roots = mod_dir
        .read_dir()
        .map_err(|e| {
            error!("Error reading mod directory {}: {}", mod_dir.display(), e);
            e
        })?
        .filter_map(|entry_result| {
            entry_result
                .map_err(|e| {
                    error!("Error reading mod directory entry: {}", e);
                })
                .ok()
        })
        .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_dir()));

    for entry in mod_roots {
        let mod_id = entry.file_name().to_string_lossy().to_string();
        let mod_files = WalkDir::new(entry.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension_eq("zip"))
            .map(|e| {
                let path = e.path().to_path_buf();
                let filename = e.file_name().to_string_lossy();
                // Remove .zip extension to get the mod name
                let name = if let Some(name) = filename.strip_suffix(".zip") {
                    name.to_string()
                } else {
                    filename.to_string()
                };
                (name, path)
            })
            .collect::<HashMap<String, PathBuf>>();

        repo.insert(mod_id, mod_files);
    }

    if repo.is_empty() {
        warn!("No valid mods found for {} in {}", id, mod_dir.display());
    } else {
        info!(
            "Created {} mods for {} in {}ms",
            repo.len(),
            id,
            start.elapsed().as_millis()
        );
    }
    Ok(repo)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_instance_config() {
        let config = SugarCubeInstanceConfig {
            id: "test".to_string(),
            name: Some("Test Instance".to_string()),
            index: "index".to_string(),
            layers: vec!["layer1".to_string(), "layer2".to_string()],
            mods: vec![
                ("mod1".to_string(), "1.0".to_string()),
                ("mod2".to_string(), "1.3.0".to_string()),
            ],
        };

        let ser_toml = toml::to_string_pretty(&config).unwrap();
        println!("Serialized toml: {}", ser_toml);

        let ser_yaml = serde_yaml::to_string(&config).unwrap();
        println!("Serialized yaml: {}", ser_yaml);

        let ser_json = serde_json::to_string_pretty(&config).unwrap();
        println!("Serialized json: {}", ser_json);
    }
}
