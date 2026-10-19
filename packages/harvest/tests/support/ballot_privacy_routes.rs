// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::route_services::{post, rows, Services};
use crate::test_claims::Claims;

const TENANT: &str = "00000000-0000-4000-8000-000000000001";
const EVENT: &str = "00000000-0000-4000-8000-000000000002";
const ELECTION: &str = "00000000-0000-4000-8000-000000000003";
const AREA: &str = "00000000-0000-4000-8000-000000000004";
fn voter() -> Claims {
    Claims::new(TENANT, "voter")
        .azp("voting-portal")
        .area(AREA)
        .election_event(EVENT)
        .authorized_elections(&[ELECTION])
        .roles([VoterPermissions::CAST_VOTE])
}
fn input() -> Value {
    json!({"election_event_id":EVENT,"election_id":ELECTION,"ballot_id":"ABCD-EFGH","document_id":"00000000-0000-4000-8000-000000000005","pseudonym":"7QKM-2HXD","reason":"Official investigation","ip":"198.51.100.7"})
}

#[tokio::test]
async fn privacy_routes_refuse_missing_permission_before_any_database() {
    let services = Services::without_database();
    let client = services.client().await;
    let claims = Claims::new(TENANT, "unprivileged");
    for path in [
        "/receipt-document",
        "/ballot-box-log",
        "/ballot-box-own",
        "/ballot-box-entry",
        "/ballot-box-keys",
        "/resolve-voter-pseudonym",
        "/forensic-network-summary",
        "/forensic-network-detail",
        "/ballot-privacy-settings",
    ] {
        let response = post(&client, path, &claims, &input()).await;
        assert!(
            [Status::Unauthorized, Status::Forbidden]
                .contains(&response.status()),
            "{path}: {}",
            response.status()
        );
    }
}

#[tokio::test]
async fn voter_event_and_election_gates_run_before_storage_reads() {
    let services = Services::without_database();
    let client = services.client().await;
    for path in [
        "/ballot-box-log",
        "/ballot-box-own",
        "/ballot-box-entry",
        "/ballot-box-keys",
    ] {
        let wrong_event =
            voter().election_event("00000000-0000-4000-8000-000000000099");
        assert_eq!(
            post(&client, path, &wrong_event, &input()).await.status(),
            Status::Forbidden,
            "{path}"
        );
        let wrong_election =
            voter().authorized_elections(&["another-election"]);
        assert_eq!(
            post(&client, path, &wrong_election, &input())
                .await
                .status(),
            Status::Forbidden,
            "{path}"
        );
        let missing_area = Claims::new(TENANT, "voter")
            .azp("voting-portal")
            .election_event(EVENT)
            .authorized_elections(&[ELECTION])
            .roles([VoterPermissions::CAST_VOTE]);
        assert_eq!(
            post(&client, path, &missing_area, &input()).await.status(),
            Status::Forbidden,
            "{path}"
        );
    }
}

#[tokio::test]
async fn separately_privileged_admin_cannot_read_another_tenants_event() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let client = services.client().await;
    let claims = Claims::new(TENANT, "investigator").roles([
        Permissions::LOGS_READ,
        Permissions::FORENSIC_READ,
        Permissions::VOTER_PSEUDONYM_RESOLVE,
        Permissions::ELECTION_EVENT_READ,
    ]);
    let mut body = input();
    body["election_event_id"] = json!(event.election_event_id);
    for path in [
        "/ballot-box-entry",
        "/ballot-box-keys",
        "/resolve-voter-pseudonym",
        "/forensic-network-summary",
        "/forensic-network-detail",
        "/ballot-privacy-settings",
    ] {
        assert_eq!(
            post(&client, path, &claims, &body).await.status(),
            Status::NotFound,
            "{path}"
        );
    }
}

