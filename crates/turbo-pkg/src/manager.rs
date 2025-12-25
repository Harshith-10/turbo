use crate::installer::Installer;
use crate::models::{PackageDefinition, PackageYaml};
use crate::repository::PackageRepository;
use std::path::{Path, PathBuf};

pub struct PackageManager {
    installer: Installer,
    repository: PackageRepository,
    runtimes_dir: PathBuf,
}

impl PackageManager {
    pub fn new(root: PathBuf, repo_path: PathBuf) -> Self {
        let runtimes_dir = root.join("runtimes");
        Self {
            installer: Installer::new(runtimes_dir.clone()),
            repository: PackageRepository::new(repo_path),
            runtimes_dir,
        }
    }

    pub async fn install(&self, name: &str, version: Option<&str>) -> anyhow::Result<()> {
        let def = self.repository.resolve(name, version).await?;
        self.installer.install(&def).await
    }

    pub async fn list_available(&self) -> anyhow::Result<Vec<crate::models::PackageInfo>> {
        let repo_packages = self.repository.list_all().await?;
        let mut result = Vec::new();

        for (name, version) in repo_packages {
            let install_path = self.runtimes_dir.join(&name).join(&version);
            let installed = install_path.exists();
            
            result.push(crate::models::PackageInfo {
                name,
                version,
                installed,
            });
        }
        
        Ok(result)
    }
}
