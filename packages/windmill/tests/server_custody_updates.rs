// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The durable queue against the migrated schema, with real signed requests.
//! Proof production must never advance the human ceremony approval.
#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use protocol_board::{
    generate_manager, Committee, CustodyRequest, DkgBoard, RawTrusteeRecord, TrusteeActionKind,
    TrusteeActionOutcome, TrusteeActionResult, TrusteeSecrets,
};
use sequent_core::services::jwt::JwtClaims;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::services::trustee_updates::{
    self, RetryPolicy, ServerCustodyInput, ServerCustodyReply,
};

struct Fixture {
    tenant: Uuid,
    event: Uuid,
    keys: Uuid,
    trustee: Uuid,
    secrets: TrusteeSecrets,
    request: CustodyRequest,
    claims: JwtClaims,
}

impl Fixture {
    async fn new(tx: &Transaction<'_>) -> Self {
        let tenant = Uuid::new_v4();
        let event = Uuid::new_v4();
        let keys = Uuid::new_v4();
        let trustee = Uuid::new_v4();
        let peer = Uuid::new_v4();
        let secrets = TrusteeSecrets::generate();
        let public = secrets.public_keys().unwrap();
        let peer_public = TrusteeSecrets::generate().public_keys().unwrap();
        let committee = Committee::new(&[
            RawTrusteeRecord {
                name: "alice".into(),
                signing_public_key: Some(public.signing_public_key.clone()),
                share_encryption_public_key: Some(public.share_encryption_public_key.clone()),
            },
            RawTrusteeRecord {
                name: "bob".into(),
                signing_public_key: Some(peer_public.signing_public_key),
                share_encryption_public_key: Some(peer_public.share_encryption_public_key),
            },
        ])
        .unwrap();
        let dkg = DkgBoard::new(&keys, &generate_manager(), &committee, 2).unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.tenant(id,slug) VALUES ($1,$2)",
            &[&tenant, &tenant.to_string()],
        )
        .await
        .unwrap();
        tx.execute("INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol) VALUES ($1,$2,'RSA256')", &[&event,&tenant]).await.unwrap();
        for (id, name) in [(trustee, "alice"), (peer, "bob")] {
            tx.execute(
                "INSERT INTO sequent_backend.trustee(id,tenant_id,name) VALUES ($1,$2,$3)",
                &[&id, &tenant, &name],
            )
            .await
            .unwrap();
        }
        let status = json!({"public_key": "verified-joint-key", "public_key_hash": "verified-hash", "logs": [], "trustees": [{"name": "alice", "status": "CUSTODY_REQUEST_RETRIEVED"}, {"name": "bob", "status": "KEY_GENERATED"}]});
        tx.execute("INSERT INTO sequent_backend.keys_ceremony(id,tenant_id,election_event_id,trustee_ids,threshold,status,execution_status) VALUES ($1,$2,$3,$4,2,$5,'IN_PROGRESS')", &[&keys,&tenant,&event,&vec![trustee,peer],&status]).await.unwrap();
        tx.execute("INSERT INTO sequent_backend.protocol_board(id,tenant_id,election_event_id,keys_ceremony_id,name,manager_message) VALUES ($1,$2,$3,$4,$5,$6)", &[&Uuid::new_v4(),&tenant,&event,&keys,&dkg.name.as_str(),&dkg.configuration.to_bytes()]).await.unwrap();
        let request = CustodyRequest {
            tenant_id: tenant.to_string(),
            election_event_id: event.to_string(),
            keys_ceremony_id: keys.to_string(),
            tally_session_id: None,
            trustee_name: "alice".into(),
            signing_public_key: public.signing_public_key,
            share_encryption_public_key: public.share_encryption_public_key,
        };
        let user = Uuid::new_v4().to_string();
        let claims = serde_json::from_value(json!({
            "exp": 0, "iat": 0, "jti": "fixture", "iss": "fixture", "sub": user, "typ": "Bearer", "azp": "fixture", "acr": "1", "allowed-origins": [], "scope": "", "email_verified": false, "trustee": "alice",
            "https://hasura.io/jwt/claims": {"x-hasura-default-role": "trustee-ceremony", "x-hasura-allowed-roles": ["trustee-ceremony"], "x-hasura-tenant-id": tenant, "x-hasura-user-id": user}
        })).unwrap();
        Self {
            tenant,
            event,
            keys,
            trustee,
            secrets,
            request,
            claims,
        }
    }

    fn input(&self, tally: Option<Uuid>) -> ServerCustodyInput {
        ServerCustodyInput {
            election_event_id: self.event.to_string(),
            keys_ceremony_id: tally.is_none().then(|| self.keys.to_string()),
            tally_session_id: tally.map(|id| id.to_string()),
            custody_request_json: serde_json::to_string(&self.request).unwrap(),
            retry_policy: RetryPolicy::Keep,
        }
    }

    async fn tally(&self, tx: &Transaction<'_>) -> Uuid {
        let tally = Uuid::new_v4();
        tx.execute("INSERT INTO sequent_backend.tally_session(id,tenant_id,election_event_id,keys_ceremony_id,threshold,execution_status) VALUES ($1,$2,$3,$4,2,'STARTED')", &[&tally,&self.tenant,&self.event,&self.keys]).await.unwrap();
        let status = json!({"logs": [], "trustees": [{"name": "alice", "status": "WAITING"}, {"name": "bob", "status": "WAITING"}], "elections_status": []});
        tx.execute("INSERT INTO sequent_backend.tally_session_execution(id,tenant_id,election_event_id,tally_session_id,current_message_id,status) VALUES ($1,$2,$3,$4,1,$5)", &[&Uuid::new_v4(),&self.tenant,&self.event,&tally,&status]).await.unwrap();
        tally
    }

    async fn pending(&self, tx: &Transaction<'_>, tally: Option<Uuid>) -> TrusteeActionResult {
        let updates = trustee_updates::event_updates(tx, &self.claims)
            .await
            .unwrap();
        let action = updates.actions.into_iter().find(|action| matches!(&action.action, TrusteeActionKind::CreateCustodyProof {request} if request.tally_session_id == tally.map(|id| id.to_string()))).expect("pending action");
        let TrusteeActionKind::CreateCustodyProof { request } = action.action else {
            panic!("wrong action kind")
        };
        let proof = request.attest(&self.secrets).unwrap();
        TrusteeActionResult {
            id: action.id,
            request,
            outcome: TrusteeActionOutcome::Ready {
                custody_proof_json: proof,
            },
        }
    }
}

