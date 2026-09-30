// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The electoral log's entry for each monitoring configuration change,
//! signed by the administrator who made it.

use super::config_store::{Author, EventRef, MonitoringConfigAudit, RecordedChange};
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::newtypes::{
    MonitoringConfigChangeAction, MonitoringConfigChangeDetails, MonitoringConfigDigestString,
    MonitoringConfigKeyString, MonitoringConfigKindString, MonitoringConfigOrigin,
    MonitoringConfigRevisionRef, MonitoringDashboardMode, MonitoringPresetIdString,
    MonitoringPresetRef,
};
use sequent_core::monitoring::revision::{DashboardMode, DocumentChange, RevisionOrigin};
use std::future::Future;
use tracing::warn;

/// The entry's details for a change.
pub fn change_details(change: &RecordedChange) -> Result<MonitoringConfigChangeDetails> {
    Ok(MonitoringConfigChangeDetails {
        origin: match change.origin {
            RevisionOrigin::Editor => MonitoringConfigOrigin::Editor,
            RevisionOrigin::Preset => MonitoringConfigOrigin::Preset,
        },
        preset: change
            .preset
            .as_ref()
            .map(|preset| {
                Ok::<_, anyhow::Error>(MonitoringPresetRef {
                    id: MonitoringPresetIdString(preset.id.clone()),
                    version: u32::try_from(preset.version)
                        .context("A preset version is positive")?,
                })
            })
            .transpose()?,
        mode: match change.mode {
            DashboardMode::Legacy => MonitoringDashboardMode::Legacy,
            DashboardMode::Configured => MonitoringDashboardMode::Configured,
        },
        generation: u64::try_from(change.generation).context("A generation is not negative")?,
        revisions: change
            .revisions
            .iter()
            .map(|revision| {
                Ok(MonitoringConfigRevisionRef {
                    kind: MonitoringConfigKindString(revision.kind.to_string()),
                    key: MonitoringConfigKeyString(revision.key.clone()),
                    revision: u32::try_from(revision.revision).context("A revision is positive")?,
                    action: match revision.change {
                        DocumentChange::Upsert => MonitoringConfigChangeAction::Upsert,
                        DocumentChange::Delete => MonitoringConfigChangeAction::Delete,
                    },
                    digest: revision.digest.clone().map(MonitoringConfigDigestString),
                })
            })
            .collect::<Result<_>>()?,
    })
}

/// Posts each change to the election event's electoral log.
pub struct ElectoralLogConfigAudit;

/// The electoral-log board of the election event.
async fn board(transaction: &Transaction<'_>, event: EventRef) -> Result<String> {
    let election_event = get_election_event_by_id(
        transaction,
        &event.tenant_id.to_string(),
        &event.election_event_id.to_string(),
    )
    .await?;
    get_election_event_board(election_event.bulletin_board_reference)
        .ok_or_else(|| anyhow!("The election event has no electoral-log board"))
}

/// What preparing the author's key came to: done, if it failed but the key
/// is there anyway (`kept`), as another change by the author kept it.
async fn kept_anyway<Kept>(prepared: Result<()>, kept: impl FnOnce() -> Kept) -> Result<()>
where
    Kept: Future<Output = Result<bool>>,
{
    let Err(error) = prepared else {
        return Ok(());
    };
    match kept().await {
        Ok(true) => {
            warn!("The administrator's signing key was kept by another change: {error:?}");
            Ok(())
        }
        Ok(false) => Err(error),
        Err(looking) => Err(error.context(format!(
            "Failed to look for the administrator's signing key too: {looking:#}"
        ))),
    }
}

async fn signing_key_exists(client: &mut Client, event: EventRef, author: &Author) -> Result<bool> {
    let transaction = client
        .transaction()
        .await
        .context("Failed to start looking for the administrator's signing key")?;
    let exists = vault::admin_user_signing_key_exists(
        &transaction,
        &event.tenant_id.to_string(),
        &author.id,
    )
    .await?;
    transaction
        .commit()
        .await
        .context("Failed to finish looking for the administrator's signing key")?;
    Ok(exists)
}

async fn prepare_signing_key(client: &mut Client, event: EventRef, author: &Author) -> Result<()> {
    let transaction = client
        .transaction()
        .await
        .context("Failed to start preparing the electoral log")?;
    let board = board(&transaction, event).await?;
    vault::get_admin_user_signing_key(
        &transaction,
        &board,
        &event.tenant_id.to_string(),
        &author.id,
        author.name.clone(),
        None,
        None,
    )
    .await?;
    transaction
        .commit()
        .await
        .context("Failed to keep the administrator's signing key")
}

#[async_trait]
impl MonitoringConfigAudit for ElectoralLogConfigAudit {
    /// Makes the author's signing key, and posts its public key, unless they
    /// have one: in a transaction of its own, so a change that then rolls
    /// back does not take with it a key whose public half the log has.
    ///
    /// Two first changes by one author at once both find no key, and the
    /// second to keep one is refused. That one then finds the key the first
    /// kept, and goes on with it.
    async fn prepare(&self, client: &mut Client, event: EventRef, author: &Author) -> Result<()> {
        let prepared = prepare_signing_key(client, event, author).await;
        kept_anyway(prepared, || signing_key_exists(client, event, author)).await
    }

    async fn record(&self, transaction: &Transaction<'_>, change: &RecordedChange) -> Result<()> {
        let details = change_details(change)?;
        let tenant_id = change.event.tenant_id.to_string();
        let election_event_id = change.event.election_event_id.to_string();
        let board = board(transaction, change.event).await?;
        let electoral_log = ElectoralLog::for_admin_user(
            transaction,
            &board,
            &tenant_id,
            &election_event_id,
            &change.author.id,
            change.author.name.clone(),
            None,
            None,
        )
        .await?;
        electoral_log
            .post_monitoring_config_changed(
                election_event_id,
                details,
                Some(change.author.id.clone()),
                change.author.name.clone(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::kept_anyway;
    use anyhow::anyhow;

    #[tokio::test]
    async fn a_key_kept_by_another_change_prepares_the_change() {
        assert!(kept_anyway(Ok(()), || async { panic!("not looked for") })
            .await
            .is_ok());
        assert!(
            kept_anyway(Err(anyhow!("duplicate key")), || async { Ok(true) })
                .await
                .is_ok()
        );
        let error = kept_anyway(Err(anyhow!("the vault is down")), || async { Ok(false) })
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "the vault is down");
        let error = kept_anyway(Err(anyhow!("the vault is down")), || async {
            Err(anyhow!("so is the database"))
        })
        .await
        .unwrap_err();
        assert!(
            format!("{error:#}").contains("the vault is down"),
            "{error:#}"
        );
        assert!(
            format!("{error:#}").contains("so is the database"),
            "{error:#}"
        );
    }
}
