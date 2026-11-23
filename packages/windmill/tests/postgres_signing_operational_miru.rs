// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! What leaves the server of a tenant that requires hardware signing: an
//! event's documents are neither exported nor linked until the deployment
//! verified the event's MIRU inventory, and a registered MIRU object never
//! is; a transmission package is sent only as the exact revision the MIRU
//! registry recorded, executed for the same tally session and destinations,
//! and with a hardware quorum; and the manual archive route gives nothing to
//! a caller it doesn't know. A legacy tenant is as before.
//!
//! TEST FICTION: the registry, the inventory and the request's execution
//! are written here directly by the test database's superuser, in place of
//! the package builder, the deployment admin and the device-backed approval.
//! No test signs an EML with hardware (the ledger has no producer of an EML
//! device grant), so a package never gets as far as being sendable here:
//! the tests show each refusal on the way.
//!
//! Each test commits its own tenant: a promotion is one-way.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use sequent_core::signing::{RequesterSigning, SigningAction, SigningRequestStatus};
use sequent_core::types::hasura::core::Document;
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use signing::*;
use std::collections::HashSet;
use uuid::Uuid;
use windmill::postgres::document::get_exportable_document_ids;
use windmill::postgres::signing::{update_signing_request_status, SigningRequestTransition};
use windmill::services::consolidation::download_log::{
    prepare_hardware_manual_archive, DownloadRefusal, Downloader, TransmissionDownload,
};
use windmill::services::consolidation::eml_generator::{
    prepend_miru_annotation, MIRU_TALLY_SESSION_DATA,
};
use windmill::services::consolidation::xz_compress::xz_compress;
use windmill::services::documents::ordinary_document_url_allowed;
use windmill::services::signing::actions::transmission::{
    guard_transmission_package, sha256_hex, transmission_send_check, transmission_send_check_bound,
    PackageToSign, TransmissionRefusal, TransmissionSendCheck,
};
use windmill::services::signing::SigningCaller;
use windmill::types::miru_plugin::{
    MiruCcsServer, MiruDocument, MiruDocumentIds, MiruTransmissionPackageData,
};

const ACTION: SigningAction = SigningAction::TransmitResults;
const EXPORT_NOT_VERIFIED: &str = "Operational MIRU export inventory not yet verified";
const DOCUMENT_NOT_VERIFIED: &str = "Operational document inventory not yet verified";

/// Promotes the world's tenant to hardware signing: one-way, committed.
async fn promote(w: &World) {
    w.execute(
        "INSERT INTO sequent_backend.operational_signing_required_tenant (tenant_id) VALUES ($1)",
        &[&w.tenant],
    )
    .await;
}

/// A stored document's row in `event` of the world's tenant.
async fn document_row(w: &World, event: Uuid, name: &str) -> Uuid {
    let id = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.document
             (id, tenant_id, election_event_id, name, media_type, size)
         VALUES ($1, $2, $3, $4, 'application/octet-stream', 1)",
        &[&id, &w.tenant, &event, &name],
    )
    .await;
    id
}

/// The document as the generic routes read it.
fn stored(w: &World, id: Uuid, event: Option<Uuid>) -> Document {
    Document {
        id: id.to_string(),
        tenant_id: Some(w.tenant.to_string()),
        election_event_id: event.map(|event| event.to_string()),
        name: Some("document".into()),
        media_type: Some("application/octet-stream".into()),
        size: Some(1),
        labels: None,
        annotations: None,
        created_at: None,
        last_updated_at: None,
        is_public: Some(false),
    }
}

/// The documents an export of `event` may include.
async fn exportable(w: &World, event: Uuid) -> anyhow::Result<HashSet<String>> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    get_exportable_document_ids(&tx, &w.tenant.to_string(), &event.to_string(), false).await
}

/// Whether a generic route may link `document`.
async fn linkable(w: &World, document: &Document) -> anyhow::Result<bool> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    ordinary_document_url_allowed(&tx, &w.tenant.to_string(), document).await
}