#[tokio::test]
async fn server_proof_is_durable_idempotent_and_leaves_human_approval_pending() {
    let mut db = schema::pool().await.get().await.unwrap();
    let tx = db.transaction().await.unwrap();
    let f = Fixture::new(&tx).await;
    assert!(trustee_updates::event_updates(&tx, &f.claims)
        .await
        .unwrap()
        .actions
        .is_empty());
    let input = f.input(None);
    assert!(matches!(
        trustee_updates::request_server_custody(&tx, &f.claims, &input)
            .await
            .unwrap(),
        ServerCustodyReply::Pending
    ));
    let result = f.pending(&tx, None).await;
    trustee_updates::request_server_custody(&tx, &f.claims, &input)
        .await
        .unwrap();
    assert_eq!(f.pending(&tx, None).await.id, result.id);
    let mut bad = result.clone();
    bad.id = Uuid::new_v4().to_string();
    assert!(trustee_updates::submit_action_result(&tx, &f.claims, &bad)
        .await
        .is_err());
    bad = result.clone();
    bad.outcome = TrusteeActionOutcome::Ready {
        custody_proof_json: "invalid".into(),
    };
    assert!(trustee_updates::submit_action_result(&tx, &f.claims, &bad)
        .await
        .is_err());
    let mut other = f.claims.clone();
    other.trustee = Some("bob".into());
    assert!(trustee_updates::submit_action_result(&tx, &other, &result)
        .await
        .is_err());
    other = f.claims.clone();
    other.hasura_claims.tenant_id = Uuid::new_v4().to_string();
    assert!(trustee_updates::submit_action_result(&tx, &other, &result)
        .await
        .is_err());
    trustee_updates::submit_action_result(&tx, &f.claims, &result)
        .await
        .unwrap();
    trustee_updates::submit_action_result(&tx, &f.claims, &result)
        .await
        .unwrap();
    assert!(trustee_updates::event_updates(&tx, &f.claims)
        .await
        .unwrap()
        .actions
        .is_empty());
    let reply = trustee_updates::request_server_custody(&tx, &f.claims, &input)
        .await
        .unwrap();
    let ServerCustodyReply::Ready { custody_proof_json } = reply else {
        panic!("missing durable proof")
    };
    assert!(f.request.verifies(&custody_proof_json).unwrap());
    let status: Value = tx
        .query_one(
            "SELECT status FROM sequent_backend.keys_ceremony WHERE id=$1",
            &[&f.keys],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(status["trustees"][0]["status"], "CUSTODY_REQUEST_RETRIEVED");
    let mut replaced = result;
    replaced.outcome = TrusteeActionOutcome::Failed {
        detail: "replace proof".into(),
    };
    assert!(
        trustee_updates::submit_action_result(&tx, &f.claims, &replaced)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn same_saved_file_requests_fresh_proofs_for_each_tally_and_cancelled_work_is_rejected() {
    let mut db = schema::pool().await.get().await.unwrap();
    let tx = db.transaction().await.unwrap();
    let f = Fixture::new(&tx).await;
    let first = f.tally(&tx).await;
    let second = f.tally(&tx).await;
    for tally in [first, second] {
        assert!(matches!(
            trustee_updates::request_server_custody(&tx, &f.claims, &f.input(Some(tally)))
                .await
                .unwrap(),
            ServerCustodyReply::Pending
        ));
    }
    let first_result = f.pending(&tx, Some(first)).await;
    let second_result = f.pending(&tx, Some(second)).await;
    assert_ne!(first_result.id, second_result.id);
    let mut replay = second_result.clone();
    replay.outcome = first_result.outcome.clone();
    assert!(
        trustee_updates::submit_action_result(&tx, &f.claims, &replay)
            .await
            .is_err()
    );
    trustee_updates::submit_action_result(&tx, &f.claims, &first_result)
        .await
        .unwrap();
    let status: Value = tx
        .query_one(
            "SELECT status FROM sequent_backend.tally_session_execution WHERE tally_session_id=$1",
            &[&first],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(status["trustees"][0]["status"], "WAITING");
    tx.execute(
        "UPDATE sequent_backend.tally_session SET execution_status='CANCELLED' WHERE id=$1",
        &[&second],
    )
    .await
    .unwrap();
    assert!(trustee_updates::event_updates(&tx, &f.claims)
        .await
        .unwrap()
        .actions
        .is_empty());
    assert!(
        trustee_updates::submit_action_result(&tx, &f.claims, &second_result)
            .await
            .is_err()
    );
    assert!(
        trustee_updates::request_server_custody(&tx, &f.claims, &f.input(Some(second)))
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn failed_work_retries_with_a_new_id_and_rejects_the_old_result() {
    let mut db = schema::pool().await.get().await.unwrap();
    let tx = db.transaction().await.unwrap();
    let f = Fixture::new(&tx).await;
    let mut input = f.input(None);
    trustee_updates::request_server_custody(&tx, &f.claims, &input)
        .await
        .unwrap();
    let mut failure = f.pending(&tx, None).await;
    failure.outcome = TrusteeActionOutcome::Failed {
        detail: "mismatched runner keys".into(),
    };
    trustee_updates::submit_action_result(&tx, &f.claims, &failure)
        .await
        .unwrap();
    assert!(matches!(
        trustee_updates::request_server_custody(&tx, &f.claims, &input)
            .await
            .unwrap(),
        ServerCustodyReply::Failed { .. }
    ));
    input.retry_policy = RetryPolicy::RetryFailed;
    assert!(matches!(
        trustee_updates::request_server_custody(&tx, &f.claims, &input)
            .await
            .unwrap(),
        ServerCustodyReply::Pending
    ));
    assert_ne!(f.pending(&tx, None).await.id, failure.id);
    assert!(
        trustee_updates::submit_action_result(&tx, &f.claims, &failure)
            .await
            .is_err()
    );
    let mut wrong_file = f.request.clone();
    wrong_file.trustee_name = "bob".into();
    input.custody_request_json = serde_json::to_string(&wrong_file).unwrap();
    assert!(
        trustee_updates::request_server_custody(&tx, &f.claims, &input)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_valid_current_file_can_replace_stale_work_after_an_event_import() {
    let mut db = schema::pool().await.get().await.unwrap();
    let tx = db.transaction().await.unwrap();
    let f = Fixture::new(&tx).await;
    let mut old_request = f.request.clone();
    old_request.election_event_id = Uuid::new_v4().to_string();
    let key = format!("trustee:server-custody:{}", f.trustee);
    let annotations = json!({key: json!({"id": "old-action", "request": old_request, "requested_by": "old-user", "outcome": null}).to_string()});
    tx.execute(
        "UPDATE sequent_backend.keys_ceremony SET annotations=$2 WHERE id=$1",
        &[&f.keys, &annotations],
    )
    .await
    .unwrap();
    assert!(trustee_updates::event_updates(&tx, &f.claims)
        .await
        .unwrap()
        .actions
        .is_empty());
    assert!(matches!(
        trustee_updates::request_server_custody(&tx, &f.claims, &f.input(None))
            .await
            .unwrap(),
        ServerCustodyReply::Pending
    ));
    assert_ne!(f.pending(&tx, None).await.id, "old-action");
    tx.rollback().await.unwrap();
}
