use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageYaml {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub compiled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PackageDefinition {
    pub path: PathBuf,
    pub yaml: PackageYaml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub installed: bool,
}

impl PackageDefinition {
    pub fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        let yaml_path = path.join("package.yaml");
        let content = std::fs::read_to_string(&yaml_path).map_err(|e| {
            anyhow::anyhow!("Failed to read package.yaml at {:?}: {}", yaml_path, e)
        })?;
        let yaml: PackageYaml = serde_yaml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse package.yaml: {}", e))?;

        Ok(Self { path, yaml })
    }
}
