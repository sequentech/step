// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Installs PGMQ and an environment's queues in its task-queue database.
//! The database owner runs it when the environment is provisioned or upgraded.

use crate::validate_queue_name;
use tokio_postgres::Transaction;

/// The PGMQ release embedded in this crate.
pub const PGMQ_VERSION: &str = "1.13.0";
const PGMQ_SQL: &str = include_str!("../sql/pgmq-1.13.0.sql");
const SETUP_SQL: &str = include_str!("../sql/setup.sql");
/// Serializes concurrent setups of the same database.
const SETUP_LOCK: &str = "step_queue.setup";

/// The roles that setup grants queue access to.
#[derive(Debug, Clone)]
pub struct Roles {
    /// Workers and the scheduler: read, send, archive and delete messages.
    pub worker: String,
    /// Services that only enqueue tasks.
    pub producer: String,
    /// Read-only inspection of queues and their archives.
    pub reader: String,
}

/// What to set up.
#[derive(Debug, Clone)]
pub struct Installation<'a> {
    /// The environment the database belongs to; brokers refuse a database of another one.
    pub environment: &'a str,
    pub queues: &'a [&'a str],
    /// `None` leaves privileges unchanged, for a database used only by its owner.
    pub roles: Option<&'a Roles>,
}

/// Whether setup installed PGMQ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgmqInstallation {
    Installed,
    AlreadyInstalled,
}

#[derive(Debug)]
pub enum SetupError {
    InvalidEnvironment,
    InvalidQueueName(String),
    InvalidRole(String),
    Database(tokio_postgres::Error),
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::InvalidEnvironment => write!(f, "the environment name is empty"),
            SetupError::InvalidQueueName(reason) => f.write_str(reason),
            SetupError::InvalidRole(role) => write!(f, "invalid role name {role:?}"),
            SetupError::Database(_) => write!(f, "task-queue database setup failed"),
        }
    }
}

impl std::error::Error for SetupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SetupError::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<tokio_postgres::Error> for SetupError {
    fn from(error: tokio_postgres::Error) -> Self {
        SetupError::Database(error)
    }
}

/// Install PGMQ if the database does not have it, record the environment, create the
/// queues and grant the roles access to them. Run it in a transaction of the database
/// owner and commit it; it is idempotent.
pub async fn setup(
    tx: &Transaction<'_>,
    installation: &Installation<'_>,
) -> Result<PgmqInstallation, SetupError> {
    if installation.environment.is_empty() {
        return Err(SetupError::InvalidEnvironment);
    }
    for queue in installation.queues {
        validate_queue_name(queue).map_err(SetupError::InvalidQueueName)?;
    }
    let roles = installation.roles.map(|roles| {
        [
            roles.worker.as_str(),
            roles.producer.as_str(),
            roles.reader.as_str(),
        ]
    });
    for role in roles.iter().flatten() {
        if role.is_empty() {
            return Err(SetupError::InvalidRole(role.to_string()));
        }
    }
    let [worker, producer, reader] = roles.unwrap_or(["", "", ""]);

    tx.execute(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        &[&SETUP_LOCK],
    )
    .await?;
    let installed: bool = tx
        .query_one("SELECT to_regclass('pgmq.meta') IS NOT NULL", &[])
        .await?
        .get(0);
    let pgmq = if installed {
        PgmqInstallation::AlreadyInstalled
    } else {
        tx.batch_execute(PGMQ_SQL).await?;
        PgmqInstallation::Installed
    };
    let queues = installation.queues.join(",");
    for (name, value) in [
        ("step.queue.environment", installation.environment),
        ("step.queue.pgmq_version", PGMQ_VERSION),
        ("step.queue.queues", queues.as_str()),
        ("step.queue.worker_role", worker),
        ("step.queue.producer_role", producer),
        ("step.queue.reader_role", reader),
    ] {
        tx.execute("SELECT set_config($1, $2, true)", &[&name, &value])
            .await?;
    }
    tx.batch_execute(SETUP_SQL).await?;
    Ok(pgmq)
}
