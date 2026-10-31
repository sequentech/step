// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The daily account check. Every sending account is checked with its
//! provider once a day, and at the next pass after its identity changed
//! (its last check named another identity); the result is stored as the
//! account's status, which the account list, the event's Messaging tab
//! and the send-time gate read.
//!
//! Beat runs the pass every hour on the beat queue. Each pass checks the
//! accounts that are due, taking tenants in turn, until its time budget is
//! spent, so one tenant's accounts never starve another's. An account
//! whose check can't run backs off: its attempt is recorded, and it isn't
//! tried again for [`CHECK_RETRY_AFTER_HOURS`].

use super::accounts::{check_account, StoredCheck};
use super::dispatch::Dispatcher;
use crate::postgres::messaging::{parse_account_status, record_messaging_account_check_attempt};
use crate::services::database::get_hasura_pool;
use crate::types::error::Result;
use celery::error::TaskError;
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::messaging::{
    AccountCheck, AccountIdentity, AccountSender, MessagingProvider, ProviderSupport,
};
use std::collections::{BTreeMap, VecDeque};
use std::str::FromStr;
use std::time::Instant;
use strum_macros::Display;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// An account is checked again once its last check is this old.
pub const CHECK_EVERY_HOURS: i64 = 24;
/// An account whose last attempt didn't store a result waits this long
/// before the next one.
pub const CHECK_RETRY_AFTER_HOURS: i64 = 3;
/// Seconds between two passes.
pub const CHECK_PASS_INTERVAL_SECONDS: u64 = 3600;
/// A pass stops starting checks after this long; the accounts left wait
/// for the next pass, in the same order. A check makes a few provider
/// requests, each bounded by `PROVIDER_TIMEOUT`.
pub const CHECK_PASS_BUDGET: std::time::Duration = std::time::Duration::from_secs(20 * 60);

/// Why an account is checked in this pass. Logged with each check.
#[allow(non_camel_case_types)]
#[derive(Display, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckDue {
    /// Its last check named another identity (an earlier number, Page or
    /// sender), or was stored before checks named one.
    #[strum(serialize = "Not checked for its current identity")]
    OTHER_IDENTITY,
    #[strum(serialize = "Never checked")]
    NEVER_CHECKED,
    #[strum(serialize = "Last checked more than a day ago")]
    STALE,
}

fn parse_time(at: Option<&str>) -> Option<DateTime<Utc>> {
    at.and_then(|at| DateTime::parse_from_rfc3339(at).ok())
        .map(|at| at.with_timezone(&Utc))
}

/// Whether an account is due, and why. Accounts of removed providers are
/// never checked: they can't send, and say so. An account whose last
/// attempt couldn't run (recorded after its last check) waits
/// [`CHECK_RETRY_AFTER_HOURS`].
pub fn check_due(
    provider: MessagingProvider,
    identity: &AccountIdentity,
    status: &AccountCheck,
    now: DateTime<Utc>,
) -> Option<CheckDue> {
    if provider.support() == ProviderSupport::UNSUPPORTED {
        return None;
    }
    let checked_at = parse_time(status.checked_at.as_deref());
    let attempted_at = parse_time(status.details.attempted_at.as_deref());
    let failed_attempt = attempted_at.filter(|attempt| checked_at.map_or(true, |at| *attempt > at));
    if failed_attempt
        .is_some_and(|attempt| now - attempt < Duration::hours(CHECK_RETRY_AFTER_HOURS))
    {
        return None;
    }
    // A time that doesn't parse is treated as never checked, so the next
    // check writes a good one.
    let Some(checked_at) = checked_at else {
        return Some(CheckDue::NEVER_CHECKED);
    };
    if status.details.identity.as_ref() != Some(identity) {
        return Some(CheckDue::OTHER_IDENTITY);
    }
    (now - checked_at >= Duration::hours(CHECK_EVERY_HOURS)).then_some(CheckDue::STALE)
}

/// An account due for its check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueAccount {
    pub tenant_id: Uuid,
    pub id: Uuid,
    pub due: CheckDue,
    /// Its last check or attempt, whichever is later: the longest waiting
    /// go first.
    last_activity: Option<DateTime<Utc>>,
}

