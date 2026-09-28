// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Context, Result};
use lapin::{Connection, ConnectionProperties};
use std::{process::Command, time::Duration};

pub struct RabbitMq {
    id: String,
    pub connection: Connection,
}

fn docker() -> Command {
    if std::env::var_os("WINDMILL_TEST_DOCKER_SUDO").is_some() {
        let mut command = Command::new("sudo");
        command.args(["-n", "docker"]);
        command
    } else {
        Command::new("docker")
    }
}

impl RabbitMq {
    pub async fn start() -> Result<Self> {
        let name = format!("electoral-delivery-test-{}", uuid::Uuid::new_v4());
        let password = uuid::Uuid::new_v4().to_string();
        let output = docker()
            .args([
                "run",
                "--detach",
                "--rm",
                "--name",
                &name,
                "--publish",
                "127.0.0.1::5672",
                "--env",
                "RABBITMQ_DEFAULT_USER=synthetic",
                "--env",
                &format!("RABBITMQ_DEFAULT_PASS={password}"),
                "--env",
                "RABBITMQ_SERVER_ADDITIONAL_ERL_ARGS=+S 2:2",
                "rabbitmq:3.12.11-management",
            ])
            .output()
            .context("starting owned RabbitMQ fixture")?;
        if !output.status.success() {
            bail!(
                "RabbitMQ startup: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let id = String::from_utf8(output.stdout)?.trim().to_owned();
        // This guard owns only the exact container ID returned by our run.
        struct Cleanup(String);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = docker().args(["rm", "--force", &self.0]).output();
            }
        }
        let cleanup = Cleanup(id.clone());
        let port = docker().args(["port", &id, "5672/tcp"]).output()?;
        let address = String::from_utf8(port.stdout)?.trim().to_owned();
        if !address.starts_with("127.0.0.1:") {
            bail!("fixture was not published on loopback");
        }
        let url = format!("amqp://synthetic:{password}@{address}/%2f");
        let connection = tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                if let Ok(connection) =
                    Connection::connect(&url, ConnectionProperties::default()).await
                {
                    break connection;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .context("RabbitMQ readiness timed out")?;
        std::mem::forget(cleanup);
        Ok(Self { id, connection })
    }
}

impl Drop for RabbitMq {
    fn drop(&mut self) {
        let _ = docker().args(["rm", "--force", &self.id]).output();
    }
}