#[test]
fn viewer_and_investigator_permissions_do_not_imply_each_other() {
    let reader = Claims::new(TENANT, "reader")
        .roles([Permissions::LOGS_READ])
        .build();
    assert!(proof_token_scope(&reader, EVENT, ELECTION).is_ok());
    for permission in [
        Permissions::FORENSIC_READ,
        Permissions::VOTER_PSEUDONYM_RESOLVE,
    ] {
        assert!(authorize(
            &reader,
            false,
            Some(TENANT.into()),
            vec![permission.clone()]
        )
        .is_err());
        let investigator = Claims::new(TENANT, "investigator")
            .roles([permission])
            .build();
        assert!(proof_token_scope(&investigator, EVENT, ELECTION).is_err());
    }
}

#[test]
fn scope_cache_and_cursor_cannot_cross_tenant_area_or_election() {
    let claims = voter().build();
    let (area, elections) =
        voter_token_scope(&claims, EVENT, Some(ELECTION)).unwrap();
    assert_eq!(area, AREA);
    assert_eq!(elections, vec![ELECTION]);
    let key = page_cache_key(TENANT, EVENT, AREA, &elections, 25);
    for other in [
        page_cache_key("other", EVENT, AREA, &elections, 25),
        page_cache_key(TENANT, "other", AREA, &elections, 25),
        page_cache_key(TENANT, EVENT, "other", &elections, 25),
        page_cache_key(TENANT, EVENT, AREA, &["other".into()], 25),
        page_cache_key(TENANT, EVENT, AREA, &elections, 100),
    ] {
        assert_ne!(key, other);
    }
    let scope = scope_key(EVENT, AREA, &elections, "");
    let cursor = encode_cursor(&scope, 100);
    assert!(decode_cursor(
        &scope_key(EVENT, "other", &elections, ""),
        Some(&cursor)
    )
    .is_err());
}

#[tokio::test]
async fn private_receipt_polling_refuses_other_event_and_missing_voter_scope_before_database(
) {
    let services = Services::without_database();
    let client = services.client().await;
    let wrong_event = voter().election_event("other-event");
    assert_eq!(
        post(&client, "/receipt-document", &wrong_event, &input())
            .await
            .status(),
        Status::Forbidden
    );
    let no_area = Claims::new(TENANT, "voter")
        .azp("voting-portal")
        .election_event(EVENT)
        .authorized_elections(&[ELECTION])
        .roles([VoterPermissions::CAST_VOTE]);
    assert_eq!(
        post(&client, "/receipt-document", &no_area, &input())
            .await
            .status(),
        Status::Forbidden
    );
}

#[test]
fn received_only_admin_proof_is_pending_without_exposing_uncast_ballots_to_voters(
) {
    use electoral_log::adapters::ballot_box_received::ReceivedStatus;
    for status in [ReceivedStatus::Received, ReceivedStatus::Audited] {
        assert!(ensure_proof_receipt_visible(false, status).is_ok());
        assert_eq!(proof_channel(status, None).unwrap(), "");
        assert_eq!(
            ensure_proof_receipt_visible(true, status).unwrap_err().0,
            Status::NotFound
        );
    }
    assert!(ensure_proof_receipt_visible(true, ReceivedStatus::Cast).is_ok());
    assert_eq!(
        proof_channel(ReceivedStatus::Cast, Some("online".into())).unwrap(),
        "online"
    );
    assert!(proof_channel(ReceivedStatus::Cast, None).is_err());
}

