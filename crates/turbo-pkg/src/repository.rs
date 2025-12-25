use std::path::{Path, PathBuf};
use crate::models::PackageDefinition;
use semver::Version;

pub struct PackageRepository {
    root: PathBuf,
}

impl PackageRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub async fn resolve(&self, name: &str, version: Option<&str>) -> anyhow::Result<PackageDefinition> {
        let pkg_dir = self.root.join(name);
        if !pkg_dir.exists() {
            return Err(anyhow::anyhow!("Package '{}' not found in repository at {:?}", name, self.root));
        }

        let version_str = if let Some(v) = version {
            v.to_string()
        } else {
            self.find_latest_version(&pkg_dir).await?
        };

        let def_path = pkg_dir.join(&version_str);
        if !def_path.exists() {
            return Err(anyhow::anyhow!("Version '{}' of package '{}' not found", version_str, name));
        }

        PackageDefinition::from_path(def_path)
    }

    async fn find_latest_version(&self, pkg_dir: &Path) -> anyhow::Result<String> {
        let mut entries = tokio::fs::read_dir(pkg_dir).await?;
        let mut versions = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Ok(ver) = Version::parse(name) {
                        versions.push(ver);
                    } else {
                        // Handle non-semver directories if any (warn or ignore)
                        // For now, try to parse loose or just ignore
                        tracing::warn!("Skipping non-semver directory: {}", name);
                    }
                }
            }
        }

        if versions.is_empty() {
            return Err(anyhow::anyhow!("No valid versions found for package"));
        }

        versions.sort();
        // Get the last one (highest version)
        if let Some(latest) = versions.last() {
            Ok(latest.to_string())
        } else {
            Err(anyhow::anyhow!("No versions found"))
        }
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<(String, String)>> {
        let mut packages = Vec::new();
        if !self.root.exists() {
            return Ok(packages);
        }

        let mut entries = tokio::fs::read_dir(&self.root).await?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    let mut ver_entries = tokio::fs::read_dir(entry.path()).await?;
                    while let Ok(Some(ver_entry)) = ver_entries.next_entry().await {
                        if ver_entry.path().is_dir() {
                            if let Some(ver) = ver_entry.file_name().to_str() {
                                // Basic semver check or just list everything
                                if Version::parse(ver).is_ok() {
                                    packages.push((name.to_string(), ver.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
        // Sort by name then version
        packages.sort_by(|a, b| {
             match a.0.cmp(&b.0) {
                 std::cmp::Ordering::Equal => {
                     // Parse versions for correct sorting
                     let ver_a = Version::parse(&a.1).unwrap_or_else(|_| Version::new(0,0,0));
                     let ver_b = Version::parse(&b.1).unwrap_or_else(|_| Version::new(0,0,0));
                     ver_a.cmp(&ver_b)
                 }
                 other => other,
             }
        });
        Ok(packages)
    }
}
