// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A disposable PostgreSQL cluster, reachable only through its private Unix
//! socket. Never reads a database URL or connects to an existing service.

use std::path::PathBuf;
use std::process::{Command, Output};
use tempfile::TempDir;
use tokio_postgres::{Client, Config, NoTls};

const DATABASE_USER: &str = "core_test";

pub struct Postgres {
    directory: TempDir,
    bin: PathBuf,
}

fn checked(output: std::io::Result<Output>) {
    let output =
        output.expect("install PostgreSQL and set PG_BIN to its bin directory");
    assert!(
        output.status.success(),
        "PostgreSQL fixture failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

impl Postgres {
    pub fn start() -> Self {
        let bin = std::env::var_os("PG_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let output = Command::new("pg_config")
                    .arg("--bindir")
                    .output()
                    .expect("install PostgreSQL and libpq-dev, or set PG_BIN");
                assert!(output.status.success(), "pg_config --bindir failed");
                PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
            });
        let directory = tempfile::tempdir().unwrap();
        checked(
            Command::new(bin.join("initdb"))
                .args(["--no-locale", "--encoding=UTF8", "--auth=trust"])
                .arg(format!("--username={DATABASE_USER}"))
                .arg("-D")
                .arg(directory.path().join("data"))
                .output(),
        );
        let server = Self { directory, bin };
        // No TCP listener, no shared database and no fixture credentials.
        // The temp directory's owner-only permissions protect the trust socket.
        checked(
            Command::new(server.bin.join("pg_ctl"))
                .arg("-D")
                .arg(server.directory.path().join("data"))
                .arg("-l")
                .arg(server.directory.path().join("postgres.log"))
                .args(["-t", "10", "-w", "-o"])
                .arg(format!(
                    "-F -h '' -k '{}'",
                    server.directory.path().display()
                ))
                .arg("start")
                .output(),
        );
        server
    }

    pub async fn connect(
        &self,
    ) -> (
        Client,
        rocket::tokio::task::JoinHandle<Result<(), tokio_postgres::Error>>,
    ) {
        let (client, connection) = Config::new()
            .host_path(self.directory.path())
            .user(DATABASE_USER)
            .dbname("postgres")
            .connect_timeout(std::time::Duration::from_secs(5))
            .connect(NoTls)
            .await
            .unwrap();
        (client, rocket::tokio::spawn(connection))
    }
}

impl Drop for Postgres {
    fn drop(&mut self) {
        // Stop the server before TempDir removes its files, also on assertion
        // failure. pg_ctl bounds startup/shutdown waits to ten seconds.
        let output = Command::new(self.bin.join("pg_ctl"))
            .arg("-D")
            .arg(self.directory.path().join("data"))
            .args(["-m", "immediate", "-t", "10", "-w", "stop"])
            .output();
        if !std::thread::panicking() {
            checked(output);
        }
    }
}