/// Real private mapping, signed vault identity and durable outbox behind the
/// actual HTTP permission boundary. Every database is a disposable fixture.
#[test]
#[ignore = "requires isolated ELECTORAL_LOG_PG_* and MASTER_SECRET"]
fn authorized_resolution_is_private_durable_and_unavailable_after_purge() {
    rows::run(async {
        use electoral_log::messages::message::Message;
        use electoral_log::ports::ElectoralLogStore;
        use voting_crypto::{
            signing::{SigningKey, VerificationKey},
            RecordDeserialize,
        };
        use windmill::services::electoral_log_board::{
            get_event_board, get_event_databases,
            get_protocol_manager_secret_path,
        };
        let services = Services::on_test_database().await;
        let seed = rows::event(&services.hasura).await;
        let event = rows::Event {
            tenant_id: seed.tenant_id,
            election_event_id: uuid::Uuid::new_v4().to_string(),
        };
        rows::execute(&services.hasura, "INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,presentation) VALUES($1::text::uuid,$2::text::uuid,'RSA256',$3)", &[&event.election_event_id,&event.tenant_id,&json!({"ballot_identity":"pseudonym","network_data_policy":"forensic-store","voter_signing_policy":"with-signature","receipts":{"policy":"signed-by-ballot-box"},"show_cast_vote_logs":"show-logs-tab"})]).await;
        let board = get_event_board(
            &event.tenant_id,
            &event.election_event_id,
            &std::env::var("ENV_SLUG").unwrap(),
        );
        rows::execute(&services.hasura, "UPDATE sequent_backend.election_event SET bulletin_board_reference=$2 WHERE id=$1::text::uuid", &[&event.election_event_id, &json!({"id":1,"database_name":board,"is_archived":false})]).await;
        let key = SigningKey::generate().unwrap();
        let receipt_key = SigningKey::generate().unwrap();
        let election = event.election(&services.hasura).await;
        let area = event.area(&services.hasura, "Synthetic area").await;
        let mut db = services.hasura.get().await.unwrap();
        let tx = db.transaction().await.unwrap();
        windmill::services::vault::save_secret(
            &tx,
            &event.tenant_id,
            Some(&event.election_event_id),
            &get_protocol_manager_secret_path(&board),
            &format!("signing_key = {:?}", key.to_b64_string().unwrap()),
        )
        .await
        .unwrap();
        windmill::services::vault::save_secret(
            &tx,
            &event.tenant_id,
            Some(&event.election_event_id),
            &windmill::services::ballot_box_key::get_ballot_box_secret_path(
                &board,
            ),
            &receipt_key.to_b64_string().unwrap(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        drop(db);
        let databases = get_event_databases().await.unwrap();
        let store = databases
            .create_event(&event.tenant_id, &event.election_event_id)
            .await
            .unwrap();
        store.create_board(&board).await.unwrap();
        store
            .create_ballot_box(&event.election_event_id)
            .await
            .unwrap();
        store
            .set_ballot_identity(
                &event.election_event_id,
                BallotIdentity::Pseudonym,
            )
            .await
            .unwrap();
        let voter = "private-resolved-voter-must-not-be-in-audit";
        store
            .allocate_voter_pseudonyms(
                &event.election_event_id,
                &[voter.into()],
            )
            .await
            .unwrap();
        let code = store
            .voter_pseudonym(&event.election_event_id, voter)
            .await
            .unwrap()
            .unwrap();
        let ballot_id = signed_private_ballot(
            &store,
            &event.tenant_id,
            &event.election_event_id,
            &election,
            &area,
            &board,
            &code,
            &key,
            &receipt_key,
        )
        .await;
        let client = services.client().await;
        let ordinary = Claims::new(&event.tenant_id, voter)
            .azp("voting-portal")
            .area(&area)
            .election_event(&event.election_event_id)
            .authorized_elections(&[&election])
            .roles([VoterPermissions::CAST_VOTE]);
        let scope = json!({"election_event_id":event.election_event_id,"election_id":election,"ballot_id":ballot_id});
        let authorize = client
            .get("/ballot-box-log-authorize")
            .header(crate::route_services::bearer(&ordinary))
            .dispatch()
            .await;
        assert_eq!(authorize.status(), Status::NoContent);
        assert_eq!(
            authorize.headers().get_one("Cache-Control"),
            Some("no-store")
        );
        let expected_scope = crate::services::ballot_log_cache::scope(
            &event.tenant_id,
            &event.election_event_id,
            &area,
            std::slice::from_ref(&election),
        );
        assert_eq!(
            authorize.headers().get_one("X-Ballot-Scope"),
            Some(expected_scope.as_str())
        );
        let mut session = ordinary.build();
        session.sid = Some("synthetic-session-a".into());
        limit_search(&session, &store).await.unwrap();
        let mut refreshed = session.clone();
        refreshed.jti = "synthetic-refreshed-token".into();
        assert_eq!(
            limit_search(&refreshed, &store).await.unwrap_err().0,
            Status::TooManyRequests
        );
        refreshed.sid = Some("synthetic-session-b".into());
        limit_search(&refreshed, &store).await.unwrap();
        let session_key =
            crate::services::ballot_log_cache::search_session(&session);
        store.client().await.unwrap().execute(
            "UPDATE sequent_backend.ballot_log_search_limit SET searched_at=clock_timestamp()-interval '2 days' WHERE session_key=$1",
            &[&session_key]).await.unwrap();
        limit_search(&session, &store).await.unwrap();
        let page = post(&client, "/ballot-box-log", &ordinary, &scope).await;
        assert_eq!(page.status(), Status::Ok);
        assert_eq!(page.headers().get_one("X-Ballot-Cacheable"), Some("1"));
        assert_eq!(
            page.headers().get_one("Cache-Control"),
            Some("private, no-store")
        );
        let page: Value = page.into_json().await.unwrap();
        assert_eq!(page["payload"]["ballots"].as_array().unwrap().len(), 1);
        let displayed =
            page["payload"]["ballots"][0]["pseudonym"].as_str().unwrap();
        assert_eq!(displayed, code);
        assert!(!page.to_string().contains(voter));
        let entry = post(&client, "/ballot-box-entry", &ordinary, &scope).await;
        assert_eq!(entry.status(), Status::Ok);
        let entry: Value = entry.into_json().await.unwrap();
        assert!(!entry.to_string().contains(voter));
        let ballot_proof: sequent_core::ballot_privacy::BallotPrivacyProof =
            serde_json::from_value(entry["payload"].clone()).unwrap();
        let keys = post(&client, "/ballot-box-keys", &ordinary, &scope).await;
        assert_eq!(keys.status(), Status::Ok);
        let keys: Value = keys.into_json().await.unwrap();
        assert!(!keys.to_string().contains(voter));
        let trust: sequent_core::ballot_privacy::BallotPrivacyTrust =
            serde_json::from_value(keys["payload"].clone()).unwrap();
        assert_eq!(
            trust.system_public_key,
            VerificationKey::from_sk(&key)
                .unwrap()
                .to_b64_string()
                .unwrap()
        );
        assert_eq!(
            trust.ballot_box_key,
            sequent_core::ballot_receipt::ballot_box_key(
                &VerificationKey::from_sk(&receipt_key).unwrap()
            )
            .unwrap()
        );
        let verify = || {
            use sequent_core::ballot_privacy::{
                verify_ballot_log_proof, CheckStatus,
            };
            let checks = verify_ballot_log_proof(&trust, &ballot_proof).checks;
            assert_eq!(checks.binding, CheckStatus::Passed);
            assert_eq!(checks.received, CheckStatus::Passed);
            assert_eq!(checks.ballot_id, CheckStatus::Passed);
            assert_eq!(checks.cast, CheckStatus::Passed);
        };
        verify();
        let body = json!({"election_event_id":event.election_event_id,"pseudonym":displayed,"reason":"Synthetic official investigation"});
        let reader = Claims::new(&event.tenant_id, "viewer")
            .roles([Permissions::LOGS_READ]);
        assert_eq!(
            post(&client, "/resolve-voter-pseudonym", &reader, &body)
                .await
                .status(),
            Status::Unauthorized
        );
        assert!(store
            .pending_privacy_messages(&event.election_event_id, 10)
            .await
            .unwrap()
            .is_empty());
        let investigator =
            Claims::new(&event.tenant_id, "synthetic-investigator")
                .roles([Permissions::VOTER_PSEUDONYM_RESOLVE]);
        let logs = privacy_trace::capture(async {
            let response =
                post(&client, "/resolve-voter-pseudonym", &investigator, &body)
                    .await;
            assert_eq!(response.status(), Status::Ok);
            let response: Value = response.into_json().await.unwrap();
            assert_eq!(response["payload"]["voter_id"], voter);
        })
        .await;
        assert!(
            !logs.contains(voter),
            "resolved voter reached tracing output"
        );
        assert!(!logs.contains(&code), "pseudonym reached tracing output");
        let pending = store
            .pending_privacy_messages(&event.election_event_id, 10)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);
        let message = Message::record_deserialize(&pending[0].message).unwrap();
        message
            .verify(&VerificationKey::from_sk(&key).unwrap())
            .unwrap();
        let json = serde_json::to_value(&message).unwrap();
        let body_json = &json["statement"]["body"]["PseudonymResolved"];
        assert_eq!(body_json["pseudonym"], code);
        assert_eq!(body_json["actor_id"], "synthetic-investigator");
        assert_eq!(body_json["reason"], "Synthetic official investigation");
        assert!(!serde_json::to_string(&message).unwrap().contains(voter));
        let durable_entry = electoral_log::LogEntry {
            delivery_id: pending[0].delivery_id.clone(),
            message: (&message).try_into().unwrap(),
        };
        store
            .append(&board, &mut std::iter::once(Ok(durable_entry.clone())))
            .await
            .unwrap();
        store
            .append(&board, &mut std::iter::once(Ok(durable_entry)))
            .await
            .unwrap();
        store
            .remove_pending_privacy_messages(
                &event.election_event_id,
                &[pending[0].id],
            )
            .await
            .unwrap();
        let records = store
            .query(&board, &electoral_log::LogQuery::default())
            .await
            .unwrap();
        assert_eq!(records.len(), 3);
        let resolution = records
            .iter()
            .find(|row| row.statement_kind == "PseudonymResolved")
            .unwrap();
        let checkpoint = store.journal().checkpoint(&board).await.unwrap();
        let proof = store
            .record_proof(
                &store.journal(),
                electoral_log::adapters::postgres::LogScope::Board(&board),
                resolution.id,
                Some(&checkpoint),
            )
            .await
            .unwrap();
        proof.verify(&checkpoint).unwrap();
        assert_eq!(
            store
                .purge_voter_pseudonyms(&event.election_event_id)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            post(&client, "/resolve-voter-pseudonym", &investigator, &body)
                .await
                .status(),
            Status::NotFound
        );
        assert!(store
            .pending_privacy_messages(&event.election_event_id, 10)
            .await
            .unwrap()
            .is_empty());
        proof.verify(&checkpoint).unwrap();
        verify();
        let own = post(&client, "/ballot-box-own", &ordinary, &scope).await;
        assert_eq!(
            own.status(),
            Status::Ok,
            "purged mapping is an empty own view"
        );
        let own: Value = own.into_json().await.unwrap();
        assert_eq!(own["payload"]["ballots"], json!([]));
        let retained =
            post(&client, "/ballot-box-entry", &ordinary, &scope).await;
        assert_eq!(retained.status(), Status::Ok);
        let retained: Value = retained.into_json().await.unwrap();
        assert_eq!(retained["payload"], entry["payload"]);
        Message::record_deserialize(
            &store
                .query(&board, &electoral_log::LogQuery::default())
                .await
                .unwrap()[0]
                .message,
        )
        .unwrap()
        .verify(&VerificationKey::from_sk(&key).unwrap())
        .unwrap();
        databases
            .drop_event(&event.tenant_id, &event.election_event_id)
            .await
            .unwrap();
    });
}

#[path = "privacy_trace.rs"]
mod privacy_trace;

/// Build a real encrypted, voter-signed received/cast pair for the HTTP proof flow.
async fn signed_private_ballot(
    store: &electoral_log::adapters::postgres::PostgresStore,
    tenant: &str,
    event: &str,
    election: &str,
    area: &str,
    board: &str,
    pseudonym: &str,
    authority_key: &voting_crypto::signing::SigningKey,
    receipt_key: &voting_crypto::signing::SigningKey,
) -> String {
    use chrono::Utc;
    use electoral_log::adapters::ballot_box::{
        AcceptBallot, AcceptOutcome, AcceptReceived, BallotStatus,
    };
    use electoral_log::adapters::ballot_box_received::{
        ReceiveBallot, ReceiveOutcome, ReceivedSignature,
    };
    use electoral_log::ports::ElectoralLogStore;
    use sequent_core::ballot_receipt::*;
    use voting_crypto::signing::{SigningKey, VerificationKey};
    let tenant = tenant.to_owned();
    let event = event.to_owned();
    let election = election.to_owned();
    let area = area.to_owned();
    let pseudonym = pseudonym.to_owned();
    let result: anyhow::Result<String> = async {
        let voter_key = SigningKey::generate()?;
        let published_key =
            ballot_box_key(&VerificationKey::from_sk(&receipt_key)?)?;
        let now = Utc::now();
        let timestamp =
            windmill::services::receive_ballot::format_signed_time(&now);
        use sequent_core::ballot::{
            sign_hashable_ballot_with_voter_signing_key, BallotStyle,
            HashableBallot, SignedHashableBallot,
        };
        use sequent_core::fixtures::ballot_codec::{
            get_test_contest, get_test_decoded_vote_contest,
            get_writein_ballot_style,
        };
        let style = BallotStyle {
            contests: vec![get_test_contest()],
            ..get_writein_ballot_style()
        };
        let ciphertext = sequent_core::encrypt::encrypt_decoded_contest(
            &voting_crypto::RistrettoCtx,
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
            voting_crypto::signing::Signature::from_b64_string(
                &voter_signature,
            )?
            .to_bytes();
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
                        received_signature: received_receipt
                            .received_signature
                            .clone(),
                    })
                },
            )
            .await?
        {
            ReceiveOutcome::Received(ballot) => ballot,
            outcome => anyhow::bail!("Unexpected receive outcome {outcome:?}"),
        };
        let cast_signature =
            sign_cast_statement(&voter_key, &election, &received.ballot_id)?;
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
        let encrypted_hash =
            sequent_core::encrypt::hash_ballot_sha512(&hashable)?;
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
                    cast_receipt_signature: &cast_receipt
                        .cast_receipt_signature,
                },
            )
            .await?;
        assert!(matches!(outcome, AcceptOutcome::Accepted { .. }));

        let queued = store.pending_received_ballots(&event, 10).await?;
        let received = store
            .received_ballot_by_id(
                &event,
                &election,
                &pseudonym,
                &received.ballot_id,
            )
            .await?
            .unwrap();
        let pending = store.pending_ballots(&event, 10).await?;
        let entries = vec![
            windmill::services::vote_privacy::received_log_entry(
                &tenant,
                &queued[0],
                authority_key,
            )?,
            windmill::services::vote_privacy::cast_log_entry(
                &received,
                &tenant,
                &event,
                &pending[0],
                authority_key,
            )?,
        ];
        store
            .append(board, &mut entries.into_iter().map(Ok))
            .await?;
        store
            .remove_pending_received(&event, &[queued[0].id])
            .await?;
        store.remove_pending(&event, &[pending[0].seq]).await?;
        Ok(received.ballot_id)
    }
    .await;
    result.unwrap()
}
