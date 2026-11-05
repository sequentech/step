// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! How a report held for its signatures is released once its request runs,
//! and the e-mail it holds back.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumString};
use tokio_postgres::Row;
use tracing::instrument;
use uuid::Uuid;

/// The e-mail a scheduled report sends with its document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportEmail {
    pub recipients: Vec<String>,
    pub subject: String,
    pub plaintext_body: String,
    pub html_body: Option<String>,
}

/// Where a released report goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseTarget {
    /// A report of the Reports tab, under the document id its generation
    /// answered.
    Report { document_id: Uuid },
    /// A tally's report of a Post (and country): the PDF of its results row.
    TallyResult {
        results_event_id: Uuid,
        election_id: Uuid,
        area_id: Option<Uuid>,
        report_type: String,
    },
}

/// Whether a released report is wrapped in its configured password, as an
/// unsigned one would have been.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum ReleaseEncryption {
    #[strum(serialize = "none")]
    NoEncryption,
    ConfiguredPassword,
}

/// What releasing a signed report does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportRelease {
    pub report_id: Option<Uuid>,
    pub target: ReleaseTarget,
    pub file_name: String,
    pub is_public: bool,
    pub encryption: ReleaseEncryption,
    /// Sent with the released document after the release commits; a report
    /// not due by mail has none.
    pub email: Option<ReportEmail>,
}

/// The document a release stored, which the held e-mail attaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasedDocument {
    pub document_id: Uuid,
    pub name: String,
    pub media_type: String,
    /// Wrapped in a password: opened through the password flow, not printed
    /// directly.
    pub protected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReportReleaseRow {
    pub request_id: Uuid,
    pub release: ReportRelease,
    pub released_at: Option<DateTime<Utc>>,
    pub released: Option<ReleasedDocument>,
    pub mail_started_at: Option<DateTime<Utc>>,
    pub mailed_at: Option<DateTime<Utc>>,
    pub mail_error: Option<String>,
}

const TARGET_REPORT: &str = "report";
const TARGET_TALLY_RESULT: &str = "tally-result";

impl TryFrom<Row> for ReportReleaseRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        let email: Option<Value> = row.try_get("email")?;
        let target: String = row.try_get("target")?;
        let target = match target.as_str() {
            TARGET_REPORT => ReleaseTarget::Report {
                document_id: row
                    .try_get::<_, Option<Uuid>>("document_id")?
                    .ok_or_else(|| anyhow!("A report release names its document"))?,
            },
            TARGET_TALLY_RESULT => ReleaseTarget::TallyResult {
                results_event_id: row
                    .try_get::<_, Option<Uuid>>("results_event_id")?
                    .ok_or_else(|| anyhow!("A tally release names its results"))?,
                election_id: row
                    .try_get::<_, Option<Uuid>>("election_id")?
                    .ok_or_else(|| anyhow!("A tally release names its Post"))?,
                area_id: row.try_get("area_id")?,
                report_type: row
                    .try_get::<_, Option<String>>("report_type")?
                    .ok_or_else(|| anyhow!("A tally release names its report type"))?,
            },
            other => return Err(anyhow!("Unknown release target {other}")),
        };
        let encryption: String = row.try_get("encryption")?;
        let released_document_id: Option<Uuid> = row.try_get("released_document_id")?;
        Ok(ReportReleaseRow {
            request_id: row.try_get("request_id")?,
            release: ReportRelease {
                report_id: row.try_get("report_id")?,
                target,
                file_name: row.try_get("file_name")?,
                is_public: row.try_get("is_public")?,
                encryption: encryption
                    .parse()
                    .map_err(|_| anyhow!("Unknown release encryption {encryption}"))?,
                email: email
                    .map(serde_json::from_value)
                    .transpose()
                    .context("Error reading the report's e-mail")?,
            },
            released_at: row.try_get("released_at")?,
            released: match released_document_id {
                Some(document_id) => Some(ReleasedDocument {
                    document_id,
                    name: row
                        .try_get::<_, Option<String>>("released_name")?
                        .unwrap_or_default(),
                    media_type: row
                        .try_get::<_, Option<String>>("released_media_type")?
                        .unwrap_or_default(),
                    protected: false,
                }),
                None => None,
            },
            mail_started_at: row.try_get("mail_started_at")?,
            mailed_at: row.try_get("mailed_at")?,
            mail_error: row.try_get("mail_error")?,
        })
    }
}

