use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};
use tracing::warn;
use walkdir::WalkDir;

/// File Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileNode {
    File(String),
}

impl FileNode {
    pub fn resolve(&self) -> Option<(Vec<u8>, String)> {
        match self {
            FileNode::File(path) => {
                let path = Path::new(path);
                if path.exists() && path.is_file() {
                    match fs::read(&path) {
                        Ok(data) => Some((
                            data,
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                        )),
                        Err(err) => {
                            warn!("Failed to read file {:?}: {}", path, err);
                            None
                        }
                    }
                } else {
                    warn!("File not found or not a file: {:?}", path);
                    None
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapFileSystem {
    map: HashMap<String, FileNode>,
}

impl MapFileSystem {
    pub fn new(map: HashMap<String, FileNode>) -> Self {
        Self { map }
    }

    pub fn new_dir(source: impl AsRef<Path>) -> Result<Self> {
        let source_path = source.as_ref();
        let mut map = HashMap::new();

        if !source_path.exists() || !source_path.is_dir() {
            warn!(
                "Source dir not exists, or not a valid directory: {:?}",
                source_path
            );
            return Ok(Self { map });
        }

        for entry in WalkDir::new(source_path).into_iter().filter_map(Result::ok) {
            let entry_path = entry.path();

            if entry_path.is_dir() {
                continue;
            }

            let rel_path = entry_path.strip_prefix(source_path)?;

            let mut path_str = String::new();
            for component in rel_path.components() {
                if !path_str.is_empty() {
                    path_str.push('/');
                }
                path_str.push_str(component.as_os_str().to_string_lossy().as_ref());
            }

            map.insert(
                path_str,
                FileNode::File(entry_path.to_string_lossy().to_string()),
            );
        }

        Ok(Self { map })
    }

    pub fn get(&self, path: &str) -> Option<&FileNode> {
        self.map.get(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &FileNode)> {
        self.map.iter()
    }
}
