// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The actual Voters action, approval settlement and one-use Keycloak callback
//! against a private migrated database. Only the PKI and realm-directory rows
//! are synthetic; no broker, HTTP delivery, identity mutation or messaging.
#![cfg(not(feature = "comelec-demo-direct-hsm"))]

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;
#[path = "support/signing_actions.rs"]
mod signing_actions;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::signing::{RequesterSigning, SigningAction};
use sequent_core::types::hasura::core::Document;
use sequent_core::types::permissions::Permissions;
use signing::{caller, world};
use signing_actions::{sign_all, Dispatched};
use uuid::Uuid;
use windmill::postgres::document::get_exportable_document_ids;
use windmill::services::signing::actions::manual_verification::{
    authorize_token, claim_render, finish_render, initiate, is_signed_issuance, may_download,
    may_read_document, Delivery, Phase,
};
use windmill::services::signing::actions::{run_dispatched, NoSeal, ProductionEffects, RunOutcome};

#[tokio::test]
async fn enrollment_and_manual_issuance_are_independent_and_only_the_exact_intent_can_mint_and_redeem(
) {
    let w = world("manual-test-post").await;
    let mut c = w.pool.get().await.unwrap();
    let db: String = c
        .query_one("SELECT current_database()", &[])
        .await
        .unwrap()
        .get(0);
    assert!(db.starts_with("windmill_schema_"));
    // This binary owns one test, and both database adapters use private test data.
    for key in [
        "HOST",
        "PORT",
        "USER",
        "PASSWORD",
        "MANAGER__RECYCLING_METHOD",
    ] {
        if let Ok(value) = std::env::var(format!("HASURA_DB__{key}")) {
            std::env::set_var(format!("KEYCLOAK_DB__{key}"), value);
        }
    }
    std::env::set_var("KEYCLOAK_DB__DBNAME", &db);
    let realm = get_event_realm(&w.tenant.to_string(), &w.event.to_string());
    c.batch_execute("CREATE TABLE realm(id text primary key,name text unique); CREATE TABLE user_entity(id text primary key,realm_id text); CREATE TABLE user_attribute(user_id text,name text,value text);").await.unwrap();
    let voter = "synthetic-voter";
    c.execute("INSERT INTO realm VALUES('test-realm',$1)", &[&realm])
        .await
        .unwrap();
    c.execute("INSERT INTO user_entity VALUES($1,'test-realm')", &[&voter])
        .await
        .unwrap();
    c.execute(
        "INSERT INTO user_attribute VALUES($1,'area-id',$2)",
        &[&voter, &w.area.to_string()],
    )
    .await
    .unwrap();
    let contest = Uuid::new_v4();
    c.execute("INSERT INTO sequent_backend.contest(id,tenant_id,election_event_id,election_id) VALUES($1,$2,$3,$4)",&[&contest,&w.tenant,&w.event,&w.post]).await.unwrap();
    c.execute("INSERT INTO sequent_backend.area_contest(id,tenant_id,election_event_id,area_id,contest_id) VALUES($1,$2,$3,$4,$5)",&[&Uuid::new_v4(),&w.tenant,&w.event,&w.area,&contest]).await.unwrap();
    let action = SigningAction::AuthorizeManualVerification;
    let owner = caller("owner", &[Permissions::VOTER_MANUALLY_VERIFY], &[&w.label]);
    let other = caller("other", &[Permissions::VOTER_MANUALLY_VERIFY], &[&w.label]);
    w.rule(
        SigningAction::ApproveVoter,
        1,
        RequesterSigning::NotAllowed,
        None,
    )
    .await;
    let tx = c.transaction().await.unwrap();
    let unsigned = Uuid::new_v4();
    assert!(matches!(
        initiate(&tx, &owner, w.tenant, w.event, voter, unsigned, &[], None)
            .await
            .unwrap(),
        Delivery::Document { .. }
    ));
    authorize_token(&tx, w.tenant, w.event, voter, Phase::Issue, None, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    w.rule(action, 1, RequesterSigning::NotAllowed, None).await;
    let i = Uuid::new_v4();
    let tx = c.transaction().await.unwrap();
    assert!(
        authorize_token(&tx, w.tenant, w.event, voter, Phase::Issue, None, None)
            .await
            .is_err()
    );
    assert!(
        authorize_token(&tx, w.tenant, w.event, voter, Phase::Redeem, None, None)
            .await
            .is_err()
    );
    assert!(
        initiate(&tx, &owner, w.tenant, w.event, voter, unsigned, &[], None)
            .await
            .is_err()
    );
    let Delivery::Waiting(q) = initiate(&tx, &owner, w.tenant, w.event, voter, i, &[], None)
        .await
        .unwrap()
    else {
        panic!("no document before signatures")
    };
    let Delivery::Waiting(again) = initiate(&tx, &owner, w.tenant, w.event, voter, i, &[], None)
        .await
        .unwrap()
    else {
        panic!("retry must still wait")
    };
    assert_eq!(q.id, again.id);
    assert!(
        initiate(&tx, &other, w.tenant, w.event, voter, i, &[], None)
            .await
            .is_err()
    );
    assert!(initiate(
        &tx,
        &owner,
        w.tenant,
        w.event,
        "different-voter",
        i,
        &[],
        None
    )
    .await
    .is_err());
    let clean:bool=tx.query_one("SELECT authorized_at IS NULL AND document_id IS NULL AND proof_sha256 IS NULL AND minted_at IS NULL FROM sequent_backend.manual_verification_issuance WHERE initiation_id=$1",&[&i]).await.unwrap().get(0);
    assert!(clean);
    tx.commit().await.unwrap();
    let stored = w.request(q.id).await;
    assert_eq!(stored.action, action);
    assert_eq!(stored.election_id, Some(w.post));
    assert_eq!(stored.requested_by, owner.user_id);
    assert_eq!(stored.subject["voter_id"], voter);
    let dispatcher = Dispatched::default();
    sign_all(&w, action, q.id, 1, "manual-signer", &dispatcher).await;
    let task = dispatcher.tasks().pop().unwrap();
    assert!(matches!(
        run_dispatched(&mut c, &ProductionEffects::default(), &NoSeal, &task)
            .await
            .unwrap(),
        RunOutcome::Executed(_)
    ));
    assert!(matches!(
        run_dispatched(&mut c, &ProductionEffects::default(), &NoSeal, &task)
            .await
            .unwrap(),
        RunOutcome::NotClaimed
    ));
    let tx = c.transaction().await.unwrap();
    assert!(
        initiate(&tx, &other, w.tenant, w.event, voter, i, &[], None)
            .await
            .is_err()
    );
    assert!(initiate(
        &tx,
        &owner,
        w.tenant,
        w.event,
        voter,
        i,
        &[],
        Some(Uuid::new_v4())
    )
    .await
    .is_err());
    assert!(initiate(
        &tx,
        &owner,
        w.tenant,
        w.event,
        voter,
        Uuid::new_v4(),
        &[],
        Some(q.id)
    )
    .await
    .is_err());
    // This transaction is confined to the fixture. Neither a newly required
    // HSM nor a raised signing threshold accepts this already-executed legacy
    // request while a credential still does not exist.
    tx.batch_execute("SAVEPOINT hardware_upgrade")
        .await
        .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.operational_signing_required_tenant(tenant_id) VALUES($1)",
        &[&w.tenant],
    )
    .await
    .unwrap();
    assert!(
        initiate(&tx, &owner, w.tenant, w.event, voter, i, &[], Some(q.id))
            .await
            .is_err()
    );
    assert!(tx.query_one("SELECT document_id IS NULL AND minted_at IS NULL FROM sequent_backend.manual_verification_issuance WHERE initiation_id=$1",&[&i]).await.unwrap().get::<_,bool>(0));
    tx.batch_execute("ROLLBACK TO hardware_upgrade; SAVEPOINT stronger_rule")
        .await
        .unwrap();
    tx.execute("UPDATE sequent_backend.signing_rule SET signatures=2,revision=revision+1 WHERE tenant_id=$1 AND election_event_id=$2 AND action=$3",&[&w.tenant,&w.event,&action.to_string()]).await.unwrap();
    assert!(
        initiate(&tx, &owner, w.tenant, w.event, voter, i, &[], Some(q.id))
            .await
            .is_err()
    );
    tx.batch_execute("ROLLBACK TO stronger_rule").await.unwrap();
    let Delivery::Document { id: doc, .. } =
        initiate(&tx, &owner, w.tenant, w.event, voter, i, &[], Some(q.id))
            .await
            .unwrap()
    else {
        panic!("executed intent should release")
    };
    let permit = claim_render(&tx, w.tenant, w.event, voter, doc, &[])
        .await
        .unwrap()
        .unwrap();
    assert!(claim_render(&tx, w.tenant, w.event, voter, doc, &[])
        .await
        .unwrap()
        .is_none());
    assert!(!may_download(&tx, doc, &owner).await.unwrap());
    assert!(authorize_token(
        &tx,
        w.tenant,
        w.event,
        voter,
        Phase::Issue,
        Some(i),
        Some(Uuid::new_v4())
    )
    .await
    .is_err());
    assert!(authorize_token(
        &tx,
        w.tenant,
        w.event,
        "wrong",
        Phase::Issue,
        Some(i),
        Some(permit.proof)
    )
    .await
    .is_err());
    assert!(authorize_token(
        &tx,
        w.tenant,
        w.event,
        voter,
        Phase::Redeem,
        Some(i),
        Some(permit.proof)
    )
    .await
    .is_err());
    authorize_token(
        &tx,
        w.tenant,
        w.event,
        voter,
        Phase::Issue,
        Some(i),
        Some(permit.proof),
    )
    .await
    .unwrap();
    assert!(authorize_token(
        &tx,
        w.tenant,
        w.event,
        voter,
        Phase::Issue,
        Some(i),
        Some(permit.proof)
    )
    .await
    .is_err());
    finish_render(&tx, &permit, true).await.unwrap();
    assert!(may_download(&tx, doc, &owner).await.unwrap());
    assert!(!may_download(&tx, doc, &other).await.unwrap());
    // The issuance ledger, not the document's editable annotations, classifies
    // the PDF: with its purpose cleared it stays private and out of archives.
    tx.execute(
        "INSERT INTO sequent_backend.document(id,tenant_id,election_event_id,is_public)
         VALUES($1,$2,$3,false) ON CONFLICT DO NOTHING",
        &[&doc, &w.tenant, &w.event],
    )
    .await
    .unwrap();
    tx.execute(
        "UPDATE sequent_backend.document SET annotations='{}'::jsonb, name='report.pdf' WHERE id=$1",
        &[&doc],
    )
    .await
    .unwrap();
    let cleared = Document {
        id: doc.to_string(),
        tenant_id: Some(w.tenant.to_string()),
        election_event_id: Some(w.event.to_string()),
        name: Some("report.pdf".into()),
        media_type: None,
        size: None,
        labels: None,
        annotations: Some(serde_json::json!({})),
        created_at: None,
        last_updated_at: None,
        is_public: Some(false),
    };
    assert!(!cleared.is_signed_manual_verification());
    assert!(is_signed_issuance(&tx, doc).await.unwrap());
    assert!(may_read_document(&tx, &cleared, &owner).await.unwrap());
    assert!(!may_read_document(&tx, &cleared, &other).await.unwrap());
    let unrelated = Document {
        id: Uuid::new_v4().to_string(),
        ..cleared.clone()
    };
    assert!(
        !is_signed_issuance(&tx, Uuid::parse_str(&unrelated.id).unwrap())
            .await
            .unwrap()
    );
    assert!(may_read_document(&tx, &unrelated, &other).await.unwrap());
    let archived =
        get_exportable_document_ids(&tx, &w.tenant.to_string(), &w.event.to_string(), true)
            .await
            .unwrap();
    assert!(!archived.contains(&doc.to_string()));
    authorize_token(
        &tx,
        w.tenant,
        w.event,
        voter,
        Phase::Redeem,
        Some(i),
        Some(permit.proof),
    )
    .await
    .unwrap();
    assert!(authorize_token(
        &tx,
        w.tenant,
        w.event,
        voter,
        Phase::Redeem,
        Some(i),
        Some(permit.proof)
    )
    .await
    .is_err());
    let decisions:i64=tx.query_one("SELECT count(*) FROM sequent_backend.applications WHERE tenant_id=$1 AND election_event_id=$2",&[&w.tenant,&w.event]).await.unwrap().get(0);
    assert_eq!(decisions, 0);
    tx.commit().await.unwrap();
}