/// Keeps how request `request_id` releases its report. A request that
/// already has one keeps it: a generation that answered the same waiting
/// request doesn't move its release. Whether it was kept now.
#[instrument(skip(hasura_transaction, release), err)]
pub async fn insert_report_release(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    release: &ReportRelease,
) -> Result<bool> {
    let email = release
        .email
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let (target, document_id, results_event_id, election_id, area_id, report_type) =
        match &release.target {
            ReleaseTarget::Report { document_id } => {
                (TARGET_REPORT, Some(*document_id), None, None, None, None)
            }
            ReleaseTarget::TallyResult {
                results_event_id,
                election_id,
                area_id,
                report_type,
            } => (
                TARGET_TALLY_RESULT,
                None,
                Some(*results_event_id),
                Some(*election_id),
                *area_id,
                Some(report_type.clone()),
            ),
        };
    let inserted = hasura_transaction
        .execute(
            "INSERT INTO sequent_backend.signing_report_release
                 (request_id, tenant_id, election_event_id, target, report_id, document_id,
                  results_event_id, election_id, area_id, report_type, file_name, is_public,
                  encryption, email)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (request_id) DO NOTHING",
            &[
                &request_id,
                &tenant_id,
                &election_event_id,
                &target,
                &release.report_id,
                &document_id,
                &results_event_id,
                &election_id,
                &area_id,
                &report_type,
                &release.file_name,
                &release.is_public,
                &release.encryption.to_string(),
                &email,
            ],
        )
        .await
        .context("Error keeping the report's release")?;
    Ok(inserted == 1)
}

/// The release of a request; `lock` takes its row `FOR UPDATE`.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_report_release(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    lock: bool,
) -> Result<Option<ReportReleaseRow>> {
    let sql = format!(
        "SELECT * FROM sequent_backend.signing_report_release
         WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3{}",
        if lock { " FOR UPDATE" } else { "" }
    );
    hasura_transaction
        .query_opt(&sql, &[&tenant_id, &election_event_id, &request_id])
        .await
        .context("Error reading the report's release")?
        .map(ReportReleaseRow::try_from)
        .transpose()
}

#[instrument(skip(hasura_transaction), err)]
pub async fn mark_report_released(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    released: &ReleasedDocument,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.signing_report_release
             SET released_at = clock_timestamp(), released_document_id = $4,
                 released_name = $5, released_media_type = $6
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3",
            &[
                &tenant_id,
                &election_event_id,
                &request_id,
                &released.document_id,
                &released.name,
                &released.media_type,
            ],
        )
        .await
        .context("Error marking the report released")?;
    Ok(())
}

/// Claims the held e-mail of a released report for its one send: `true`
/// once.
#[instrument(skip(hasura_transaction), err)]
pub async fn claim_report_mail(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
) -> Result<bool> {
    Ok(hasura_transaction
        .execute(
            "UPDATE sequent_backend.signing_report_release SET mail_started_at = clock_timestamp()
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3
                 AND email IS NOT NULL AND released_at IS NOT NULL
                 AND mail_started_at IS NULL",
            &[&tenant_id, &election_event_id, &request_id],
        )
        .await
        .context("Error claiming the report's e-mail")?
        == 1)
}

/// Records how the held e-mail's one send went.
#[instrument(skip(hasura_transaction), err)]
pub async fn finish_report_mail(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    error: Option<&str>,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.signing_report_release
             SET mailed_at = CASE WHEN $4::text IS NULL THEN clock_timestamp() END,
                 mail_error = $4
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3",
            &[&tenant_id, &election_event_id, &request_id, &error],
        )
        .await
        .context("Error recording the report's e-mail")?;
    Ok(())
}

/// Whether a signing request waits for the configured report `report_id`:
/// its next scheduled generation is skipped instead of replacing it.
#[instrument(skip(hasura_transaction), err)]
pub async fn report_awaits_signatures(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    report_id: Uuid,
) -> Result<bool> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT 1 FROM sequent_backend.signing_report_release r
             JOIN sequent_backend.signing_request s
                 ON s.tenant_id = r.tenant_id AND s.election_event_id = r.election_event_id
                    AND s.id = r.request_id
             WHERE r.tenant_id = $1 AND r.election_event_id = $2 AND r.report_id = $3
                 AND s.status = 'waiting'
             LIMIT 1",
            &[&tenant_id, &election_event_id, &report_id],
        )
        .await
        .context("Error reading the report's waiting request")?
        .is_some())
}

