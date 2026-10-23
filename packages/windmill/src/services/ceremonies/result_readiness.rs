// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Readiness belongs to the execution that produced a report, not today's tally head.
use anyhow::{bail, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::{Document, TallySession, TallySessionExecution};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;

pub const POLICY: &str = "result_readiness_version";
pub const ORIGIN: &str = "tally_result_origin";

pub fn uses_readiness_policy(annotations: Option<&Value>) -> bool {
    annotations.and_then(|value| value.get(POLICY)).is_some()
}

pub fn is_numerically_complete(session: &TallySession) -> bool {
    session.is_execution_completed
        && (session.execution_status.as_deref() == Some("SUCCESS")
            || (uses_readiness_policy(session.annotations.as_ref())
                && session.execution_status.as_deref() == Some("IN_PROGRESS")))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResultOrigin {
    pub version: u8,
    pub tally_session_id: String,
    pub execution_id: String,
    pub results_event_id: String,
    pub election_id: Option<String>,
    pub complete: bool,
}

// Only result-document relations are inspected. Intermediate SQLite snapshots
// remain available to the UI; generic documents and legacy renders are untouched.
const REFERENCES: &str = r#"
SELECT DISTINCT d.value AS document_id, r.results_event_id, r.election_id
FROM (
 SELECT tenant_id, election_event_id, id AS results_event_id, NULL::uuid AS election_id, documents FROM sequent_backend.results_event
 UNION ALL SELECT tenant_id, election_event_id, results_event_id, election_id, documents FROM sequent_backend.results_election
 UNION ALL SELECT tenant_id, election_event_id, results_event_id, election_id, documents FROM sequent_backend.results_election_area
 UNION ALL SELECT tenant_id, election_event_id, results_event_id, election_id, documents FROM sequent_backend.results_contest
 UNION ALL SELECT tenant_id, election_event_id, results_event_id, election_id, documents FROM sequent_backend.results_area_contest
 UNION ALL SELECT tenant_id, election_event_id, results_event_id, NULL::uuid, jsonb_build_object('xlsx', documents->'xlsx') FROM sequent_backend.tally_session_execution
) r CROSS JOIN LATERAL jsonb_each_text(COALESCE(r.documents, '{}'::jsonb)) d
WHERE r.tenant_id = $1 AND r.election_event_id = $2
 AND ($3::uuid IS NULL OR r.results_event_id = $3)
 AND ($4::text IS NULL OR d.value = $4) AND d.value IS NOT NULL
"#;

fn scope_complete(status: &Value, election_ids: &[String], election_id: Option<&str>) -> bool {
    let Some(progress) = status.get("elections_status").and_then(Value::as_array) else {
        return false;
    };
    if election_ids.is_empty()
        || election_id.is_some_and(|id| !election_ids.iter().any(|e| e == id))
    {
        return false;
    }
    election_ids
        .iter()
        .filter(|id| election_id.is_none_or(|scope| scope == id.as_str()))
        .all(|id| {
            progress.iter().any(|row| {
                row.get("election_id").and_then(Value::as_str) == Some(id)
                    && row.get("status").and_then(Value::as_str) == Some("SUCCESS")
                    && row.get("progress").and_then(Value::as_f64) == Some(100.0)
            })
        })
}

type AreaContest = (String, String, String);

fn rows_complete(
    election_id: &str,
    expected: &[AreaContest],
    areas: &HashSet<AreaContest>,
    contests: &HashSet<(String, String)>,
    elections: &HashSet<String>,
) -> bool {
    let pairs: Vec<_> = expected
        .iter()
        .filter(|pair| pair.0 == election_id)
        .collect();
    pairs.is_empty()
        || (elections.contains(election_id)
            && pairs.into_iter().all(|pair| {
                areas.contains(pair) && contests.contains(&(pair.0.clone(), pair.1.clone()))
            }))
}

/// Called in the same transaction as the execution/results insert. Expected
/// pairs come from that run's mapped area-contests, including zero-vote areas.
/// Progress must also be complete, so receiving a subset of weight batches can
/// never certify its otherwise-present rows as final.
pub async fn record_result_origins(
    tx: &Transaction<'_>,
    execution: &TallySessionExecution,
    election_ids: &[String],
    expected: &[AreaContest],
    require_all: bool,
) -> Result<()> {
    let Some(result_id) = execution.results_event_id.as_deref() else {
        if require_all && !expected.is_empty() {
            bail!("Completed tally is missing its expected results");
        }
        return Ok(());
    };
    let tenant = parse_uuid_v4(&execution.tenant_id)?;
    let event = parse_uuid_v4(&execution.election_event_id)?;
    let result = parse_uuid_v4(result_id)?;
    let status = execution
        .status
        .as_ref()
        .context("Missing execution progress")?;
    let area_rows = tx.query("SELECT election_id, contest_id, area_id FROM sequent_backend.results_area_contest WHERE tenant_id=$1 AND election_event_id=$2 AND results_event_id=$3", &[&tenant, &event, &result]).await?;
    let areas: HashSet<_> = area_rows
        .iter()
        .map(|r| {
            (
                r.get::<_, Uuid>(0).to_string(),
                r.get::<_, Uuid>(1).to_string(),
                r.get::<_, Uuid>(2).to_string(),
            )
        })
        .collect();
    let contest_rows = tx.query("SELECT election_id, contest_id, annotations FROM sequent_backend.results_contest WHERE tenant_id=$1 AND election_event_id=$2 AND results_event_id=$3", &[&tenant, &event, &result]).await?;
    let contests: HashSet<_> = contest_rows
        .iter()
        .map(|r| {
            (
                r.get::<_, Uuid>(0).to_string(),
                r.get::<_, Uuid>(1).to_string(),
            )
        })
        .collect();
    let election_rows = tx.query("SELECT election_id FROM sequent_backend.results_election WHERE tenant_id=$1 AND election_event_id=$2 AND results_event_id=$3", &[&tenant, &event, &result]).await?;
    let blocked: HashSet<String> = contest_rows
        .iter()
        .filter(|row| {
            row.get::<_, Option<Value>>(2)
                .as_ref()
                .and_then(|a| a.get("process_results"))
                .and_then(|p| p.get("pending_tie_resolution"))
                .is_some_and(|tie| !tie.is_null())
        })
        .map(|row| row.get::<_, Uuid>(0).to_string())
        .collect();
    let elections: HashSet<_> = election_rows
        .iter()
        .map(|r| r.get::<_, Uuid>(0).to_string())
        .collect();
    let complete: HashSet<_> = election_ids
        .iter()
        .filter(|id| {
            !blocked.contains(id.as_str())
                && scope_complete(status, election_ids, Some(id))
                && rows_complete(id, expected, &areas, &contests, &elections)
        })
        .cloned()
        .collect();
    if require_all && complete.len() != election_ids.len() {
        bail!("Completed tally is missing expected election/area-contest results");
    }
    for row in tx
        .query(
            REFERENCES,
            &[&tenant, &event, &Some(result), &None::<String>],
        )
        .await?
    {
        let election_id = row.get::<_, Option<Uuid>>(2).map(|id| id.to_string());
        let origin = ResultOrigin {
            version: 1,
            tally_session_id: execution.tally_session_id.clone(),
            execution_id: execution.id.clone(),
            results_event_id: result_id.to_string(),
            complete: election_id
                .as_ref()
                .map(|id| complete.contains(id))
                .unwrap_or(complete.len() == election_ids.len()),
            election_id,
        };
        // A later execution sharing a result ID must not promote an older
        // document whose originating snapshot was incomplete.
        let document_id: String = row.get(0);
        tx.execute("UPDATE sequent_backend.document SET annotations=COALESCE(annotations,'{}'::jsonb) || $3 WHERE tenant_id=$1 AND id=$2 AND NOT (COALESCE(annotations,'{}'::jsonb) ? 'tally_result_origin')", &[&tenant, &parse_uuid_v4(&document_id)?, &json!({(ORIGIN): origin})]).await?;
    }
    Ok(())
}

pub async fn write_origin(
    tx: &Transaction<'_>,
    tenant_id: &str,
    document_id: &str,
    origin: &ResultOrigin,
) -> Result<()> {
    let updated = tx.execute("UPDATE sequent_backend.document SET annotations=COALESCE(annotations,'{}'::jsonb) || $3 WHERE tenant_id=$1 AND id=$2", &[&parse_uuid_v4(tenant_id)?, &parse_uuid_v4(document_id)?, &json!({(ORIGIN): origin})]).await?;
    if updated != 1 {
        bail!("Missing result document while recording its origin");
    }
    Ok(())
}

/// Return a verified origin for new-policy reports, and None only for legacy or
/// unrelated documents. A new tally's missing/malformed provenance fails closed.
pub async fn require_complete_result_document(
    tx: &Transaction<'_>,
    tenant_id: &str,
    document: &Document,
    requested_tally_id: Option<&str>,
) -> Result<Option<ResultOrigin>> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let origin_value = document.annotations.as_ref().and_then(|a| a.get(ORIGIN));
    if let Some(value) = origin_value {
        let origin: ResultOrigin =
            serde_json::from_value(value.clone()).context("Invalid result document origin")?;
        if origin.version != 1
            || !origin.complete
            || requested_tally_id.is_some_and(|id| id != origin.tally_session_id)
        {
            bail!("Result document does not belong to a complete requested tally scope");
        }
        let row = tx.query_opt("SELECT e.status, e.documents, s.election_ids, s.annotations FROM sequent_backend.tally_session_execution e JOIN sequent_backend.tally_session s ON s.id=e.tally_session_id AND s.tenant_id=e.tenant_id AND s.election_event_id=e.election_event_id WHERE e.tenant_id=$1 AND e.election_event_id=$2 AND e.id=$3 AND e.tally_session_id=$4 AND e.results_event_id=$5", &[&tenant, &parse_uuid_v4(document.election_event_id.as_deref().context("Result document has no event")?)?, &parse_uuid_v4(&origin.execution_id)?, &parse_uuid_v4(&origin.tally_session_id)?, &parse_uuid_v4(&origin.results_event_id)?]).await?.context("Result document origin no longer exists")?;
        let annotations: Option<Value> = row.get(3);
        let ids: Option<Vec<Uuid>> = row.get(2);
        let ids: Vec<String> = ids
            .unwrap_or_default()
            .into_iter()
            .map(|id| id.to_string())
            .collect();
        let status: Option<Value> = row.get(0);
        let documents: Option<Value> = row.get(1);
        if !uses_readiness_policy(annotations.as_ref())
            || !scope_complete(
                status.as_ref().unwrap_or(&Value::Null),
                &ids,
                origin.election_id.as_deref(),
            )
            || documents
                .as_ref()
                .and_then(|d| d.get("sqlite"))
                .and_then(Value::as_str)
                .is_none()
        {
            bail!("Result document's originating execution is incomplete");
        }
        return Ok(Some(origin));
    }
    if let Some(id) = requested_tally_id {
        if tx.query_opt("SELECT 1 FROM sequent_backend.tally_session WHERE tenant_id=$1 AND id=$2 AND annotations ? 'result_readiness_version'", &[&tenant, &parse_uuid_v4(id)?]).await?.is_some() {
            bail!("Document is not bound to the requested new tally");
        }
    }
    // Check ownership independently of document annotations. This prevents a
    // missing new-policy origin from being mistaken for a legacy document.
    if let Some(event_id) = document.election_event_id.as_deref() {
        let event = parse_uuid_v4(event_id)?;
        if tx.query_opt("SELECT 1 FROM sequent_backend.tally_session WHERE tenant_id=$1 AND election_event_id=$2 AND annotations ? 'result_readiness_version' LIMIT 1", &[&tenant, &event]).await?.is_none() {
            return Ok(None);
        }
        for row in tx
            .query(
                REFERENCES,
                &[&tenant, &event, &None::<Uuid>, &Some(document.id.clone())],
            )
            .await?
        {
            let result: Uuid = row.get(1);
            if tx.query_opt("SELECT 1 FROM sequent_backend.tally_session_execution e JOIN sequent_backend.tally_session s ON s.id=e.tally_session_id AND s.tenant_id=e.tenant_id WHERE e.tenant_id=$1 AND e.election_event_id=$2 AND e.results_event_id=$3 AND s.annotations ? 'result_readiness_version' LIMIT 1", &[&tenant, &event, &result]).await?.is_some() {
                bail!("New tally result document is missing provenance");
            }
        }
        // Detached generated PDFs retain a separate durable task binding.
        if tx.query_opt("SELECT 1 FROM sequent_backend.tasks_execution WHERE tenant_id=$1 AND election_event_id=$2 AND annotations ? 'result_readiness_version' AND annotations->>'expected_document_id'=$3 LIMIT 1", &[&tenant, &event, &document.id]).await?.is_some() {
            bail!("New tally PDF is missing provenance");
        }
    }
    Ok(None)
}

/// Event-wide exports must validate the exact execution selected by their worker.
/// Legacy sessions retain their existing export behavior.
pub async fn require_complete_execution_report(
    tx: &Transaction<'_>,
    tenant_id: &str,
    event_id: &str,
    execution_id: &str,
    results_id: &str,
) -> Result<Option<ResultOrigin>> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let event = parse_uuid_v4(event_id)?;
    let result = parse_uuid_v4(results_id)?;
    let row = tx.query_opt("SELECT s.annotations FROM sequent_backend.tally_session_execution e JOIN sequent_backend.tally_session s ON s.id=e.tally_session_id AND s.tenant_id=e.tenant_id WHERE e.tenant_id=$1 AND e.election_event_id=$2 AND e.id=$3 AND e.results_event_id=$4", &[&tenant, &event, &parse_uuid_v4(execution_id)?, &result]).await?.context("Export execution/result identity mismatch")?;
    let annotations: Option<Value> = row.get(0);
    if !uses_readiness_policy(annotations.as_ref()) {
        return Ok(None);
    }
    let row = tx.query_opt("SELECT to_jsonb(d) FROM sequent_backend.results_event r JOIN sequent_backend.document d ON d.id=(r.documents->>'tar_gz')::uuid AND d.tenant_id=r.tenant_id AND d.election_event_id=r.election_event_id WHERE r.tenant_id=$1 AND r.election_event_id=$2 AND r.id=$3", &[&tenant, &event, &result]).await?.context("New tally export is missing its result artifact")?;
    let document: Document = serde_json::from_value(row.get(0))?;
    let origin = require_complete_result_document(tx, tenant_id, &document, None)
        .await?
        .context("New tally export is missing provenance")?;
    if origin.results_event_id != results_id || origin.election_id.is_some() {
        bail!("Export requires the complete event scope");
    }
    Ok(Some(origin))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scope_requires_all_its_elections_and_weight_batches() {
        let ids = vec!["a".into(), "b".into()];
        let status = json!({"elections_status":[{"election_id":"a","status":"SUCCESS","progress":100},{"election_id":"b","status":"DECRYPTING","progress":99.9}]});
        assert!(scope_complete(&status, &ids, Some("a")));
        assert!(!scope_complete(&status, &ids, None));
        assert!(!scope_complete(&status, &ids, Some("other")));
        assert!(!scope_complete(&Value::Null, &ids, Some("a")));
    }
    #[test]
    fn missing_rows_block_zero_vote_and_inherited_pairs_but_no_area_elections_complete() {
        let pairs = vec![
            ("a".into(), "c".into(), "parent".into()),
            ("a".into(), "c".into(), "child".into()),
        ];
        let mut areas = HashSet::from([pairs[0].clone()]);
        let contests = HashSet::from([("a".into(), "c".into())]);
        let elections = HashSet::from(["a".into()]);
        assert!(!rows_complete("a", &pairs, &areas, &contests, &elections));
        areas.insert(pairs[1].clone());
        assert!(rows_complete("a", &pairs, &areas, &contests, &elections));
        assert!(!rows_complete(
            "a",
            &pairs,
            &areas,
            &HashSet::new(),
            &elections
        ));
        assert!(rows_complete(
            "empty", &pairs, &areas, &contests, &elections
        ));
    }
    #[test]
    fn numeric_completion_blocks_cancellation_and_allows_recount_during_reports() {
        let mut session: TallySession = serde_json::from_value(json!({
            "id":"tally", "tenant_id":"tenant", "election_event_id":"event",
            "keys_ceremony_id":"keys", "threshold":1, "is_execution_completed":true,
            "execution_status":"IN_PROGRESS", "annotations":{(POLICY):1}
        }))
        .unwrap();
        assert!(is_numerically_complete(&session));
        session.annotations = None;
        assert!(!is_numerically_complete(&session));
        session.execution_status = Some("SUCCESS".into());
        assert!(is_numerically_complete(&session));
        session.is_execution_completed = false;
        assert!(!is_numerically_complete(&session));
    }
    #[test]
    fn only_explicit_creation_marker_enables_policy() {
        assert!(!uses_readiness_policy(None));
        assert!(!uses_readiness_policy(Some(
            &json!({"is_post_task_completed":false})
        )));
        assert!(uses_readiness_policy(Some(&json!({(POLICY):1}))));
        assert!(uses_readiness_policy(Some(&json!({(POLICY):null}))));
    }
}