fn ids(documents: &[Uuid]) -> HashSet<String> {
    documents.iter().map(Uuid::to_string).collect()
}

/// A tally session of the world's event.
async fn tally_session(w: &World) -> Uuid {
    tally_session_with(w, &[]).await
}

/// A tally session of the world's event that holds `packages`, stored as
/// the package builder stores them.
async fn tally_session_with(w: &World, packages: &[MiruTransmissionPackageData]) -> Uuid {
    let (ceremony, session) = (Uuid::new_v4(), Uuid::new_v4());
    let annotations = (!packages.is_empty()).then(|| {
        let mut annotations = serde_json::Map::new();
        annotations.insert(
            prepend_miru_annotation(MIRU_TALLY_SESSION_DATA),
            json!(serde_json::to_string(packages).unwrap()),
        );
        serde_json::Value::Object(annotations)
    });
    w.execute(
        "INSERT INTO sequent_backend.keys_ceremony
             (id, tenant_id, election_event_id, trustee_ids, threshold)
         VALUES ($1, $2, $3, '{}', 1)",
        &[&ceremony, &w.tenant, &w.event],
    )
    .await;
    w.execute(
        "INSERT INTO sequent_backend.tally_session
             (id, tenant_id, election_event_id, keys_ceremony_id, threshold, annotations)
         VALUES ($1, $2, $3, $4, 1, $5)",
        &[&session, &w.tenant, &w.event, &ceremony, &annotations],
    )
    .await;
    session
}

/// The world's country votes in a contest of the world's Post: the link the
/// manual archive route reads the event of a Post and country from.
async fn link_country_to_post(w: &World) {
    let contest = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
         VALUES ($1, $2, $3, $4)",
        &[&contest, &w.tenant, &w.event, &w.post],
    )
    .await;
    w.execute(
        "INSERT INTO sequent_backend.area_contest
             (id, tenant_id, election_event_id, area_id, contest_id)
         VALUES ($1, $2, $3, $4, $5)",
        &[&Uuid::new_v4(), &w.tenant, &w.event, &w.area, &contest],
    )
    .await;
}

/// Registers `document` as the `kind` member of package revision `revision`
/// of the world's Post and country, as the package builder does.
async fn register(
    w: &World,
    tally: Uuid,
    revision: Uuid,
    document: Uuid,
    kind: &str,
    sha256: &str,
) -> bool {
    w.pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT sequent_backend.operational_miru_register_artifact(
                 $1, $2, $3, $4, $5, $6, $7, $8, $9, NULL)",
            &[
                &w.tenant, &w.event, &tally, &w.post, &w.area, &revision, &document, &kind, &sha256,
            ],
        )
        .await
        .unwrap()
        .get(0)
}

/// The deployment admin opens the inventory of `event`.
async fn open_inventory(w: &World, event: Uuid) {
    w.execute(
        "INSERT INTO sequent_backend.operational_miru_inventory (tenant_id, event_id)
         VALUES ($1, $2)",
        &[&w.tenant, &event],
    )
    .await;
}

/// The deployment admin attests that the inventory of `event` is complete.
async fn verify_inventory(w: &World, event: Uuid) {
    w.execute(
        "UPDATE sequent_backend.operational_miru_inventory
         SET state = 'verified', verified_at = clock_timestamp()
         WHERE tenant_id = $1 AND event_id = $2",
        &[&w.tenant, &event],
    )
    .await;
}

/// Neither the export of the world's event nor a link to `document` is
/// answered while the event's inventory is not verified.
async fn assert_unverified(w: &World, document: &Document) {
    assert_eq!(
        exportable(w, w.event).await.unwrap_err().to_string(),
        EXPORT_NOT_VERIFIED
    );
    assert_eq!(
        linkable(w, document).await.unwrap_err().to_string(),
        DOCUMENT_NOT_VERIFIED
    );
}