/// Whether the country `area_id` votes in the Post `election_id`.
#[instrument(skip(hasura_transaction), err)]
pub async fn area_votes_in_post(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
    area_id: Uuid,
) -> Result<bool> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT 1 FROM sequent_backend.area_contest ac
             JOIN sequent_backend.contest c
                 ON c.tenant_id = ac.tenant_id AND c.election_event_id = ac.election_event_id
                    AND c.id = ac.contest_id
             WHERE ac.tenant_id = $1 AND ac.election_event_id = $2 AND c.election_id = $3
                 AND ac.area_id = $4
             LIMIT 1",
            &[&tenant_id, &election_event_id, &election_id, &area_id],
        )
        .await
        .context("Error reading the Post's countries")?
        .is_some())
}

/// Sets the PDF of a tally's results row (a Post, or a Post and country).
#[instrument(skip(hasura_transaction), err)]
pub async fn set_result_pdf(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    results_event_id: Uuid,
    election_id: Uuid,
    area_id: Option<Uuid>,
    document_id: Uuid,
) -> Result<()> {
    let document_id = document_id.to_string();
    let updated = match area_id {
        Some(area_id) => {
            hasura_transaction
                .execute(
                    "UPDATE sequent_backend.results_election_area
                     SET documents = jsonb_set(COALESCE(documents, '{}'), '{pdf}', to_jsonb($6::text))
                     WHERE tenant_id = $1 AND election_event_id = $2 AND results_event_id = $3
                         AND election_id = $4 AND area_id = $5",
                    &[
                        &tenant_id,
                        &election_event_id,
                        &results_event_id,
                        &election_id,
                        &area_id,
                        &document_id,
                    ],
                )
                .await
        }
        None => {
            hasura_transaction
                .execute(
                    "UPDATE sequent_backend.results_election
                     SET documents = jsonb_set(COALESCE(documents, '{}'), '{pdf}', to_jsonb($5::text))
                     WHERE tenant_id = $1 AND election_event_id = $2 AND results_event_id = $3
                         AND election_id = $4",
                    &[
                        &tenant_id,
                        &election_event_id,
                        &results_event_id,
                        &election_id,
                        &document_id,
                    ],
                )
                .await
        }
    }
    .context("Error setting the results' PDF")?;
    if updated == 0 {
        return Err(anyhow!("The results row of the released report is gone"));
    }
    Ok(())
}

/// A tally's report held for its signatures whose request may not have
/// started yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TallyHoldRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub results_event_id: Uuid,
    pub action: String,
    pub report_type: String,
    pub election_id: Uuid,
    pub area_id: Option<Uuid>,
    pub file_name: String,
    pub base_document_id: Uuid,
    pub base_sha256: String,
    /// Who ran the tally: the request's requester.
    pub requested_by: String,
    pub requested_by_username: String,
    pub request_id: Option<Uuid>,
    pub error: Option<String>,
}

impl TryFrom<Row> for TallyHoldRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(TallyHoldRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            results_event_id: row.try_get("results_event_id")?,
            action: row.try_get("action")?,
            report_type: row.try_get("report_type")?,
            election_id: row.try_get("election_id")?,
            area_id: row.try_get("area_id")?,
            file_name: row.try_get("file_name")?,
            base_document_id: row.try_get("base_document_id")?,
            base_sha256: row.try_get("base_sha256")?,
            requested_by: row.try_get("requested_by")?,
            requested_by_username: row.try_get("requested_by_username")?,
            request_id: row.try_get("request_id")?,
            error: row.try_get("error")?,
        })
    }
}

/// Records a held tally report; its request starts after the tally's
/// transaction commits.
#[instrument(skip(hasura_transaction, hold), err)]
pub async fn insert_tally_hold(
    hasura_transaction: &Transaction<'_>,
    hold: &TallyHoldRow,
) -> Result<()> {
    hasura_transaction
        .execute(
            "INSERT INTO sequent_backend.signing_tally_hold
                 (id, tenant_id, election_event_id, results_event_id, action, report_type,
                  election_id, area_id, file_name, base_document_id, base_sha256, requested_by,
                  requested_by_username)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            &[
                &hold.id,
                &hold.tenant_id,
                &hold.election_event_id,
                &hold.results_event_id,
                &hold.action,
                &hold.report_type,
                &hold.election_id,
                &hold.area_id,
                &hold.file_name,
                &hold.base_document_id,
                &hold.base_sha256,
                &hold.requested_by,
                &hold.requested_by_username,
            ],
        )
        .await
        .context("Error recording the held tally report")?;
    Ok(())
}

