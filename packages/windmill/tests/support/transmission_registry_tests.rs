// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The MIRU custody query on its own: a package is the registered revision
//! only when its archive, EML and XZ are the three members of one revision,
//! each with its recorded content, of the same tenant, event, tally session,
//! Post and country. The send check answers `NotTheSignedPackage` on either
//! side of this query while no hardware quorum exists, so only a direct
//! call tells its answers apart.
//!
//! TEST FICTION: the test database's superuser promotes the tenant and
//! registers the revision, in place of the deployment admin and the package
//! builder. Everything is rolled back with the test's transaction.

use super::{registered_revision, sha256_hex};
use deadpool_postgres::Transaction;
use serde_json::json;
use uuid::Uuid;

#[path = "schema.rs"]
mod schema;

/// The values the custody query binds.
#[derive(Clone)]
struct Custody {
    tenant: Uuid,
    event: Uuid,
    tally: Uuid,
    election: Uuid,
    area: Uuid,
    archive: Uuid,
    archive_sha: Option<String>,
    eml: Uuid,
    eml_sha: String,
    xz: Uuid,
    xz_sha: String,
}

async fn registered(tx: &Transaction<'_>, custody: &Custody) -> bool {
    registered_revision(
        tx,
        custody.tenant,
        custody.event,
        custody.tally,
        custody.election,
        custody.area,
        custody.archive,
        custody.archive_sha.as_deref(),
        custody.eml,
        &custody.eml_sha,
        custody.xz,
        &custody.xz_sha,
    )
    .await
    .unwrap()
}

/// Registers `document` as the `kind` member of `revision`, as the package
/// builder does.
async fn register(
    tx: &Transaction<'_>,
    custody: &Custody,
    revision: Uuid,
    document: Uuid,
    kind: &str,
    sha256: &str,
) -> bool {
    tx.query_one(
        "SELECT sequent_backend.operational_miru_register_artifact(
             $1, $2, $3, $4, $5, $6, $7, $8, $9, NULL)",
        &[
            &custody.tenant,
            &custody.event,
            &custody.tally,
            &custody.election,
            &custody.area,
            &revision,
            &document,
            &kind,
            &sha256,
        ],
    )
    .await
    .unwrap()
    .get(0)
}

/// A hardware tenant with one event, Post, country and tally session, and
/// the ids and digests of a package revision nobody registered yet.
async fn custody(tx: &Transaction<'_>) -> Custody {
    let custody = Custody {
        tenant: Uuid::new_v4(),
        event: Uuid::new_v4(),
        tally: Uuid::new_v4(),
        election: Uuid::new_v4(),
        area: Uuid::new_v4(),
        archive: Uuid::new_v4(),
        archive_sha: Some(sha256_hex(b"the signed archive")),
        eml: Uuid::new_v4(),
        eml_sha: sha256_hex(b"<EML><Count>42</Count></EML>"),
        xz: Uuid::new_v4(),
        xz_sha: sha256_hex(b"the compressed EML"),
    };
    let ceremony = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&custody.tenant, &format!("tenant-{}", custody.tenant)],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&custody.event, &custody.tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election
             (id, tenant_id, election_event_id, presentation, permission_label)
         VALUES ($1, $2, $3, $4, 'custody')",
        &[
            &custody.election,
            &custody.tenant,
            &custody.event,
            &json!({ "i18n": { "en": { "name": "Post custody" } } }),
        ],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
         VALUES ($1, $2, $3, 'Country')",
        &[&custody.area, &custody.tenant, &custody.event],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.keys_ceremony
             (id, tenant_id, election_event_id, trustee_ids, threshold)
         VALUES ($1, $2, $3, '{}', 1)",
        &[&ceremony, &custody.tenant, &custody.event],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tally_session
             (id, tenant_id, election_event_id, keys_ceremony_id, threshold)
         VALUES ($1, $2, $3, $4, 1)",
        &[&custody.tally, &custody.tenant, &custody.event, &ceremony],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.operational_signing_required_tenant (tenant_id) VALUES ($1)",
        &[&custody.tenant],
    )
    .await
    .unwrap();
    custody
}