/// Tenants in turn: each tenant's accounts in their order, one tenant's
/// next account after every other tenant's, so a tenant with many due
/// accounts doesn't keep the others waiting. Tenants start in the order
/// of their first account.
pub fn round_robin(due: Vec<DueAccount>) -> Vec<DueAccount> {
    let mut order: Vec<Uuid> = vec![];
    let mut queues: BTreeMap<Uuid, VecDeque<DueAccount>> = BTreeMap::new();
    for account in due {
        if !queues.contains_key(&account.tenant_id) {
            order.push(account.tenant_id);
        }
        queues
            .entry(account.tenant_id)
            .or_default()
            .push_back(account);
    }
    let mut turns = vec![];
    loop {
        let mut took = false;
        for tenant in &order {
            if let Some(account) = queues.get_mut(tenant).and_then(VecDeque::pop_front) {
                turns.push(account);
                took = true;
            }
        }
        if !took {
            return turns;
        }
    }
}

/// The accounts due at `now`, across tenants: those whose check is of
/// another identity first, then never checked, then stale; the longest
/// waiting first within each; tenants in turn ([`round_robin`]).
pub async fn due_accounts(
    tx: &Transaction<'_>,
    now: DateTime<Utc>,
) -> anyhow::Result<Vec<DueAccount>> {
    let rows = tx
        .query(
            r#"
            SELECT tenant_id, id, provider, sender, status
            FROM sequent_backend.messaging_account
            "#,
            &[],
        )
        .await?;
    let mut due = vec![];
    for row in rows {
        let id: Uuid = row.try_get("id")?;
        let provider: String = row.try_get("provider")?;
        // A row without a readable provider or sender has no identity to
        // check: it is logged on every pass, never the whole pass lost.
        let parsed = MessagingProvider::from_str(&provider)
            .ok()
            .zip(serde_json::from_value::<AccountSender>(row.try_get("sender")?).ok());
        let Some((provider, sender)) = parsed else {
            warn!(account = %id, "account check skipped: unreadable provider or sender");
            continue;
        };
        // An unreadable status is logged and taken as never checked: the
        // account stays due, so the bad row is seen on every pass.
        let status = parse_account_status(row.try_get("status")?).unwrap_or_else(|error| {
            warn!(account = %id, "account status unreadable, checked as never checked: {error:#}");
            AccountCheck::default()
        });
        if let Some(reason) = check_due(provider, &sender.identity(), &status, now) {
            due.push(DueAccount {
                tenant_id: row.try_get("tenant_id")?,
                id,
                due: reason,
                last_activity: parse_time(status.checked_at.as_deref())
                    .max(parse_time(status.details.attempted_at.as_deref())),
            });
        }
    }
    due.sort_by(|a, b| (a.due, a.last_activity).cmp(&(b.due, b.last_activity)));
    Ok(round_robin(due))
}

/// One pass: checks the accounts that are due, each in its own transaction,
/// until [`CHECK_PASS_BUDGET`] is spent. An account whose check can't run
/// gets its attempt recorded and backs off; one whose result is dropped
/// because it changed meanwhile is checked again at the next pass. Returns
/// how many were stored.
pub async fn check_due_accounts(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
) -> anyhow::Result<usize> {
    let started = Instant::now();
    let tx = client.transaction().await?;
    let due = due_accounts(&tx, Utc::now()).await?;
    tx.commit().await?;
    let total = due.len();
    let mut checked = 0;
    for (done, account) in due.into_iter().enumerate() {
        if started.elapsed() > CHECK_PASS_BUDGET {
            info!(
                left = total - done,
                "account check pass stopped at its time budget; the rest wait for the next pass"
            );
            break;
        }
        let attempt = Utc::now();
        let tx = client.transaction().await?;
        let stored = match check_account(
            &tx,
            &account.tenant_id,
            &account.id,
            &dispatcher.endpoints,
        )
        .await
        {
            Ok(Some(StoredCheck {
                check,
                stored: true,
            })) => {
                tx.commit().await?;
                checked += 1;
                info!(
                    account = %account.id,
                    due = %account.due,
                    connected = check.connected,
                    production_access = check.production_access,
                    reason = check.reason.as_deref().unwrap_or("-"),
                    "account checked"
                );
                true
            }
            // The account changed while it was checked: the next pass
            // checks it as it is now, without backing off.
            Ok(Some(StoredCheck { stored: false, .. })) => {
                tx.commit().await?;
                true
            }
            Ok(None) => {
                tx.rollback().await?;
                true
            }
            Err(error) => {
                tx.rollback().await?;
                warn!(account = %account.id, due = %account.due, "account check could not run: {error:#}");
                false
            }
        };
        if !stored {
            let tx = client.transaction().await?;
            record_messaging_account_check_attempt(&tx, &account.tenant_id, &account.id, attempt)
                .await?;
            tx.commit().await?;
        }
    }
    Ok(checked)
}

