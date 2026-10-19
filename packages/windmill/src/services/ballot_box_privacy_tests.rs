// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use electoral_log::adapters::ballot_box::{AcceptReceived, BallotStatus};
use electoral_log::adapters::ballot_box_received::{
    ReceiveBallot, ReceiveOutcome, ReceivedSignature,
};
use electoral_log::adapters::privacy::BallotIdentity;
use sequent_core::ballot_privacy::{
    verify_ballot_log_proof, BallotPrivacyProof, BallotPrivacyTrust, CheckStatus,
};
use sequent_core::ballot_receipt::{
    ballot_box_key, sign_cast_receipt, sign_cast_statement, sign_received_ballot,
    verify_cast_receipt, verify_received_ballot, CastReceiptStatement, ReceivedStatement,
};
use voting_crypto::signing::VerificationKey;
use voting_crypto::{RecordDeserialize, RecordSerialize};

/// The real writer must publish exactly one binding and one cast, including
/// recovery when a cast appears after the received-outbox empty read.
#[tokio::test]
#[ignore = "requires disposable ELECTORAL_LOG_PG_* and ENV_SLUG"]
async fn privacy_sequencer_orders_bindings_and_deduplicates_recovery() -> Result<()> {
    let databases = get_event_databases().await?;
    for simulate_queue_race in [false, true] {
        let tenant = uuid::Uuid::new_v4().to_string();
        let event = uuid::Uuid::new_v4().to_string();
        let election = uuid::Uuid::new_v4().to_string();
        let area = uuid::Uuid::new_v4().to_string();
        let board = crate::services::electoral_log_board::get_event_board(
            &tenant,
            &event,
            &std::env::var("ENV_SLUG")?,
        );
        let store = databases.create_event(&tenant, &event).await?;
        let real_voter = "private-voter-must-never-be-published".to_string();
        store.create_board(&board).await?;
        store.create_ballot_box(&event).await?;
        store
            .set_ballot_identity(&event, BallotIdentity::Pseudonym)
            .await?;
        store
            .allocate_voter_pseudonyms(&event, &[real_voter.clone()])
            .await?;
        let pseudonym = store
            .voter_pseudonym(&event, &real_voter)
            .await?
            .context("mapping")?;
        let authority_key = SigningKey::generate()?;
        let receipt_key = SigningKey::generate()?;
        let voter_key = SigningKey::generate()?;
        let published_key = ballot_box_key(&VerificationKey::from_sk(&receipt_key)?)?;
        let now = Utc::now();
        let timestamp = crate::services::receive_ballot::format_signed_time(&now);
        use sequent_core::ballot::{
            sign_hashable_ballot_with_voter_signing_key, BallotStyle, HashableBallot,
            SignedHashableBallot,
        };
        use sequent_core::fixtures::ballot_codec::{
            get_test_contest, get_test_decoded_vote_contest, get_writein_ballot_style,
        };
        let style = BallotStyle {
            contests: vec![get_test_contest()],
            ..get_writein_ballot_style()
        };
        let ciphertext = sequent_core::encrypt::encrypt_decoded_contest(
            &cryptography::context::RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &style,
        )?;
        let mut signed = SignedHashableBallot::try_from(&ciphertext)?;
        let hashable = HashableBallot::try_from(&signed)?;
        let ballot_hash = sequent_core::encrypt::hash_ballot(&hashable)?;
        let signature = sign_hashable_ballot_with_voter_signing_key(
            &voter_key,
            &ballot_hash,
            &election,
            &hashable,
        )
        .map_err(anyhow::Error::msg)?;
        let voter_pk = signature.public_key;
        let voter_signature = signature.signature;
        let voter_signature_bytes =
            voting_crypto::signing::Signature::from_b64_string(&voter_signature)?.to_bytes();
        signed.voter_signing_pk = Some(voter_pk.clone());
        signed.voter_ballot_signature = Some(voter_signature.clone());
        let encrypted_json = serde_json::to_string(&signed)?;
        let encrypted = encrypted_json.as_str();
        let received_receipt = sign_received_ballot(
            &receipt_key,
            ReceivedStatement {
                tenant_id: tenant.clone(),
                election_event_id: event.clone(),
                election_id: election.clone(),
                ballot_hash: ballot_hash.clone(),
                voter_signing_pk: voter_pk.clone(),
                voter_ballot_signature: voter_signature.clone(),
                received_at: timestamp.clone(),
                key_id: published_key.key_id.clone(),
            },
        )?;
        verify_received_ballot(&published_key, &received_receipt)?;
        let received = match store
            .receive_ballot(
                &ReceiveBallot {
                    election_event_id: &event,
                    election_id: &election,
                    area_id: &area,
                    voter_id: &pseudonym,
                    ballot_hash: &ballot_hash,
                    content: encrypted,
                    voter_signing_pk: &voter_pk,
                    voter_ballot_signature: &voter_signature,
                },
                now,
                |_| {
                    Ok(ReceivedSignature {
                        ballot_id: received_receipt.ballot_id.clone(),
                        key_id: published_key.key_id.clone(),
                        received_signature: received_receipt.received_signature.clone(),
                    })
                },
            )
            .await?
        {
            ReceiveOutcome::Received(ballot) => ballot,
            outcome => anyhow::bail!("Unexpected receive outcome {outcome:?}"),
        };
        let cast_signature = sign_cast_statement(&voter_key, &election, &received.ballot_id)?;
        let cast_receipt = sign_cast_receipt(
            &receipt_key,
            CastReceiptStatement {
                election_event_id: event.clone(),
                election_id: election.clone(),
                ballot_id: received.ballot_id.clone(),
                received_at: timestamp.clone(),
                cast_at: timestamp,
                key_id: published_key.key_id.clone(),
                cast_signature: cast_signature.clone(),
            },
        )?;
        verify_cast_receipt(&published_key, &cast_receipt)?;
        let hash = voting_crypto::hash::hash_to_array(pseudonym.as_bytes())?;
        let encrypted_hash = sequent_core::encrypt::hash_ballot_sha512(&hashable)?;
        let outcome = store
            .accept_received_ballot(
                &AcceptBallot {
                    election_event_id: &event,
                    election_id: &election,
                    area_id: &area,
                    voter_id: &pseudonym,
                    ballot_id: &ballot_hash,
                    format: "hashable-ballot",
                    content: encrypted,
                    voter_signature: Some(&voter_signature_bytes),
                    pseudonym_hash: &hash,
                    ballot_hash: &encrypted_hash,
                    voting_channel: "ONLINE",
                    status: BallotStatus::Valid,
                    voter_ip: None,
                    voter_country: None,
                    username: None,
                    allowed_votes: 2,
                },
                &AcceptReceived {
                    ballot_id: &received.ballot_id,
                    cast_at: now,
                    cast_signature: &cast_signature,
                    cast_receipt_signature: &cast_receipt.cast_receipt_signature,
                },
            )
            .await?;
        assert!(matches!(outcome, AcceptOutcome::Accepted { .. }));
        let queued = store.pending_received_ballots(&event, 100).await?;
        assert_eq!(queued.len(), 1);
        if simulate_queue_race {
            // Model the writer's queue-empty observation before receive+cast
            // committed: the writer must reconstruct the binding from the cast.
            store
                .remove_pending_received(&event, &[queued[0].id])
                .await?;
        }
        append_pending(&store, &board, &tenant, &event, &authority_key).await?;
        let records = store
            .query(
                &board,
                &electoral_log::LogQuery {
                    order: vec![(
                        electoral_log::OrderColumn::Id,
                        electoral_log::SortDirection::Asc,
                    )],
                    ..Default::default()
                },
            )
            .await?;
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].statement_kind, "BallotReceived");
        // Existing CastVote queries retain their header-kind compatibility;
        // the signed body carries the new privacy evidence format.
        assert_eq!(records[1].statement_kind, "CastVote");
        let cast_message = Message::record_deserialize(&records[1].message)?;
        assert!(matches!(
            cast_message.statement.body,
            electoral_log::messages::statement::StatementBody::PrivacyCastVote(_)
        ));
        for record in &records {
            assert!(record.user_id.is_none());
            assert_eq!(record.username.as_deref(), Some(pseudonym.as_str()));
            let message = Message::record_deserialize(&record.message)?;
            assert!(message.user_id.is_none());
            assert_eq!(message.sender.name, pseudonym);
            assert!(!serde_json::to_string(&message)?.contains(&real_voter));
        }
        if simulate_queue_race {
            // Its delayed outbox delivery uses the same stable delivery ID.
            store.client().await?.execute(
                "INSERT INTO sequent_backend.ballot_evidence_pending (election_event_id, received_id, kind)
                 VALUES ($1::text::uuid, $2::text::uuid, 'received')",
                &[&event, &received.id],
            ).await?;
        }
        append_pending(&store, &board, &tenant, &event, &authority_key).await?;
        assert_eq!(
            store
                .query(&board, &electoral_log::LogQuery::default())
                .await?
                .len(),
            2
        );
        assert_eq!(store.pending_count(&event).await?, 0);
        assert_eq!(
            append_pending(&store, &board, &tenant, &event, &authority_key).await?,
            0
        );
        let audit = store.audit(&board, &[]).await?;
        assert!(audit.is_clean(), "{:?}", audit.findings());
        verify_private_acceptance(
            &store,
            &board,
            &tenant,
            &event,
            &election,
            &area,
            &real_voter,
            &pseudonym,
            &received.ballot_id,
            &authority_key,
            &published_key,
            &records,
        )
        .await?;
        databases.drop_event(&tenant, &event).await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn verify_private_acceptance(
    store: &PostgresStore,
    board: &str,
    tenant: &str,
    event: &str,
    election: &str,
    area: &str,
    real_voter: &str,
    pseudonym: &str,
    ballot_id: &str,
    authority_key: &SigningKey,
    published_key: &sequent_core::ballot::BallotBoxKey,
    records: &[electoral_log::ElectoralLogMessage],
) -> Result<()> {
    use crate::services::electoral_log::{
        count_electoral_log, list_electoral_log, ElectoralLogRow, GetElectoralLogBody, OrderField,
    };
    use electoral_log::messages::message::SigningData;
    use electoral_log::messages::newtypes::{ErrorMessageString, KeycloakEventTypeString};
    use electoral_log::messages::privacy::PseudonymResolution;
    store
        .record_forensic_network(&electoral_log::adapters::privacy::ForensicNetworkRecord {
            election_event_id: event,
            election_id: Some(election),
            area_id: Some(area),
            event_kind: "cast",
            delivery_id: "acceptance:network",
            pseudonym,
            ballot_id: Some(ballot_id),
            ip: "198.51.100.73",
            country: Some("AQ"),
            user_agent: Some("synthetic-private-browser"),
            recorded_at: Utc::now(),
        })
        .await?;
    let private_count: i64 = store.client().await?.query_one("SELECT count(*) FROM sequent_backend.forensic_network_record WHERE election_event_id=$1::text::uuid AND ip=$2 AND user_agent=$3", &[&event, &"198.51.100.73", &"synthetic-private-browser"]).await?.get(0);
    assert_eq!(private_count, 1);
    let trust = BallotPrivacyTrust {
        tenant_id: tenant.into(),
        election_event_id: event.into(),
        election_id: election.into(),
        system_public_key: VerificationKey::from_sk(authority_key)?.to_b64_string()?,
        ballot_box_key: published_key.clone(),
    };
    let proof = BallotPrivacyProof {
        ballot_id: ballot_id.into(),
        pseudonym: pseudonym.into(),
        voting_channel: "ONLINE".into(),
        received_entry: Some(STANDARD.encode(&records[0].message)),
        cast_entry: Some(STANDARD.encode(&records[1].message)),
    };
    let check = || {
        let report = verify_ballot_log_proof(&trust, &proof);
        assert_eq!(report.checks.binding, CheckStatus::Passed);
        assert_eq!(report.checks.received, CheckStatus::Passed);
        assert_eq!(report.checks.ballot_id, CheckStatus::Passed);
        assert_eq!(report.checks.cast, CheckStatus::Passed);
    };
    check();
    // The browser proof and the journal inclusion proof both use real sequenced bytes.
    let checkpoint = store.journal().checkpoint(board).await?;
    let inclusion = store
        .record_proof(
            &store.journal(),
            electoral_log::adapters::postgres::LogScope::Board(board),
            records[1].id,
            Some(&checkpoint),
        )
        .await?;
    inclusion.verify(&checkpoint)?;
    let sd = SigningData::new(authority_key.clone(), "system", authority_key.clone());
    let login = Message::keycloak_user_event(
        EventIdString(event.into()),
        KeycloakEventTypeString("LOGIN".into()),
        ErrorMessageString(String::new()),
        Some(real_voter.into()),
        Some("synthetic-account-name".into()),
        &sd,
        None,
    )?;
    let communication = Message::send_template(
        EventIdString(event.into()),
        ElectionIdString(None),
        &sd,
        Some(real_voter.into()),
        Some("synthetic-account-name".into()),
        Some("Message delivered".into()),
        None,
    )?;
    for (id, message) in [("login", login), ("communication", communication)] {
        store
            .append(
                board,
                &mut std::iter::once(Ok(LogEntry {
                    delivery_id: format!("acceptance:{id}"),
                    message: (&message).try_into()?,
                })),
            )
            .await?;
    }
    let input = GetElectoralLogBody {
        tenant_id: tenant.into(),
        election_event_id: event.into(),
        filter: Some(HashMap::from([(OrderField::UserId, real_voter.into())])),
        ..Default::default()
    };
    let trace = PrivacyTrace::default();
    let writer = trace.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_writer(move || writer.clone())
        .finish();
    use tracing::instrument::WithSubscriber;
    let user_page = list_electoral_log(input.clone())
        .with_subscriber(subscriber)
        .await?;
    let logs = String::from_utf8(trace.0.lock().unwrap().clone())?;
    assert!(
        !logs.contains(real_voter),
        "real voter query leaked to worker tracing"
    );
    assert_eq!(count_electoral_log(input).await?, 2);
    assert_eq!(user_page.total.aggregate.count, 2);
    let mut kinds = user_page
        .items
        .iter()
        .map(|row| row.statement_kind.as_str())
        .collect::<Vec<_>>();
    kinds.sort();
    assert_eq!(kinds, ["KeycloakUserEvent", "SendCommunications"]);
    let user_csv = crate::services::reports::activity_log::generate_export_data(
        &user_page.items,
        "privacy-user-log",
    )
    .await?;
    let exported_users = crate::services::electoral_log::exported_log_reader(user_csv.reopen()?)?
        .deserialize::<ElectoralLogRow>()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    assert_eq!(exported_users.len(), 2);
    assert!(exported_users
        .iter()
        .all(|row| row.user_id.as_deref() == Some(real_voter)));
    assert!(exported_users
        .iter()
        .all(|row| !row.message.contains(pseudonym) && !row.message.contains(ballot_id)));

    // Resolve only through the private mapping; the durable signed audit contains
    // the investigation, actor and pseudonym, never the returned voter identity.
    assert_eq!(
        store.resolve_pseudonym(event, pseudonym).await?.as_deref(),
        Some(real_voter)
    );
    let audit = Message::pseudonym_resolved_message(
        event,
        PseudonymResolution {
            tenant_id: tenant.into(),
            pseudonym: pseudonym.into(),
            actor_id: "synthetic-investigator".into(),
            reason: "Synthetic acceptance investigation".into(),
        },
        &sd,
    )?;
    store
        .queue_privacy_message(event, "acceptance:resolution", &audit.record_serialize()?)
        .await?;
    append_pending(store, board, tenant, event, authority_key).await?;
    let all = store
        .query(board, &electoral_log::LogQuery::default())
        .await?;
    let resolution = all
        .iter()
        .find(|row| row.statement_kind == "PseudonymResolved")
        .context("signed resolution audit")?;
    Message::record_deserialize(&resolution.message)?
        .verify(&VerificationKey::from_sk(authority_key)?)?;
    assert!(!String::from_utf8_lossy(&resolution.message).contains(real_voter));
    assert!(
        String::from_utf8_lossy(&resolution.message).contains("Synthetic acceptance investigation")
    );

    // Real JSON/CSV exports, canonical signed bytes and public ballot details
    // are scanned by the same offline artifact auditor used by operators.
    let ballot_rows = records
        .iter()
        .cloned()
        .map(ElectoralLogRow::try_from)
        .collect::<Result<Vec<_>>>()?;
    let csv = crate::services::reports::activity_log::generate_export_data(
        &ballot_rows,
        "privacy-ballot-log",
    )
    .await?;
    let artifacts = tempfile::tempdir()?;
    let json_path = artifacts.path().join("ballot-log.json");
    let proof_path = artifacts.path().join("ballot-proof.json");
    let detail_path = artifacts.path().join("ballot-store.json");
    let csv_path = artifacts.path().join("ballot-log.csv");
    let forbidden_path = artifacts.path().join("forbidden.json");
    let pdf_path = artifacts.path().join("ballot-log.pdf");
    let pdf_text_path = artifacts.path().join("ballot-log-pdf.txt");
    use crate::services::reports::activity_log::{ActivityLogRow, LogZones};
    let zones = LogZones::new(None, None, &[]);
    let pdf_rows = records
        .iter()
        .cloned()
        .map(|record| ActivityLogRow::new(record, &zones))
        .collect::<Result<Vec<_>>>()?;
    let context = serde_json::json!({"rows":serde_json::to_string(&pdf_rows)?});
    let html = handlebars::Handlebars::new().render_template(r#"<html><body><h1>Synthetic ballot evidence</h1><pre style="white-space:pre-wrap;overflow-wrap:anywhere">{{ rows }}</pre></body></html>"#, &context)?;
    let pdf = sequent_core::services::pdf::html_to_pdf_with_policy(
        html,
        None,
        sequent_core::services::pdf::PdfResourcePolicy::RestrictedNoScripts,
    )?;
    assert!(pdf.starts_with(b"%PDF"));
    std::fs::write(&pdf_path, &pdf)?;
    let pdf_document = lopdf::Document::load_mem(&pdf)?;
    let pdf_text =
        pdf_document.extract_text(&pdf_document.get_pages().keys().copied().collect::<Vec<_>>())?;
    assert!(pdf_text.contains("Synthetic ballot evidence"));
    assert!(pdf_text.contains("BallotReceived"));
    std::fs::write(&pdf_text_path, pdf_text)?;
    std::fs::write(&json_path, serde_json::to_vec(&ballot_rows)?)?;
    std::fs::write(&proof_path, serde_json::to_vec(&proof)?)?;
    let details = store
        .privacy_ballot_details(event, area, Some(election), ballot_id)
        .await?
        .context("public ballot details")?;
    std::fs::write(
        &detail_path,
        serde_json::to_vec(
            &serde_json::json!({"ballot_id":details.ballot_id,"pseudonym":details.voter_id,"election_id":details.election_id,"cast_at":details.cast_at,"content":details.content,"voter_signing_pk":details.voter_signing_pk,"voter_ballot_signature":details.voter_ballot_signature}),
        )?,
    )?;
    std::fs::copy(csv.path(), &csv_path)?;
    std::fs::write(
        &forbidden_path,
        serde_json::to_vec(
            &serde_json::json!({"voter_id":[real_voter],"username":["synthetic-account-name"],"ip":["198.51.100.73"],"device":["synthetic-private-browser"],"country":["AQ"]}),
        )?,
    )?;
    let scanner = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/vote_privacy_audit.py");
    let scan = std::process::Command::new("python3")
        .arg(scanner)
        .arg("--forbidden")
        .arg(&forbidden_path)
        .args([
            &json_path,
            &proof_path,
            &detail_path,
            &csv_path,
            &pdf_text_path,
        ])
        .output()?;
    assert!(
        scan.status.success(),
        "artifact scan failed: {} {}",
        String::from_utf8_lossy(&scan.stdout),
        String::from_utf8_lossy(&scan.stderr)
    );
    // Positive control: prove that the same scanner rejects an actual forbidden
    // field instead of passing because the selected files were empty.
    std::fs::write(
        &detail_path,
        serde_json::to_vec(&serde_json::json!({"voter_id":real_voter}))?,
    )?;
    let rejected = std::process::Command::new("python3")
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../scripts/vote_privacy_audit.py"),
        )
        .arg("--forbidden")
        .arg(&forbidden_path)
        .arg(&detail_path)
        .output()?;
    assert_eq!(rejected.status.code(), Some(1));
    assert_eq!(store.purge_voter_pseudonyms(event).await?, 1);
    assert!(store.resolve_pseudonym(event, pseudonym).await?.is_none());
    assert!(store
        .allocate_voter_pseudonyms(event, &[real_voter.into()])
        .await
        .is_err());
    check();
    inclusion.verify(&checkpoint)?;
    let after = store
        .query(board, &electoral_log::LogQuery::default())
        .await?;
    assert_eq!(after.len(), all.len());
    for before in &all {
        assert_eq!(
            after
                .iter()
                .find(|row| row.id == before.id)
                .unwrap()
                .message,
            before.message
        );
    }
    Ok(())
}

