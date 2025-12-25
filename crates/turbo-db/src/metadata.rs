use anyhow::Result;
use sqlx::{Pool, Row, Sqlite, sqlite::SqlitePoolOptions};
use turbo_core::models::{Package, Runtime};

#[derive(Clone)]
pub struct SqliteMetadataStore {
    pool: Pool<Sqlite>,
}

impl SqliteMetadataStore {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new().connect(database_url).await?;

        let store = Self { pool };
        store.ensure_schema().await?;
        Ok(store)
    }

    async fn ensure_schema(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS runtimes (
                language TEXT NOT NULL,
                version TEXT NOT NULL,
                aliases TEXT NOT NULL,
                runtime TEXT,
                PRIMARY KEY (language, version)
            );",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS packages (
                language TEXT NOT NULL,
                version TEXT NOT NULL,
                installed BOOLEAN NOT NULL,
                PRIMARY KEY (language, version)
            );",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_runtimes(&self) -> Result<Vec<Runtime>> {
        let rows = sqlx::query("SELECT language, version, aliases, runtime FROM runtimes")
            .fetch_all(&self.pool)
            .await?;

        let mut runtimes = Vec::new();
        for row in rows {
            let aliases_json: String = row.try_get("aliases")?;
            let aliases: Vec<String> = serde_json::from_str(&aliases_json).unwrap_or_default();

            // Handle nullable 'runtime'
            let runtime: Option<String> = row.try_get("runtime")?;

            runtimes.push(Runtime {
                language: row.try_get("language")?,
                version: row.try_get("version")?,
                aliases,
                runtime,
            });
        }
        Ok(runtimes)
    }

    pub async fn add_runtime(&self, runtime: &Runtime) -> Result<()> {
        let aliases_json = serde_json::to_string(&runtime.aliases)?;
        sqlx::query(
            "INSERT OR REPLACE INTO runtimes (language, version, aliases, runtime) VALUES (?, ?, ?, ?)"
        )
        .bind(&runtime.language)
        .bind(&runtime.version)
        .bind(aliases_json)
        .bind(&runtime.runtime)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_packages(&self) -> Result<Vec<Package>> {
        let rows = sqlx::query("SELECT language, version, installed FROM packages")
            .fetch_all(&self.pool)
            .await?;

        let mut packages = Vec::new();
        for row in rows {
            packages.push(Package {
                language: row.try_get("language")?,
                language_version: row.try_get("version")?,
                installed: row.try_get("installed")?,
            });
        }
        Ok(packages)
    }
}