#[tokio::test]
async fn a_legacy_tenant_exports_and_links_every_document() {
    let w = world("miru-legacy").await;
    let first = document_row(&w, w.event, "report.pdf").await;
    let second = document_row(&w, w.event, "er.xml").await;
    assert_eq!(
        exportable(&w, w.event).await.unwrap(),
        ids(&[first, second])
    );
    assert!(linkable(&w, &stored(&w, first, Some(w.event)))
        .await
        .unwrap());
    // Ids that are no ids are an error, not an answer.
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert!(
        ordinary_document_url_allowed(&tx, "tenant", &stored(&w, first, Some(w.event)))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_hardware_tenant_releases_documents_only_of_a_verified_inventory_and_never_miru_objects()
{
    let w = world("miru-inventory").await;
    promote(&w).await;
    let tally = tally_session(&w).await;
    let report = document_row(&w, w.event, "report.pdf").await;
    let archive = document_row(&w, w.event, "all_servers.zip").await;
    let eml = document_row(&w, w.event, "er.xml").await;
    let xz = document_row(&w, w.event, "er.xz").await;
    let in_event = |id: Uuid| stored(&w, id, Some(w.event));

    // Nobody classified the event's documents yet: nothing is released.
    assert_unverified(&w, &in_event(report)).await;
    // An opened inventory is not a verified one.
    open_inventory(&w, w.event).await;
    assert_unverified(&w, &in_event(report)).await;
    // A document of no event is never MIRU output.
    assert!(linkable(&w, &stored(&w, Uuid::new_v4(), None))
        .await
        .unwrap());

    // The package builder registers a revision's objects. A registered
    // object is withheld from the generic routes at once.
    assert!(register(&w, tally, archive, archive, "archive", &sha("archive")).await);
    assert!(register(&w, tally, archive, eml, "eml", &sha("eml")).await);
    assert!(register(&w, tally, archive, xz, "xz", &sha("xz")).await);
    // The same registration again changes nothing; other content doesn't fit.
    assert!(register(&w, tally, archive, eml, "eml", &sha("eml")).await);
    assert!(!register(&w, tally, archive, eml, "eml", &sha("other")).await);
    for registered in [archive, eml, xz] {
        assert!(
            !linkable(&w, &in_event(registered)).await.unwrap(),
            "a registered object is linked"
        );
    }
    assert_unverified(&w, &in_event(report)).await;

    // The verified inventory releases what is not a MIRU object.
    verify_inventory(&w, w.event).await;
    assert_eq!(exportable(&w, w.event).await.unwrap(), ids(&[report]));
    assert!(linkable(&w, &in_event(report)).await.unwrap());
    for registered in [archive, eml, xz] {
        assert!(!linkable(&w, &in_event(registered)).await.unwrap());
    }
    // An object registered after the verification is left out as well.
    let later = document_row(&w, w.event, "later.zip").await;
    assert_eq!(
        exportable(&w, w.event).await.unwrap(),
        ids(&[report, later])
    );
    assert!(register(&w, tally, later, later, "archive", &sha("later")).await);
    assert_eq!(exportable(&w, w.event).await.unwrap(), ids(&[report]));

    // Another event of the tenant has its own inventory: verified and
    // without documents, its export is empty.
    let other = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&other, &w.tenant],
    )
    .await;
    assert_eq!(
        exportable(&w, other).await.unwrap_err().to_string(),
        EXPORT_NOT_VERIFIED
    );
    open_inventory(&w, other).await;
    verify_inventory(&w, other).await;
    assert!(exportable(&w, other).await.unwrap().is_empty());
}

fn destinations() -> Vec<MiruCcsServer> {
    vec![MiruCcsServer {
        name: "central".into(),
        tag: "tag-central".into(),
        address: "https://central.invalid".into(),
        public_key_pem: String::new(),
        send_logs: None,
    }]
}

