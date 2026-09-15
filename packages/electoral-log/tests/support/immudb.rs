// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Each integration test owns its database process and directory. No deployment
//! URL, account, or existing database is accepted by this fixture.

use anyhow::{anyhow, Context, Result};
use electoral_log::BoardClient;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

pub const DATABASE: &str = "electoralcoveragetest";
pub const USERNAME: &str = "immudb";

pub struct DatabaseServer {
    process: Child,
    directory: TempDir,
    pub url: String,
    pub password: String,
}

impl DatabaseServer {
    pub async fn start() -> Result<Self> {
        Self::start_with_first_port(None).await
    }

    pub async fn start_with_first_port(first_port: Option<u16>) -> Result<Self> {
        for attempt in 0..5 {
            let port = if attempt == 0 && first_port.is_some() {
                first_port.unwrap()
            } else {
                let listener = TcpListener::bind(("127.0.0.1", 0))?;
                listener.local_addr()?.port()
            };
            match Self::start_on_port(port).await {
                Ok(server) => return Ok(server),
                Err(error)
                    if attempt < 4 && error.to_string().contains("address already in use") => {}
                Err(error) => return Err(error),
            }
        }
        unreachable!("the last startup attempt returns its error")
    }

    async fn start_on_port(port: u16) -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let password = format!(
            "synthetic-{}",
            directory.path().file_name().unwrap().to_string_lossy()
        );

        let output = std::fs::File::create(directory.path().join("server.log"))?;
        let binary =
            std::env::var_os("ELECTORAL_LOG_TEST_IMMUDB_BINARY").unwrap_or_else(|| "immudb".into());
        let process = Command::new(binary)
            .args([
                "--address",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--admin-password",
                &password,
            ])
            .args([
                "--auth",
                "--web-server=false",
                "--metrics-server=false",
                "--pgsql-server=false",
            ])
            .arg("--dir")
            .arg(directory.path().join("data"))
            .current_dir(directory.path())
            .stdin(Stdio::null())
            .stdout(output.try_clone()?)
            .stderr(output)
            .spawn()
            .context("install ImmuDB 1.9.6 or set ELECTORAL_LOG_TEST_IMMUDB_BINARY")?;
        let mut server = Self {
            process,
            directory,
            url: format!("http://127.0.0.1:{port}"),
            password,
        };

        for _ in 0..100 {
            if server.process.try_wait()?.is_some() {
                return Err(anyhow!("ImmuDB exited during startup: {}", server.logs()));
            }
            if let Ok(Ok(_)) =
                tokio::time::timeout(Duration::from_millis(200), server.client()).await
            {
                // The per-process password prevents adopting another fixture
                // that won the port race. Also check that our child is alive.
                if server.process.try_wait()?.is_none() {
                    return Ok(server);
                }
                return Err(anyhow!("ImmuDB exited during startup: {}", server.logs()));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(anyhow!("ImmuDB did not become ready: {}", server.logs()))
    }

    pub async fn client(&self) -> Result<BoardClient> {
        BoardClient::new(&self.url, USERNAME, &self.password).await
    }

    pub fn logs(&self) -> String {
        std::fs::read_to_string(self.directory.path().join("server.log")).unwrap_or_default()
    }
}

impl Drop for DatabaseServer {
    fn drop(&mut self) {
        // Reap the child before TempDir removes files, including after a failed
        // assertion. The fixture never leaves a listener or database behind.
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