/// The daily account check (plan §2.7), on the beat queue.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(expires = 3000, max_retries = 0)]
pub async fn check_messaging_accounts() -> Result<()> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting hasura db pool: {err}"))?;
    let dispatcher = Dispatcher::global()
        .await
        .map_err(|err| format!("Error creating the message dispatcher: {err}"))?;
    check_due_accounts(&mut client, dispatcher)
        .await
        .map_err(|err| format!("Error in the account check pass: {err:#}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn sns(sender_id: &str) -> AccountSender {
        AccountSender::AWS_SNS {
            sender_id: Some(sender_id.to_string()),
            origination_number: None,
            region: None,
        }
    }

    fn status(checked_at: Option<&str>, identity: Option<AccountIdentity>) -> AccountCheck {
        let mut status = AccountCheck {
            checked_at: checked_at.map(str::to_string),
            ..Default::default()
        };
        status.details.identity = identity;
        status
    }

    #[test]
    fn accounts_are_due_daily_after_an_identity_change_or_when_never_checked() {
        let now = at("2026-10-10T12:00:00Z");
        let provider = MessagingProvider::AWS_SNS;
        let current = sns("Elections").identity();
        let due = |status: &AccountCheck| check_due(provider, &current, status, now);
        assert_eq!(due(&status(None, None)), Some(CheckDue::NEVER_CHECKED));
        assert_eq!(
            due(&status(Some("not a time"), Some(current.clone()))),
            Some(CheckDue::NEVER_CHECKED)
        );
        assert_eq!(
            due(&status(Some("2026-10-09T12:00:01Z"), Some(current.clone()))),
            None
        );
        assert_eq!(
            due(&status(
                Some("2026-10-09T12:00:00+00:00"),
                Some(current.clone())
            )),
            Some(CheckDue::STALE)
        );
        // A recent check of another identity, or of an unrecorded one.
        assert_eq!(
            due(&status(
                Some("2026-10-10T11:59:00Z"),
                Some(sns("Other").identity())
            )),
            Some(CheckDue::OTHER_IDENTITY)
        );
        assert_eq!(
            due(&status(Some("2026-10-10T11:59:00Z"), None)),
            Some(CheckDue::OTHER_IDENTITY)
        );
        for removed in [MessagingProvider::CONSOLE, MessagingProvider::HTTP_API] {
            assert_eq!(check_due(removed, &current, &status(None, None), now), None);
        }
    }

    /// R5-14: an account whose check couldn't run kept its old check time
    /// and stayed first in every pass. Its attempt is recorded, and it
    /// waits before the next one.
    #[test]
    fn an_account_whose_check_fails_to_run_backs_off() {
        let now = at("2026-10-10T12:00:00Z");
        let current = sns("Elections").identity();
        let mut failed = status(Some("2026-10-08T12:00:00Z"), Some(current.clone()));
        failed.details.attempted_at = Some("2026-10-10T11:00:00Z".to_string());
        assert_eq!(
            check_due(MessagingProvider::AWS_SNS, &current, &failed, now),
            None
        );
        failed.details.attempted_at = Some("2026-10-10T08:59:00Z".to_string());
        assert_eq!(
            check_due(MessagingProvider::AWS_SNS, &current, &failed, now),
            Some(CheckDue::STALE)
        );
        // A never-checked account backs off the same way.
        let mut never = status(None, None);
        never.details.attempted_at = Some("2026-10-10T11:30:00Z".to_string());
        assert_eq!(
            check_due(MessagingProvider::AWS_SNS, &current, &never, now),
            None
        );
    }

    /// R5-14: tenants take turns, so one tenant's many due accounts don't
    /// keep another's waiting until the budget runs out.
    #[test]
    fn tenants_take_turns() {
        let tenant = |n: u128| Uuid::from_u128(n);
        let account = |t: u128, n: u128| DueAccount {
            tenant_id: tenant(t),
            id: Uuid::from_u128(t * 100 + n),
            due: CheckDue::STALE,
            last_activity: None,
        };
        let order: Vec<Uuid> = round_robin(vec![
            account(1, 1),
            account(1, 2),
            account(1, 3),
            account(2, 1),
            account(3, 1),
            account(2, 2),
        ])
        .into_iter()
        .map(|account| account.id)
        .collect();
        assert_eq!(
            order,
            [101, 201, 301, 102, 202, 103].map(Uuid::from_u128).to_vec()
        );
    }

    #[test]
    fn each_reason_has_one_text() {
        assert_eq!(
            CheckDue::OTHER_IDENTITY.to_string(),
            "Not checked for its current identity"
        );
        assert_eq!(CheckDue::NEVER_CHECKED.to_string(), "Never checked");
        assert_eq!(
            CheckDue::STALE.to_string(),
            "Last checked more than a day ago"
        );
    }
}
