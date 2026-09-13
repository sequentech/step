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
pub const PASSWORD: &str = "immudb";

pub struct DatabaseServer {
    process: Child,
    directory: TempDir,
    pub url: String,
}

impl DatabaseServer {
    pub async fn start() -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        drop(listener);

        let output = std::fs::File::create(directory.path().join("server.log"))?;
        let binary =
            std::env::var_os("ELECTORAL_LOG_TEST_IMMUDB_BINARY").unwrap_or_else(|| "immudb".into());
        let process = Command::new(binary)
            .args(["--address", "127.0.0.1", "--port", &port.to_string()])
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
        };

        for _ in 0..100 {
            if server.process.try_wait()?.is_some() {
                return Err(anyhow!("ImmuDB exited during startup: {}", server.logs()));
            }
            if let Ok(Ok(_)) =
                tokio::time::timeout(Duration::from_millis(200), server.client()).await
            {
                return Ok(server);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(anyhow!("ImmuDB did not become ready: {}", server.logs()))
    }

    pub async fn client(&self) -> Result<BoardClient> {
        BoardClient::new(&self.url, USERNAME, PASSWORD).await
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
