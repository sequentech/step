// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CeleryOpt, Queue};
use crate::services::signing::operational_device::{installed_demo, installed_profile};
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Object;
use sequent_core::signing::SigningAction;
use std::ffi::{OsStr, OsString};

const EXPECTED: &[(&str, &str)] = &[
    ("OPERATIONAL_SIGNING_DEMO_WORKER", "1"),
    ("ENV_SLUG", "operational_demo_14990"),
    ("HASURA_DB__DBNAME", "operational_demo_14990"),
    (
        "HASURA_DB__HOST",
        "/run/sequent-operational-demo-14990/postgres",
    ),
    ("HASURA_DB__PORT", "5432"),
    ("HASURA_DB__USER", "os14990_windmill"),
    ("QUEUE_DB__DBNAME", "operational_demo_queues_14990"),
    (
        "QUEUE_DB__HOST",
        "/run/sequent-operational-demo-14990/postgres",
    ),
    ("QUEUE_DB__PORT", "5432"),
    ("QUEUE_DB__USER", "os14990_queue"),
    ("AWS_S3_PRIVATE_URI", "http://127.0.0.1:19000"),
    ("AWS_S3_PUBLIC_URI", "http://127.0.0.1:19000"),
    ("AWS_S3_BUCKET", "operational-demo-14990-documents"),
    ("AWS_S3_PUBLIC_BUCKET", "operational-demo-14990-public"),
    ("AWS_EC2_METADATA_DISABLED", "true"),
    ("AWS_CONFIG_FILE", "/dev/null"),
    ("AWS_SHARED_CREDENTIALS_FILE", "/dev/null"),
    ("KEYCLOAK_URL", "https://127.0.0.1:18443"),
    ("WBRAID_B4_URL", "http://127.0.0.1:13005"),
];
const LOCAL_AWS: &[&str] = &[
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_S3_ACCESS_KEY",
    "AWS_S3_ACCESS_SECRET",
    "AWS_REGION",
];

fn destinations(
    producer: bool,
    vars: impl IntoIterator<Item = (OsString, OsString)>,
) -> Result<()> {
    let vars: std::collections::BTreeMap<_, _> = vars.into_iter().collect();
    let selector = if producer {
        "OPERATIONAL_SIGNING_DEMO_PRODUCER"
    } else {
        "OPERATIONAL_SIGNING_DEMO_WORKER"
    };
    let other = if producer {
        "OPERATIONAL_SIGNING_DEMO_WORKER"
    } else {
        "OPERATIONAL_SIGNING_DEMO_PRODUCER"
    };
    ensure!(
        !vars.contains_key(OsStr::new(other)),
        "ambiguous fixture role"
    );
    for &(raw, default) in EXPECTED {
        let key = if raw == "OPERATIONAL_SIGNING_DEMO_WORKER" {
            selector
        } else {
            raw
        };
        let val = if producer && raw == "HASURA_DB__USER" {
            "os14990_harvest"
        } else {
            default
        };
        ensure!(
            vars.get(OsStr::new(key)).map(|v| v.as_os_str()) == Some(OsStr::new(val)),
            "unexpected fixture worker configuration: {key}"
        );
    }
    for &key in LOCAL_AWS {
        ensure!(
            vars.get(OsStr::new(key)).is_some_and(|v| !v.is_empty()),
            "fixture-local storage credentials and region required"
        );
    }
    // No ambient DB URL/credentials, redirects, AWS credential profiles, proxy,
    // or case-insensitive alternative keys for the config library.
    for name in vars.keys() {
        let name = name.to_string_lossy();
        let upper = name.to_ascii_uppercase();
        if upper.starts_with("HASURA_DB")
            || upper.starts_with("QUEUE_DB")
            || upper.starts_with("AWS_")
            || upper.starts_with("KEYCLOAK_")
            || upper.starts_with("WBRAID_B4_")
        {
            ensure!(
                EXPECTED.iter().any(|(key, _)| *key == name) || LOCAL_AWS.contains(&name.as_ref()),
                "unapproved database or storage configuration for fixture worker"
            );
        }
        ensure!(
            !matches!(upper.as_str(), "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY"),
            "fixture worker does not accept ambient proxy configuration"
        );
    }
    Ok(())
}

