// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, bail, Context, Result};
use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration as StdDuration, Instant};
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tokio_postgres::{Client, NoTls};

struct LocalPostgresProcess {
    data_dir: TempDir,
}

impl Drop for LocalPostgresProcess {
    fn drop(&mut self) {
        let _ = Command::new("pg_ctl")
            .args(["-D"])
            .arg(self.data_dir.path())
            .args(["-m", "immediate", "stop"])
            .output();
    }
}

pub(crate) struct TestDatabase {
    pub(crate) client: Client,
    connection_task: JoinHandle<()>,
    _local_process: Option<LocalPostgresProcess>,
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        self.connection_task.abort();
    }
}

impl TestDatabase {
    pub(crate) async fn bootstrap() -> Result<Self> {
        let (database_url, local_process) = match env::var("WINDMILL_TEST_DATABASE_URL") {
            Ok(database_url) => (database_url, None),
            Err(_) => {
                let (database_url, local_process) = start_local_postgres_process()?;
                (database_url, Some(local_process))
            }
        };

        let start = Instant::now();
        let (client, connection) = loop {
            match tokio_postgres::connect(database_url.as_str(), NoTls).await {
                Ok(connection) => break connection,
                Err(error) if start.elapsed() < StdDuration::from_secs(30) => {
                    tokio::time::sleep(StdDuration::from_millis(500)).await;
                    let _ = error;
                }
                Err(error) => {
                    return Err(anyhow!(
                        "Could not connect to test Postgres at {}: {}",
                        database_url,
                        error
                    ));
                }
            }
        };

        let connection_task = tokio::spawn(async move {
            if let Err(error) = connection.await {
                panic!("Postgres connection error: {error}");
            }
        });

        bootstrap_backend_database(&client).await?;

        Ok(Self {
            client,
            connection_task,
            _local_process: local_process,
        })
    }
}

async fn bootstrap_backend_database(client: &Client) -> Result<()> {
    client
        .batch_execute("DROP SCHEMA IF EXISTS sequent_backend CASCADE;")
        .await?;

    let init_sql = extract_postgres_init_sql()?;
    client.batch_execute(init_sql.as_str()).await?;

    for migration in backend_migration_paths()? {
        let migration_sql = fs::read_to_string(&migration)
            .with_context(|| format!("Could not read backend migration {}", migration.display()))?;
        client
            .batch_execute(migration_sql.as_str())
            .await
            .with_context(|| {
                format!("Could not apply backend migration {}", migration.display())
            })?;
    }

    Ok(())
}

fn extract_postgres_init_sql() -> Result<String> {
    let init_script_path = repo_root().join(".devcontainer/postgresql/init.sh");
    let init_script = fs::read_to_string(&init_script_path)
        .with_context(|| format!("Could not read {}", init_script_path.display()))?;

    let mut in_sql_block = false;
    let mut sql_lines = Vec::new();

    for line in init_script.lines() {
        if in_sql_block {
            if line.trim() == "EOSQL" {
                in_sql_block = false;
                continue;
            }
            sql_lines.push(line);
        } else if is_eosql_heredoc_start(line) {
            in_sql_block = true;
        }
    }

    if sql_lines.is_empty() {
        bail!(
            "Could not extract SQL bootstrap block from {}",
            init_script_path.display()
        );
    }

    Ok(sql_lines.join("\n"))
}

fn is_eosql_heredoc_start(line: &str) -> bool {
    let Some(heredoc_start) = line.find("<<") else {
        return false;
    };

    let mut chars = line[heredoc_start + 2..].chars().peekable();

    while let Some(&next) = chars.peek() {
        if next == '-' || next.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    let rest: String = chars.collect();

    rest.starts_with("'EOSQL'") || rest.starts_with("\"EOSQL\"") || rest.starts_with("EOSQL")
}

fn backend_migration_paths() -> Result<Vec<PathBuf>> {
    let migrations_dir = repo_root().join("hasura/migrations/backend-db");
    let mut migrations = fs::read_dir(&migrations_dir)
        .with_context(|| format!("Could not read {}", migrations_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path().join("up.sql")))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();

    migrations.sort();

    if migrations.is_empty() {
        bail!(
            "Could not find backend migrations in {}",
            migrations_dir.display()
        );
    }

    Ok(migrations)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("windmill crate should live under the repository packages directory")
        .to_path_buf()
}

fn start_local_postgres_process() -> Result<(String, LocalPostgresProcess)> {
    let data_dir = TempDir::new().context("Could not create temp dir for local Postgres data")?;
    let log_file_path = data_dir.path().join("postgres.log");
    let listener = TcpListener::bind("127.0.0.1:0")
        .context("Could not reserve a local TCP port for Postgres")?;
    let port = listener
        .local_addr()
        .context("Could not inspect reserved Postgres port")?
        .port();
    drop(listener);

    let server_options = format!("-F -h 127.0.0.1 -k {} -p {port}", data_dir.path().display());

    let initdb_output = Command::new("initdb")
        .arg("-D")
        .arg(data_dir.path())
        .args(["--username=postgres", "--auth=trust"])
        .output()
        .context("Could not initialize a local Postgres data directory")?;

    if !initdb_output.status.success() {
        bail!(
            "Could not initialize local Postgres: {}",
            String::from_utf8_lossy(&initdb_output.stderr)
        );
    }

    let start_output = Command::new("pg_ctl")
        .arg("-D")
        .arg(data_dir.path())
        .arg("-l")
        .arg(&log_file_path)
        .arg("-o")
        .arg(server_options.as_str())
        .args(["-w", "start"])
        .output()
        .context("Could not start a local Postgres process")?;

    if !start_output.status.success() {
        let log_output = fs::read_to_string(&log_file_path).unwrap_or_default();
        bail!(
            "Could not start local Postgres: {}{}{}",
            String::from_utf8_lossy(&start_output.stderr),
            if log_output.is_empty() { "" } else { "\n" },
            log_output
        );
    }

    Ok((
        format!("postgresql://postgres@127.0.0.1:{port}/postgres"),
        LocalPostgresProcess { data_dir },
    ))
}

#[cfg(test)]
mod tests {
    use super::is_eosql_heredoc_start;

    #[test]
    fn detects_quoted_eosql_heredoc_start() {
        assert!(is_eosql_heredoc_start("    <<-'EOSQL'"));
        assert!(is_eosql_heredoc_start("<<'EOSQL'"));
    }

    #[test]
    fn detects_unquoted_eosql_heredoc_start() {
        assert!(is_eosql_heredoc_start("<<-EOSQL"));
        assert!(is_eosql_heredoc_start("<< EOSQL"));
        assert!(is_eosql_heredoc_start("<<- 'EOSQL'"));
    }

    #[test]
    fn ignores_other_heredoc_tokens() {
        assert!(!is_eosql_heredoc_start("<<-SQL"));
        assert!(!is_eosql_heredoc_start("echo hi"));
    }
}
