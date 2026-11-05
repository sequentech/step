// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Board authorization and legacy report errors over migrated, private rows.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use protocol_board::{BoardName, TrusteeReport, TrusteeReportKind};
use uuid::Uuid;
use windmill::services::trustee_boards::{self, BoardAccess, BoardRefusal};

struct World {
    tenant: String,
    other_tenant: String,
    board: BoardName,
}

async fn world(tx: &Transaction<'_>) -> World {
    let tenant = Uuid::new_v4();
    let other_tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let ceremony = Uuid::new_v4();
    let member = Uuid::new_v4();
    for id in [tenant, other_tenant] {
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&id, &id.to_string()],
        )
        .await
        .unwrap();
    }
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
    for (id, tenant_id, name) in [
        (member, tenant, "trustee1"),
        (Uuid::new_v4(), tenant, "trustee2"),
        (Uuid::new_v4(), other_tenant, "trustee1"),
    ] {
        tx.execute(
            "INSERT INTO sequent_backend.trustee (id, tenant_id, name) VALUES ($1, $2, $3)",
            &[&id, &tenant_id, &name],
        )
        .await
        .unwrap();
    }
    tx.execute(
        "INSERT INTO sequent_backend.keys_ceremony
         (id, tenant_id, election_event_id, trustee_ids, threshold)
         VALUES ($1, $2, $3, $4, 1)",
        &[&ceremony, &tenant, &event, &vec![member]],
    )
    .await
    .unwrap();
    let board = BoardName::for_dkg(&ceremony);
    tx.execute(
        "INSERT INTO sequent_backend.protocol_board
         (tenant_id, election_event_id, keys_ceremony_id, name, manager_message)
         VALUES ($1, $2, $3, $4, $5)",
        &[
            &tenant,
            &event,
            &ceremony,
            &board.as_str(),
            &Vec::<u8>::new(),
        ],
    )
    .await
    .unwrap();
    World {
        tenant: tenant.to_string(),
        other_tenant: other_tenant.to_string(),
        board,
    }
}

#[tokio::test]
async fn authorization_requires_a_claim_and_membership_of_this_tenants_ceremony() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    match trustee_boards::authorize_board(&tx, &w.tenant, Some("trustee1"), &w.board)
        .await
        .unwrap()
    {
        BoardAccess::Member(member) => {
            assert_eq!(member.name, "trustee1");
            assert_eq!(member.board.name, w.board.as_str());
            assert_eq!(member.board.tenant_id, w.tenant);
        }
        BoardAccess::Refused(reason) => panic!("valid member refused: {reason}"),
    }
    for (tenant, claim, reason, message) in [
        (
            &w.tenant,
            None,
            BoardRefusal::NoTrusteeClaim,
            "the token carries no trustee claim",
        ),
        (
            &w.tenant,
            Some("missing"),
            BoardRefusal::UnknownTrustee,
            "the token's trustee is not a trustee of this tenant",
        ),
        (
            &w.tenant,
            Some("trustee2"),
            BoardRefusal::NotMember,
            "the board is not one of the trustee's boards",
        ),
        (
            &w.other_tenant,
            Some("trustee1"),
            BoardRefusal::NotMember,
            "the board is not one of the trustee's boards",
        ),
    ] {
        match trustee_boards::authorize_board(&tx, tenant, claim, &w.board)
            .await
            .unwrap()
        {
            BoardAccess::Refused(actual) => {
                assert_eq!(actual, reason);
                // Harvest answers this text to a refused board writer.
                assert_eq!(actual.to_string(), message);
            }
            BoardAccess::Member(_) => panic!("nonmember authorized: {claim:?}"),
        }
    }
    let unknown_board = BoardName::for_dkg(&Uuid::new_v4());
    assert!(matches!(
        trustee_boards::authorize_board(&tx, &w.tenant, Some("trustee1"), &unknown_board)
            .await
            .unwrap(),
        BoardAccess::Refused(BoardRefusal::NotMember)
    ));
}

#[tokio::test]
async fn a_failed_membership_lookup_is_an_error_instead_of_an_authorization_refusal() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    assert!(matches!(
        trustee_boards::authorize_board(&tx, &w.tenant, Some("trustee1"), &w.board)
            .await
            .unwrap(),
        BoardAccess::Member(_)
    ));
    tx.batch_execute("ALTER TABLE sequent_backend.trustee RENAME TO unavailable_trustee")
        .await
        .unwrap();
    let error =
        match trustee_boards::authorize_board(&tx, &w.tenant, Some("trustee1"), &w.board).await {
            Err(error) => error,
            Ok(_) => panic!("failed lookup answered an authorization decision"),
        };
    assert_eq!(
        error.to_string(),
        format!("error finding trustee trustee1 in tenant {}", w.tenant)
    );
    assert!(error.chain().any(
        |cause| cause.downcast_ref::<tokio_postgres::Error>().is_some_and(
            |error| error.code() == Some(&tokio_postgres::error::SqlState::UNDEFINED_TABLE)
        )
    ));
}

#[tokio::test]
async fn refused_reports_keep_their_existing_errors() {
    let pool = schema::pool().await;
    for claim in [None, Some("missing"), Some("trustee2")] {
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = world(&tx).await;
        let expected = match claim {
            None => "the token carries no trustee claim".to_string(),
            Some("missing") => format!(
                "error finding trustee missing in tenant {}: Trustee missing not found",
                w.tenant
            ),
            Some(_) => format!("board {} is not one of trustee trustee2's boards", w.board),
        };
        let error = trustee_boards::report(
            tx,
            &w.tenant,
            claim,
            TrusteeReport {
                board: w.board,
                kind: TrusteeReportKind::HALTED,
                detail: "synthetic halt".into(),
            },
        )
        .await
        .unwrap_err();
        assert_eq!(format!("{error:#}"), expected);
    }
}