#[cfg(test)]
mod postgres_tests {
    use super::*;
    /// Uses an empty disposable database; schema creation fails if it is not
    /// empty. All fixtures, including the schema, are rolled back.
    #[tokio::test]
    #[ignore = "requires RESULT_READINESS_TEST_DATABASE_URL pointing to an empty disposable PostgreSQL database"]
    async fn result_document_origins_enforce_scope_and_preserve_legacy() -> Result<()> {
        let config = std::env::var("RESULT_READINESS_TEST_DATABASE_URL")?.parse()?;
        let manager =
            deadpool_postgres::Manager::new(config, deadpool_postgres::tokio_postgres::NoTls);
        let pool = deadpool_postgres::Pool::builder(manager)
            .max_size(1)
            .build()?;
        let mut client = pool.get().await?;
        let tx = client.transaction().await?;
        tx.batch_execute(r#"
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.document (id uuid, tenant_id uuid, election_event_id uuid, annotations jsonb);
CREATE TABLE sequent_backend.tally_session (id uuid, tenant_id uuid, election_event_id uuid, annotations jsonb, election_ids uuid[]);
CREATE TABLE sequent_backend.tally_session_execution (id uuid, tenant_id uuid, election_event_id uuid, tally_session_id uuid, results_event_id uuid, status jsonb, documents jsonb);
CREATE TABLE sequent_backend.results_event (id uuid, tenant_id uuid, election_event_id uuid, documents jsonb);
CREATE TABLE sequent_backend.results_election (tenant_id uuid, election_event_id uuid, results_event_id uuid, election_id uuid, documents jsonb);
CREATE TABLE sequent_backend.results_election_area (LIKE sequent_backend.results_election);
CREATE TABLE sequent_backend.results_contest (LIKE sequent_backend.results_election, contest_id uuid, annotations jsonb);
CREATE TABLE sequent_backend.results_area_contest (LIKE sequent_backend.results_election, contest_id uuid, area_id uuid);
CREATE TABLE sequent_backend.tasks_execution (tenant_id uuid, election_event_id uuid, annotations jsonb);
"#).await?;
        let tenant = Uuid::new_v4();
        let event = Uuid::new_v4();
        let tally = Uuid::new_v4();
        let result = Uuid::new_v4();
        let execution_id = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let contest = Uuid::new_v4();
        let area = Uuid::new_v4();
        let report = Uuid::new_v4();
        let archive = Uuid::new_v4();
        let detached = Uuid::new_v4();
        let generic = Uuid::new_v4();
        let ids = vec![a.to_string(), b.to_string()];
        let status = json!({"elections_status":[{"election_id":a,"status":"SUCCESS","progress":100},{"election_id":b,"status":"DECRYPTING","progress":40}]});
        tx.execute(
            "INSERT INTO sequent_backend.tally_session VALUES ($1,$2,$3,$4,$5)",
            &[&tally, &tenant, &event, &json!({(POLICY):1}), &vec![a, b]],
        )
        .await?;
        tx.execute(
            "INSERT INTO sequent_backend.tally_session_execution VALUES ($1,$2,$3,$4,$5,$6,$7)",
            &[
                &execution_id,
                &tenant,
                &event,
                &tally,
                &result,
                &status,
                &json!({"sqlite":Uuid::new_v4()}),
            ],
        )
        .await?;
        tx.execute(
            "INSERT INTO sequent_backend.results_event VALUES ($1,$2,$3,$4)",
            &[&result, &tenant, &event, &json!({"tar_gz":archive})],
        )
        .await?;
        tx.execute(
            "INSERT INTO sequent_backend.results_election VALUES ($1,$2,$3,$4,$5)",
            &[&tenant, &event, &result, &a, &json!({"html":report})],
        )
        .await?;
        tx.execute(
            "INSERT INTO sequent_backend.results_contest VALUES ($1,$2,$3,$4,NULL,$5,NULL)",
            &[&tenant, &event, &result, &a, &contest],
        )
        .await?;
        tx.execute(
            "INSERT INTO sequent_backend.results_area_contest VALUES ($1,$2,$3,$4,NULL,$5,$6)",
            &[&tenant, &event, &result, &a, &contest, &area],
        )
        .await?;
        for id in [report, archive, detached, generic] {
            tx.execute(
                "INSERT INTO sequent_backend.document VALUES ($1,$2,$3,NULL)",
                &[&id, &tenant, &event],
            )
            .await?;
        }
        let execution: TallySessionExecution = serde_json::from_value(
            json!({"id":execution_id,"tenant_id":tenant,"election_event_id":event,"tally_session_id":tally,"results_event_id":result,"current_message_id":1,"status":status,"documents":{"sqlite":Uuid::new_v4()}}),
        )?;
        let expected = vec![(a.to_string(), contest.to_string(), area.to_string())];
        let mut empty_execution = execution.clone();
        empty_execution.results_event_id = None;
        assert!(
            record_result_origins(&tx, &empty_execution, &ids, &expected, true)
                .await
                .is_err()
        );
        record_result_origins(&tx, &empty_execution, &ids, &[], true).await?;
        record_result_origins(&tx, &execution, &ids, &expected, false).await?;
        async fn document(
            tx: &Transaction<'_>,
            id: Uuid,
            tenant: Uuid,
            event: Uuid,
        ) -> Result<Document> {
            let annotations: Option<Value> = tx
                .query_one(
                    "SELECT annotations FROM sequent_backend.document WHERE id=$1",
                    &[&id],
                )
                .await?
                .get(0);
            Ok(serde_json::from_value(
                json!({"id":id,"tenant_id":tenant,"election_event_id":event,"annotations":annotations}),
            )?)
        }
        assert!(require_complete_execution_report(
            &tx,
            &tenant.to_string(),
            &event.to_string(),
            &execution_id.to_string(),
            &result.to_string()
        )
        .await
        .is_err());
        let source = document(&tx, report, tenant, event).await?;
        let origin = require_complete_result_document(
            &tx,
            &tenant.to_string(),
            &source,
            Some(&tally.to_string()),
        )
        .await?
        .context("New report lost origin")?;
        assert!(require_complete_result_document(
            &tx,
            &tenant.to_string(),
            &source,
            Some(&Uuid::new_v4().to_string())
        )
        .await
        .is_err());
        assert!(require_complete_result_document(
            &tx,
            &tenant.to_string(),
            &document(&tx, archive, tenant, event).await?,
            None
        )
        .await
        .is_err());
        assert!(require_complete_result_document(
            &tx,
            &tenant.to_string(),
            &document(&tx, generic, tenant, event).await?,
            None
        )
        .await?
        .is_none());
        tx.execute(
            "UPDATE sequent_backend.document SET annotations=NULL WHERE id=$1",
            &[&report],
        )
        .await?;
        assert!(require_complete_result_document(
            &tx,
            &tenant.to_string(),
            &document(&tx, report, tenant, event).await?,
            None
        )
        .await
        .is_err());
        write_origin(&tx, &tenant.to_string(), &report.to_string(), &origin).await?;
        tx.execute(
            "INSERT INTO sequent_backend.tasks_execution VALUES ($1,$2,$3)",
            &[
                &tenant,
                &event,
                &json!({(POLICY):1,"expected_document_id":detached}),
            ],
        )
        .await?;
        assert!(require_complete_result_document(
            &tx,
            &tenant.to_string(),
            &document(&tx, detached, tenant, event).await?,
            None
        )
        .await
        .is_err());
        write_origin(&tx, &tenant.to_string(), &detached.to_string(), &origin).await?;
        assert!(require_complete_result_document(
            &tx,
            &tenant.to_string(),
            &document(&tx, detached, tenant, event).await?,
            None
        )
        .await?
        .is_some());
        // Advancing progress cannot promote a historical partial archive.
        let mut completed = execution.clone();
        completed.status.as_mut().unwrap()["elections_status"][1]["status"] = json!("SUCCESS");
        completed.status.as_mut().unwrap()["elections_status"][1]["progress"] = json!(100);
        tx.execute(
            "UPDATE sequent_backend.tally_session_execution SET status=$1 WHERE id=$2",
            &[&completed.status, &execution_id],
        )
        .await?;
        record_result_origins(&tx, &completed, &ids, &expected, true).await?;
        assert!(require_complete_result_document(
            &tx,
            &tenant.to_string(),
            &document(&tx, archive, tenant, event).await?,
            None
        )
        .await
        .is_err());
        let mut missing = expected.clone();
        missing.push((
            a.to_string(),
            contest.to_string(),
            Uuid::new_v4().to_string(),
        ));
        assert!(record_result_origins(&tx, &completed, &ids, &missing, true)
            .await
            .is_err());
        // A legacy tally's document remains available, even with partial progress.
        tx.execute(
            "UPDATE sequent_backend.tally_session SET annotations='{}' WHERE id=$1",
            &[&tally],
        )
        .await?;
        tx.execute(
            "UPDATE sequent_backend.document SET annotations=NULL WHERE id=$1",
            &[&report],
        )
        .await?;
        assert!(require_complete_result_document(
            &tx,
            &tenant.to_string(),
            &document(&tx, report, tenant, event).await?,
            None
        )
        .await?
        .is_none());
        assert!(require_complete_execution_report(
            &tx,
            &tenant.to_string(),
            &event.to_string(),
            &execution_id.to_string(),
            &result.to_string()
        )
        .await?
        .is_none());
        tx.rollback().await?;
        Ok(())
    }
}