/// The send check of `package`, bound to `tally` and to the digest of the
/// archive bytes about to leave, as the release does.
async fn send_check(
    w: &World,
    tally: Option<Uuid>,
    package: &MiruTransmissionPackageData,
    observed_archive_sha256: Option<&str>,
) -> Result<TransmissionSendCheck, TransmissionRefusal> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    transmission_send_check_bound(
        &tx,
        w.tenant,
        w.event,
        tally,
        package,
        observed_archive_sha256,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn a_hardware_package_is_not_sent_without_its_registered_revision_and_a_hardware_quorum() {
    let w = world("miru-send").await;
    w.rule(ACTION, 1, RequesterSigning::NotAllowed, None).await;
    promote(&w).await;
    let tally = tally_session(&w).await;
    let tally_text = tally.to_string();
    let eml = b"<EML><Count>42</Count></EML>".to_vec();
    let compressed = xz_compress(&eml).unwrap();
    let servers = destinations();
    let (archive_id, eml_id, xz_id) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());

    // The package waits for its request, whose subject pins the exact
    // destination configuration for a hardware tenant.
    let operator = caller("operator", &[Permissions::MIRU_CREATE], &[&w.label]);
    let signing = {
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let signing = guard_transmission_package(
            &tx,
            Some(&operator),
            &PackageToSign {
                tenant_id: w.tenant,
                election_event_id: w.event,
                election_id: w.post,
                area_id: w.area,
                tally_session_id: &tally_text,
                eml_document_id: eml_id,
                eml: &eml,
                package: &compressed,
                servers: &servers,
            },
        )
        .await
        .unwrap()
        .expect("the rule requires signatures");
        tx.commit().await.unwrap();
        signing
    };
    let id: Uuid = signing.id.parse().unwrap();
    let request = w.request(id).await;
    let destination_sha256 = request.subject["destination_configuration_sha256"]
        .as_str()
        .expect("a hardware request pins its destinations")
        .to_owned();
    assert_eq!(destination_sha256.len(), 64);
    assert_eq!(request.document_id, Some(eml_id));

    let package = MiruTransmissionPackageData {
        election_id: w.post.to_string(),
        area_id: w.area.to_string(),
        servers: servers.clone(),
        documents: vec![MiruDocument {
            document_ids: MiruDocumentIds {
                eml: eml_id.to_string(),
                xz: xz_id.to_string(),
                all_servers: archive_id.to_string(),
                csv: None,
            },
            transaction_id: "tx".into(),
            servers_sent_to: vec![],
            created_at: "2028-05-12T10:00:00+08:00".into(),
            signatures: vec![],
            sha256: None,
        }],
        logs: vec![],
        threshold: 1,
        signing_request: Some(signing.clone()),
        ovcs_public_key: None,
    };
    // While its request waits, the package is not signed.
    assert_eq!(
        send_check(&w, Some(tally), &package, None).await,
        Err(TransmissionRefusal::NotSigned {
            code: Some(signing.code.clone()),
            status: Some(SigningRequestStatus::Waiting),
        })
    );

    // FICTION: the request is completed and executed here without any
    // approval, with the result the signed-package builder records.
    let archive_sha256 = sha256_hex(b"the signed archive");
    let result = json!({
        "all_servers_document_id": archive_id.to_string(),
        "archive_sha256": archive_sha256,
        "eml_document_id": eml_id.to_string(),
        "xz_document_id": xz_id.to_string(),
        "tally_session_id": tally_text,
        "destination_configuration_sha256": destination_sha256,
        "package_sha256": sha256_hex(&compressed),
        "eml_sha256": sha256_hex(&eml),
        "signatures": 0,
    });
    {
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        update_signing_request_status(
            &tx,
            w.tenant,
            w.event,
            id,
            &SigningRequestTransition::Complete,
        )
        .await
        .unwrap()
        .expect("the request completes");
        update_signing_request_status(
            &tx,
            w.tenant,
            w.event,
            id,
            &SigningRequestTransition::Execute {
                result: Some(result),
            },
        )
        .await
        .unwrap()
        .expect("the request executes");
        tx.commit().await.unwrap();
    }
    let not_it = Err(TransmissionRefusal::NotTheSignedPackage {
        code: signing.code.clone(),
    });

    // A preflight that names no tally session can't release bytes.
    {
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        assert_eq!(
            transmission_send_check(&tx, w.tenant, w.event, &package)
                .await
                .unwrap(),
            not_it
        );
    }
    // The registry knows no revision of these three documents.
    assert_eq!(send_check(&w, Some(tally), &package, None).await, not_it);

    // The builder's registrations: the archive, and the EML and XZ it holds.
    assert!(
        register(
            &w,
            tally,
            archive_id,
            archive_id,
            "archive",
            &archive_sha256
        )
        .await
    );
    assert!(register(&w, tally, archive_id, eml_id, "eml", &sha256_hex(&eml)).await);
    assert!(register(&w, tally, archive_id, xz_id, "xz", &sha256_hex(&compressed)).await);
    let registered: i64 = w
        .pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT count(*) FROM sequent_backend.operational_miru_revision_artifact
             WHERE tenant_id = $1 AND event_id = $2 AND revision_id = $3",
            &[&w.tenant, &w.event, &archive_id],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(registered, 3);

    // The revision is the registered one now. The request still has no
    // applied hardware signature, so the package stays unsendable: the same
    // answer, one check further.
    assert_eq!(send_check(&w, Some(tally), &package, None).await, not_it);
    // Neither with the archive's own digest observed, nor another one.
    assert_eq!(
        send_check(&w, Some(tally), &package, Some(archive_sha256.as_str())).await,
        not_it
    );
    let other_bytes = sha256_hex(b"other bytes");
    assert_eq!(
        send_check(&w, Some(tally), &package, Some(other_bytes.as_str())).await,
        not_it
    );
    // Nor for another tally session, another archive, or other destinations.
    assert_eq!(
        send_check(&w, Some(Uuid::new_v4()), &package, None).await,
        not_it
    );
    let mut rebuilt = package.clone();
    rebuilt.documents[0].document_ids.xz = Uuid::new_v4().to_string();
    assert_eq!(send_check(&w, Some(tally), &rebuilt, None).await, not_it);
    let mut redirected = package.clone();
    redirected.servers[0].address = "https://elsewhere.invalid".into();
    assert_eq!(send_check(&w, Some(tally), &redirected, None).await, not_it);
    let mut unparsable = package.clone();
    unparsable.documents[0].document_ids.eml = "eml".into();
    assert_eq!(send_check(&w, Some(tally), &unparsable, None).await, not_it);
}

