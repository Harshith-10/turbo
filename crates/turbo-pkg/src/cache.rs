use crate::models::PackageInfo;
use crate::repository::PackageRepository;
use semver::Version;
use std::path::PathBuf;
use std::sync::RwLock;

/// In-memory cache of installed packages, populated at startup.
pub struct PackageCache {
    packages: RwLock<Vec<PackageInfo>>,
}

impl PackageCache {
    /// Scan the filesystem and build the package cache.
    ///
    /// - `repo_path`: Path to the package definitions (e.g., ./packages)
    /// - `runtimes_dir`: Path to installed runtimes (e.g., ~/.turbo/runtimes)
    pub async fn from_paths(repo_path: PathBuf, runtimes_dir: PathBuf) -> anyhow::Result<Self> {
        let repo = PackageRepository::new(repo_path);
        let repo_packages = repo.list_all().await?;

        let mut packages = Vec::new();
        for (name, version) in repo_packages {
            let install_path = runtimes_dir.join(&name).join(&version);
            let installed = install_path.exists();

            packages.push(PackageInfo {
                name,
                version,
                installed,
            });
        }

        // Sort by name, then by version descending
        packages.sort_by(|a, b| {
            match a.name.cmp(&b.name) {
                std::cmp::Ordering::Equal => {
                    let ver_a = Version::parse(&a.version).unwrap_or_else(|_| Version::new(0, 0, 0));
                    let ver_b = Version::parse(&b.version).unwrap_or_else(|_| Version::new(0, 0, 0));
                    ver_b.cmp(&ver_a) // Descending
                }
                other => other,
            }
        });

        tracing::info!("Loaded {} packages into cache", packages.len());

        Ok(Self {
            packages: RwLock::new(packages),
        })
    }

    /// Return a clone of all cached packages.
    pub fn list(&self) -> Vec<PackageInfo> {
        self.packages.read().unwrap().clone()
    }
}
