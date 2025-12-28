use crate::models::{PackageDefinition};
use std::path::{PathBuf};
use std::process::Command;
use tokio::fs;

pub struct Installer {
    runtimes_dir: PathBuf,
}

impl Installer {
    pub fn new(runtimes_dir: PathBuf) -> Self {
        Self { runtimes_dir }
    }

    pub async fn install(&self, def: &PackageDefinition) -> anyhow::Result<()> {
        let pkg_name = &def.yaml.name;
        let pkg_version = &def.yaml.version;

        let install_dir = self.runtimes_dir.join(pkg_name).join(pkg_version);

        if install_dir.exists() {
            tracing::info!(
                "Package {}@{} is already installed at {:?}",
                pkg_name,
                pkg_version,
                install_dir
            );
            return Ok(());
        }

        tracing::info!(
            "Installing {}@{} from {:?}",
            pkg_name,
            pkg_version,
            def.path
        );

        // 1. Run build.sh
        // Canonicalize path to ensure reliable execution independent of CWD
        let abs_pkg_path = def.path.canonicalize().map_err(|e| {
            anyhow::anyhow!("Failed to canonicalize package path {:?}: {}", def.path, e)
        })?;
        let build_script = abs_pkg_path.join("build.sh");

        if !build_script.exists() {
            return Err(anyhow::anyhow!("build.sh not found at {:?}", build_script));
        }

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&build_script)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&build_script, perms)?;
        }

        // Prepare install directory
        fs::create_dir_all(&install_dir).await?;

        // Execute build.sh
        // Pass install_dir as argument $1
        let status = Command::new(&build_script)
            .arg(&install_dir)
            .current_dir(&abs_pkg_path)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to execute build.sh: {}", e))?;

        if !status.success() {
            // Cleanup on failure
            let _ = fs::remove_dir_all(&install_dir).await;
            return Err(anyhow::anyhow!("build.sh failed with status: {}", status));
        }

        // 2. Copy run.sh and env
        let run_script = abs_pkg_path.join("run.sh");
        if run_script.exists() {
            fs::copy(&run_script, install_dir.join("run.sh")).await?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(install_dir.join("run.sh"))?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(install_dir.join("run.sh"), perms)?;
            }
        }

        let compile_script = abs_pkg_path.join("compile.sh");
        if compile_script.exists() {
            fs::copy(&compile_script, install_dir.join("compile.sh")).await?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(install_dir.join("compile.sh"))?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(install_dir.join("compile.sh"), perms)?;
            }
        }

        let env_file = abs_pkg_path.join("env");
        if env_file.exists() {
            fs::copy(&env_file, install_dir.join("env")).await?;
        }

        // Copy package.yaml for metadata
        fs::copy(
            abs_pkg_path.join("package.yaml"),
            install_dir.join("package.yaml"),
        )
        .await?;

        tracing::info!("Successfully installed {}@{}", pkg_name, pkg_version);
        Ok(())
    }
}
