use clap::{Parser, Subcommand};
use colored::Colorize;
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
    /// Execute a file
    Execute {
        /// Language (e.g. python, java)
        language: String,
        /// Version
        #[arg(short, long)]
        version: Option<String>,
        /// Path to file
        file: PathBuf,
        /// Server URL (default: http://localhost:3000)
        #[arg(long, default_value = "http://localhost:4000")]
        server: String,
    },
    /// Package Management
    Pkg {
        #[command(subcommand)]
        cmd: PkgCommands,
    },
    /// Cache Management
    Cache {
        #[command(subcommand)]
        cmd: CacheCommands,
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

#[derive(Subcommand)]
enum CacheCommands {
    /// Clear the compilation cache
    Clear,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Default turbo home
    let home = std::env::var("TURBO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".turbo")
        });

    // Repository path (default: ./packages)
    let repo_path = std::env::var("TURBO_PACKAGES_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap().join("packages"));

    match cli.command {
        Commands::Start => {
            let server_bin = if let Ok(exe) = std::env::current_exe() {
                let candidate = exe.parent().unwrap().join("turbo-server");
                if candidate.exists() {
                    candidate.to_string_lossy().to_string()
                } else {
                    "turbo-server".to_string()
                }
            } else {
                "turbo-server".to_string()
            };

            // Check if root using 'id -u'
            let is_root = if let Ok(output) = std::process::Command::new("id").arg("-u").output() {
                String::from_utf8_lossy(&output.stdout).trim() == "0"
            } else {
                false
            };

            if !is_root {
                info!("Turbo Server requires root privileges.");
                info!("Requesting sudo access to start server...");

                let mut cmd = std::process::Command::new("sudo");
                // -E preserves environment variables (HOME) so server sees user's home
                cmd.arg("-E").arg(&server_bin);

                match cmd.status() {
                    Ok(status) => {
                        if !status.success() {
                            tracing::error!("Server process exited with code: {:?}", status.code());
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to start server with sudo: {}", e);
                    }
                }
            } else {
                info!("Starting Turbo Server...");
                match std::process::Command::new(&server_bin).status() {
                    Ok(status) => {
                        if !status.success() {
                            tracing::error!("Server process exited with code: {:?}", status.code());
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to start server: {}", e);
                    }
                }
            }
        }
        Commands::Execute {
            language,
            version,
            file,
            server,
        } => {
            use turbo_core::models::{FileRequest, JobRequest};

            let content = std::fs::read_to_string(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read file {:?}: {}", file, e))?;

            let filename = file
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());

            let req = JobRequest {
                language,
                version,
                files: vec![FileRequest {
                    name: filename.clone(),
                    content,
                    encoding: Some("utf8".to_string()),
                }],
                testcases: None, // Interactive/One-shot mode
                args: Some(vec![filename.clone().unwrap_or("main".to_string())]),
                stdin: None, // TODO: Read from stdin if needed?
                run_timeout: None,
                compile_timeout: None,
                run_memory_limit: None,
                compile_memory_limit: None,
            };

            let client = reqwest::Client::new();
            let url = format!("{}/api/v1/execute", server);

            let res = client.post(&url).json(&req).send().await?;

            if !res.status().is_success() {
                let err_text = res.text().await?;
                eprintln!("Execution failed: {}", err_text);
                std::process::exit(1);
            }

            let job_result: turbo_core::models::JobResult = res.json().await?;

            if let Some(compile) = job_result.compile {
                if compile.status != turbo_core::models::StageStatus::Success {
                    println!("{}", "Compilation Failed".red().bold());
                    println!("{}", compile); // StageResult implements Display
                    return Ok(());
                }
            }

            if let Some(run) = job_result.run {
                println!("{}", "Execution Result".green().bold());
                println!("{}", run);
            } else {
                println!("No execution result returned.");
            }
        }
        Commands::Pkg { cmd } => {
            let pkg_root = home;
            let manager = PackageManager::new(pkg_root, repo_path);

            match cmd {
                PkgCommands::Install {
                    name,
                    version,
                    local: _,
                } => {
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
                                let ver_a = semver::Version::parse(&a.version)
                                    .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
                                let ver_b = semver::Version::parse(&b.version)
                                    .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
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
        Commands::Cache { cmd } => {
            match cmd {
                CacheCommands::Clear => {
                    let cache_path = std::path::PathBuf::from("/tmp/turbo-cache");
                    if cache_path.exists() {
                        match std::fs::remove_dir_all(&cache_path) {
                            Ok(_) => println!("{}", "Cache cleared successfully.".green().bold()),
                            Err(e) => {
                                eprintln!("{} {}", "Failed to clear cache:".red().bold(), e);
                                eprintln!("(You might need to run with sudo if the cache is owned by root)");
                            }
                        }
                    } else {
                        println!("Cache directory not found. Nothing to clear.");
                    }
                }
            }
        }
    }

    Ok(())
}