#[tokio::test]
async fn a_revision_is_registered_only_as_its_three_exact_members() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let exact = custody(&tx).await;
    let archive_sha = exact.archive_sha.clone().unwrap();

    // Nobody registered anything: no revision.
    assert!(!registered(&tx, &exact).await);
    // Two of the three members are not a revision yet.
    assert!(
        register(
            &tx,
            &exact,
            exact.archive,
            exact.archive,
            "archive",
            &archive_sha
        )
        .await
    );
    assert!(register(&tx, &exact, exact.archive, exact.eml, "eml", &exact.eml_sha).await);
    assert!(!registered(&tx, &exact).await);
    assert!(register(&tx, &exact, exact.archive, exact.xz, "xz", &exact.xz_sha).await);

    // The positive: all three members, each with its recorded content.
    assert!(
        registered(&tx, &exact).await,
        "the registered revision is not recognised"
    );

    // Each case differs from the registered revision in exactly one bound
    // value, and is not it.
    let other_id = Uuid::new_v4();
    let other_sha = sha256_hex(b"other bytes");
    let cases: Vec<(&str, Custody)> = vec![
        (
            "another tenant",
            Custody {
                tenant: other_id,
                ..exact.clone()
            },
        ),
        (
            "another event",
            Custody {
                event: other_id,
                ..exact.clone()
            },
        ),
        (
            "another tally session",
            Custody {
                tally: other_id,
                ..exact.clone()
            },
        ),
        (
            "another Post",
            Custody {
                election: other_id,
                ..exact.clone()
            },
        ),
        (
            "another country",
            Custody {
                area: other_id,
                ..exact.clone()
            },
        ),
        (
            "another archive",
            Custody {
                archive: other_id,
                ..exact.clone()
            },
        ),
        (
            "other archive bytes",
            Custody {
                archive_sha: Some(other_sha.clone()),
                ..exact.clone()
            },
        ),
        (
            "no archive digest",
            Custody {
                archive_sha: None,
                ..exact.clone()
            },
        ),
        (
            "another EML",
            Custody {
                eml: other_id,
                ..exact.clone()
            },
        ),
        (
            "other EML bytes",
            Custody {
                eml_sha: other_sha.clone(),
                ..exact.clone()
            },
        ),
        (
            "another XZ",
            Custody {
                xz: other_id,
                ..exact.clone()
            },
        ),
        (
            "other XZ bytes",
            Custody {
                xz_sha: other_sha.clone(),
                ..exact.clone()
            },
        ),
    ];
    for (what, case) in &cases {
        assert!(
            !registered(&tx, case).await,
            "{what} passes for the registered revision"
        );
    }
    // The EML is not the XZ member, nor the XZ the EML, whatever the bytes.
    assert!(
        !registered(
            &tx,
            &Custody {
                eml: exact.xz,
                eml_sha: exact.xz_sha.clone(),
                xz: exact.eml,
                xz_sha: exact.eml_sha.clone(),
                ..exact.clone()
            }
        )
        .await
    );

    // A later revision shares the EML but has no XZ of its own: the XZ of
    // the first revision does not complete it.
    let later = Custody {
        archive: Uuid::new_v4(),
        archive_sha: Some(sha256_hex(b"a later archive")),
        ..exact.clone()
    };
    let later_sha = later.archive_sha.clone().unwrap();
    assert!(
        register(
            &tx,
            &later,
            later.archive,
            later.archive,
            "archive",
            &later_sha
        )
        .await
    );
    assert!(register(&tx, &later, later.archive, later.eml, "eml", &later.eml_sha).await);
    assert!(!registered(&tx, &later).await);
    // The first revision is still itself.
    assert!(registered(&tx, &exact).await);
}