/// The held tally reports of an event whose request hasn't started, oldest
/// first, locked for this transaction (rows another one holds are skipped).
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_pending_tally_holds(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<TallyHoldRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.signing_tally_hold
             WHERE tenant_id = $1 AND election_event_id = $2
                 AND request_id IS NULL AND error IS NULL
             ORDER BY created_at, id
             FOR UPDATE SKIP LOCKED",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the held tally reports")?
        .into_iter()
        .map(TallyHoldRow::try_from)
        .collect()
}

/// Whether a newer tally already held a report of this Post, country and
/// type. A delayed retry must not revive the earlier tally's report.
#[instrument(skip(hasura_transaction, hold), err)]
pub async fn tally_hold_was_superseded(
    hasura_transaction: &Transaction<'_>,
    hold: &TallyHoldRow,
) -> Result<bool> {
    Ok(hasura_transaction
        .query_one(
            "SELECT EXISTS (
                 SELECT 1 FROM sequent_backend.signing_tally_hold newer
                 JOIN sequent_backend.signing_tally_hold current ON current.id = $1
                 WHERE newer.tenant_id = current.tenant_id
                     AND newer.election_event_id = current.election_event_id
                     AND newer.action = current.action
                     AND newer.report_type = current.report_type
                     AND newer.election_id = current.election_id
                     AND newer.area_id IS NOT DISTINCT FROM current.area_id
                     AND newer.results_event_id <> current.results_event_id
                     AND (newer.created_at, newer.id) > (current.created_at, current.id)
             )",
            &[&hold.id],
        )
        .await
        .context("Error checking whether a held tally report was superseded")?
        .try_get(0)?)
}

/// The events with held tally reports whose request hasn't started.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_events_with_pending_tally_holds(
    hasura_transaction: &Transaction<'_>,
) -> Result<Vec<(Uuid, Uuid)>> {
    hasura_transaction
        .query(
            "SELECT DISTINCT tenant_id, election_event_id FROM sequent_backend.signing_tally_hold
             WHERE request_id IS NULL AND error IS NULL",
            &[],
        )
        .await
        .context("Error listing the held tally reports")?
        .into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect()
}

/// Records how a held tally report's request started (or why it couldn't).
#[instrument(skip(hasura_transaction), err)]
pub async fn finish_tally_hold(
    hasura_transaction: &Transaction<'_>,
    id: Uuid,
    request_id: Option<Uuid>,
    error: Option<&str>,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.signing_tally_hold SET request_id = $2, error = $3
             WHERE id = $1",
            &[&id, &request_id, &error],
        )
        .await
        .context("Error recording the held tally report's request")?;
    Ok(())
}

/// Released reports whose held e-mail no send claimed yet.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_unmailed_releases(
    hasura_transaction: &Transaction<'_>,
) -> Result<Vec<(Uuid, Uuid, Uuid)>> {
    hasura_transaction
        .query(
            "SELECT tenant_id, election_event_id, request_id
             FROM sequent_backend.signing_report_release
             WHERE email IS NOT NULL AND released_at IS NOT NULL AND mail_started_at IS NULL",
            &[],
        )
        .await
        .context("Error listing the held e-mails")?
        .into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?, row.try_get(2)?)))
        .collect()
}

/// Claimed sends that never reported back after `seconds`: marked failed,
/// never sent again. Their (tenant, event, request).
#[instrument(skip(hasura_transaction), err)]
pub async fn fail_stale_mail_claims(
    hasura_transaction: &Transaction<'_>,
    seconds: i64,
) -> Result<Vec<(Uuid, Uuid, Uuid)>> {
    hasura_transaction
        .query(
            "UPDATE sequent_backend.signing_report_release
             SET mail_error = 'the send never reported back'
             WHERE mail_started_at IS NOT NULL AND mailed_at IS NULL AND mail_error IS NULL
                 AND mail_started_at <= clock_timestamp() - make_interval(secs => $1::bigint)
             RETURNING tenant_id, election_event_id, request_id",
            &[&seconds],
        )
        .await
        .context("Error failing stale e-mail claims")?
        .into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?, row.try_get(2)?)))
        .collect()
}