#[derive(Clone, Default)]
struct PrivacyTrace(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for PrivacyTrace {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn privacy_worker_read_filters_never_enter_trace_spans() {
    use crate::services::electoral_log::{
        get_cols_match_count_and_select, list_cast_vote_messages, GetElectoralLogBody, OrderField,
    };
    use tracing::instrument::WithSubscriber;
    let trace = PrivacyTrace::default();
    let writer = trace.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .without_time()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_writer(move || writer.clone())
        .finish();
    async {
        tracing::error!("privacy-worker-positive-control");
        get_cols_match_count_and_select("event", "private-worker-voter", "private-worker-ballot");
        let input = GetElectoralLogBody {
            filter: Some(HashMap::from([(
                OrderField::UserId,
                "private-worker-voter".into(),
            )])),
            ..Default::default()
        };
        // Odd-length input is refused before any database access, but the
        // instrumented entry span still captures arguments unless explicitly skipped.
        assert!(list_cast_vote_messages(
            input,
            "private-worker-ballot",
            "private-worker-voter",
            "private-worker-name"
        )
        .await
        .is_err());
    }
    .with_subscriber(subscriber)
    .await;
    let logs = String::from_utf8(trace.0.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("privacy-worker-positive-control"));
    for value in [
        "private-worker-voter",
        "private-worker-ballot",
        "private-worker-name",
    ] {
        assert!(
            !logs.contains(value),
            "worker trace disclosed a private filter"
        );
    }
}
