use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use turbo_pkg::manager::PackageManager;

#[derive(Parser)]
#[command(name = "turbo")]
#[command(about = "Turbo High-Performance Execution Engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Turbo Server
    Start,
    /// Package Management
    Pkg {
        #[command(subcommand)]
        cmd: PkgCommands,
    },
}

#[derive(Subcommand)]
enum PkgCommands {
    /// Install a package
    Install {
        /// Name of the package (e.g. python)
        name: String,
        /// Version to install (default: latest)
        #[arg(short, long)]
        version: Option<String>,
        /// Install from local path (not implemented yet)
        #[arg(long)]
        local: Option<PathBuf>,
    },
    /// List installed packages
    List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Default turbo home
    let home = std::env::var("TURBO_HOME").map(PathBuf::from).unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME not set");
        PathBuf::from(home).join(".turbo")
    });

    // Repository path (default: ./packages)
    let repo_path = std::env::var("TURBO_PACKAGES_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap().join("packages"));

    match cli.command {
        Commands::Start => {
            info!("Starting Turbo Server... (Not implemented in Phase 2)");
        }
        Commands::Pkg { cmd } => {
            let pkg_root = home;
            let manager = PackageManager::new(pkg_root, repo_path);

            match cmd {
                PkgCommands::Install { name, version, local: _ } => {
                     // Pass name and optional version
                     manager.install(&name, version.as_deref()).await?;
                }
                PkgCommands::List => {
                    use colored::*;
                    use std::collections::BTreeMap;
                    // Fix: Use correct import path for PackageInfo
                    use turbo_pkg::models::PackageInfo;
                    
                    let packages = manager.list_available().await?;
                    if packages.is_empty() {
                         println!("No packages found in repository.");
                    } else {
                        // Group by package name
                        let mut grouped: BTreeMap<String, Vec<PackageInfo>> = BTreeMap::new();
                        for pkg in packages {
                            grouped.entry(pkg.name.clone()).or_default().push(pkg);
                        }

                        for (name, mut versions) in grouped {
                            println!("> {}", name.bold());
                            // Sort versions descending (newest first)
                            versions.sort_by(|a, b| {
                                let ver_a = semver::Version::parse(&a.version).unwrap_or_else(|_| semver::Version::new(0,0,0));
                                let ver_b = semver::Version::parse(&b.version).unwrap_or_else(|_| semver::Version::new(0,0,0));
                                ver_b.cmp(&ver_a)
                            });

                            for pkg in versions {
                                let icon = if pkg.installed {
                                    "●".green()
                                } else {
                                    "●".red()
                                };
                                println!("    {} {}", icon, pkg.version);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