fn configuration_with(
    opt: &CeleryOpt,
    vars: impl IntoIterator<Item = (OsString, OsString)>,
) -> Result<()> {
    destinations(false, vars)?;
    ensure!(
        matches!(opt, CeleryOpt::Consume { queues, acks_late: true, worker_threads: Some(1), .. }
           if queues.len() == 2 && queues[0] == Queue::Short.queue_name()
                    && queues[1] == Queue::ElectoralLogBeat.queue_name()),
        "only the dedicated worker queues are permitted in this fixture"
    );
    Ok(())
}

pub(super) fn configuration(opt: &CeleryOpt) -> Result<()> {
    configuration_with(opt, std::env::vars_os())
}

pub(super) async fn ready(mut client: Object) -> Result<Object> {
    let installed = installed_demo().context("missing immutable signing demo installation")?;
    let tx = client.transaction().await?;
    let safe: bool = tx
        .query_one(include_str!("operational_demo/app-ready.sql"), &[])
        .await?
        .try_get(0)?;
    ensure!(safe, "unsafe fixture Windmill login");
    ensure!(
        installed_profile(
            &tx,
            installed.tenant,
            installed.event,
            SigningAction::InitializeVoting
        )
        .await?
            == Some(installed.device),
        "isolated signing marker or provider mismatch"
    );
    tx.commit().await?;
    queue_ready().await?;
    Ok(client)
}

async fn queue_ready() -> Result<()> {
    let queue = crate::services::database::try_get_queue_pool()
        .await?
        .get()
        .await?;
    let ready: bool = queue
        .query_one(include_str!("operational_demo/queues-ready.sql"), &[])
        .await?
        .try_get(0)?;
    ensure!(ready, "fixture task queues are unavailable");
    Ok(())
}

/// Harvest is the producer of the signed-action and log-outbox work. Admit
/// it to exactly the same isolated queue and storage destinations as Windmill.
pub(super) async fn producer_ready() -> Result<()> {
    destinations(true, std::env::vars_os())?;
    queue_ready().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn fixture() -> Vec<(OsString, OsString)> {
        EXPECTED
            .iter()
            .map(|&(k, v)| (k.into(), v.into()))
            .chain(LOCAL_AWS.iter().map(|&k| (k.into(), "synthetic".into())))
            .collect()
    }
    #[test]
    fn producer_cannot_publish_to_ambient_queues_or_storage() {
        let convert = |mut v: Vec<(OsString, OsString)>| {
            for (key, val) in &mut v {
                if *key == OsStr::new("OPERATIONAL_SIGNING_DEMO_WORKER") {
                    *key = OsString::from("OPERATIONAL_SIGNING_DEMO_PRODUCER");
                } else if *key == OsStr::new("HASURA_DB__USER") {
                    *val = OsString::from("os14990_harvest");
                }
            }
            v
        };
        assert!(destinations(true, convert(fixture())).is_ok());
        assert!(destinations(true, fixture()).is_err());
        for (bad, value) in [
            ("QUEUE_DB__HOST", "hosted.invalid"),
            ("ENV_SLUG", "hosted"),
            ("AWS_S3_PUBLIC_URI", "https://external.invalid"),
            ("OPERATIONAL_SIGNING_DEMO_WORKER", "1"),
        ] {
            let mut vars = convert(fixture());
            vars.push((bad.into(), value.into()));
            assert!(destinations(true, vars).is_err());
        }
    }

    #[test]
    fn only_expected_queue_config_and_environment_can_consume() {
        let consume = CeleryOpt::parse_from([
            "windmill",
            "consume",
            "--queues",
            "short_queue",
            "electoral_log_beat_queue",
            "--worker-threads",
            "1",
        ]);
        assert!(configuration_with(&consume, fixture()).is_ok());
        for name in [
            "HASURA_DB__URL",
            "queue_db__host",
            "AWS_PROFILE",
            "HTTPS_PROXY",
        ] {
            let mut vars = fixture();
            vars.push((name.into(), "external".into()));
            assert!(configuration_with(&consume, vars).is_err());
        }
        assert!(configuration_with(&CeleryOpt::Produce, fixture()).is_err());
    }
}