/// What the manual archive route answers `caller` for `download`.
async fn manual_archive(
    w: &World,
    download: &TransmissionDownload,
    caller: &SigningCaller,
    downloader: &Downloader,
) -> Option<DownloadRefusal> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    prepare_hardware_manual_archive(&tx, download, caller, downloader)
        .await
        .unwrap()
        .err()
}

#[tokio::test]
async fn the_manual_archive_route_gives_nothing_to_a_caller_it_does_not_know() {
    let w = world("miru-manual").await;
    // The event's required transmit-results rule: a hardware tenant's send
    // check is an error without one, not an answer.
    w.rule(ACTION, 1, RequesterSigning::NotAllowed, None).await;
    // A real package of the world's Post and country, for a destination
    // that receives it by hand, in a tally session of the event: every
    // refusal below is of a download that names this package's own archive.
    link_country_to_post(&w).await;
    let archive = Uuid::new_v4();
    let package = MiruTransmissionPackageData {
        election_id: w.post.to_string(),
        area_id: w.area.to_string(),
        servers: vec![MiruCcsServer {
            name: "by-hand".into(),
            tag: "tag-by-hand".into(),
            address: String::new(),
            public_key_pem: String::new(),
            send_logs: None,
        }],
        documents: vec![MiruDocument {
            document_ids: MiruDocumentIds {
                eml: Uuid::new_v4().to_string(),
                xz: Uuid::new_v4().to_string(),
                all_servers: archive.to_string(),
                csv: None,
            },
            transaction_id: "tx".into(),
            servers_sent_to: vec![],
            created_at: "2028-05-12T10:00:00+08:00".into(),
            signatures: vec![],
            sha256: None,
        }],
        logs: vec![],
        threshold: 1,
        signing_request: None,
        ovcs_public_key: None,
    };
    let tally = tally_session_with(&w, &[package]).await;
    let download_of = |election: Uuid, tally: Uuid| TransmissionDownload {
        tenant_id: w.tenant.to_string(),
        election_event_id: w.event.to_string(),
        election_id: election.to_string(),
        area_id: w.area.to_string(),
        tally_session_id: tally.to_string(),
        document_id: archive.to_string(),
    };
    let download = |election: Uuid| download_of(election, tally);
    let downloader = Downloader {
        user_id: "auditor".into(),
        username: Some("auditor-name".into()),
        name: "Auditor".into(),
    };
    let auditor = caller("auditor", &[Permissions::MIRU_DOWNLOAD], &[&w.label]);

    // Without the download permission, or on someone else's behalf.
    assert_eq!(
        manual_archive(
            &w,
            &download(w.post),
            &caller("auditor", &[], &[&w.label]),
            &downloader
        )
        .await,
        Some(DownloadRefusal::NoPackage)
    );
    assert_eq!(
        manual_archive(
            &w,
            &download(w.post),
            &caller("someone-else", &[Permissions::MIRU_DOWNLOAD], &[&w.label]),
            &downloader
        )
        .await,
        Some(DownloadRefusal::NoPackage)
    );
    // The route is the hardware one: a legacy tenant has no package there.
    assert_eq!(
        manual_archive(&w, &download(w.post), &auditor, &downloader).await,
        Some(DownloadRefusal::NoPackage)
    );

    promote(&w).await;
    // The control: the same download by the same permitted caller, refused
    // a moment ago for the legacy tenant, now passes every "no package"
    // refusal. The route finds the package and its archive, and stops one
    // check further: the rule requires signatures and no request signed
    // this package (TEST FICTION: no hardware quorum exists here, so the
    // archive itself is never handed out).
    assert_eq!(
        manual_archive(&w, &download(w.post), &auditor, &downloader).await,
        Some(DownloadRefusal::NotAPackageFile),
        "a permitted caller does not reach the package"
    );
    // Promotion alone opens nothing to the callers the route doesn't know.
    assert_eq!(
        manual_archive(
            &w,
            &download(w.post),
            &caller("auditor", &[], &[&w.label]),
            &downloader
        )
        .await,
        Some(DownloadRefusal::NoPackage)
    );
    assert_eq!(
        manual_archive(
            &w,
            &download(w.post),
            &caller("someone-else", &[Permissions::MIRU_DOWNLOAD], &[&w.label]),
            &downloader
        )
        .await,
        Some(DownloadRefusal::NoPackage)
    );
    // A Post the event doesn't have, and a Post the caller doesn't reach.
    assert_eq!(
        manual_archive(&w, &download(Uuid::new_v4()), &auditor, &downloader).await,
        Some(DownloadRefusal::NoPackage)
    );
    assert_eq!(
        manual_archive(
            &w,
            &download(w.post),
            &caller("auditor", &[Permissions::MIRU_DOWNLOAD], &["another-post"]),
            &downloader
        )
        .await,
        Some(DownloadRefusal::NoPackage)
    );
    // A tally session of the event that holds no package.
    let empty = tally_session(&w).await;
    assert_eq!(
        manual_archive(&w, &download_of(w.post, empty), &auditor, &downloader).await,
        Some(DownloadRefusal::NoPackage)
    );
    // A tenant that is no id is an error.
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let mut unreadable = download(w.post);
    unreadable.tenant_id = "tenant".into();
    assert!(
        prepare_hardware_manual_archive(&tx, &unreadable, &auditor, &downloader)
            .await
            .is_err()
    );
}
