// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The check an election event import runs before reading anything, against
//! the migrated schema: a signed package verified with the tenant's trust,
//! anti-rollback against what the tenant imported, revocation lists it has
//! seen, and an unsigned file where the tenant requires signatures. The
//! certificates come from the signing tests' OpenSSL PKI: an RSA and an EC
//! approver, and an EC package signing key.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing_pki.rs"]
#[allow(dead_code)]
mod signing_pki;

use std::collections::BTreeMap;
use std::io::Write;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use deadpool_postgres::Transaction;
use openssl::x509::X509Crl;
use sequent_core::election_config::archive::{zip, Artifact};
use sequent_core::election_config::manifest::{
    approval_payload, file_entries, package, sha256_hex, Approval, ConfigurationRevision, Content,
    Manifest, ManifestFormat, Produced, ReportFormat, ReportSetting,
};
use sequent_core::signing::SignatureAlgorithm;
use serde_json::json;
use signing_pki::{crl, crl_number, crl_with, ec_key, issued, pki_now, Issued, Pki, Spec};
use std::time::Duration;
use tempfile::NamedTempFile;
use uuid::Uuid;
use windmill::postgres::configuration_packages;
use windmill::postgres::document::get_event_or_tenant_document_names;
use windmill::postgres::reports::ReportType;
use windmill::services::import::configuration_package::{admit_document, record};
use windmill::services::import::rejection::problems_of;
use windmill::services::reports::report_variables::{
    configuration_stamp, configuration_stamp_without_template,
};

const SIGNING_KEY_SERIAL: u32 = 4001;

fn signing_key() -> &'static Issued {
    static KEY: std::sync::OnceLock<Issued> = std::sync::OnceLock::new();
    KEY.get_or_init(|| {
        issued(
            &Spec::signer(
                &[
                    ("O", "Election Commission"),
                    ("CN", "Configuration Signing Key"),
                ],
                SIGNING_KEY_SERIAL,
            ),
            ec_key(),
            Some(&Pki::get().individual_ca),
        )
    })
}

fn chain(leaf: &Issued) -> String {
    format!("{}{}", leaf.pem(), Pki::get().individual_ca.pem())
}

fn members() -> Vec<Artifact> {
    vec![
        Artifact {
            name: "official_election_setup.zip".to_string(),
            bytes: zip(&[Artifact {
                name: "export_election_event-1.json".to_string(),
                bytes: b"{}\n".to_vec(),
            }])
            .unwrap(),
        },
        Artifact {
            name: "blueprint.json".to_string(),
            bytes: b"{\"version\":4}".to_vec(),
        },
    ]
}

fn approval(
    manifest: &Manifest,
    who: &Issued,
    name: &str,
    algorithm: SignatureAlgorithm,
) -> Approval {
    let payload = approval_payload(
        &manifest.configuration.external_id,
        manifest.configuration.revision,
        &manifest.content_sha256,
    )
    .unwrap();
    Approval {
        name: name.to_string(),
        role: "approver".to_string(),
        signed_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        algorithm,
        certificate_chain: chain(who),
        signature: STANDARD.encode(who.sign(payload.as_bytes())),
    }
}

fn signed_package(revision: u64) -> Vec<u8> {
    signed_package_with_reports(revision, Vec::new())
}

fn signed_package_with_reports(revision: u64, reports: Vec<ReportSetting>) -> Vec<u8> {
    package_signed_by(revision, reports, signing_key(), Vec::new())
}

/// A package signed with `key`, whose manifest carries `revocation_lists`
/// (PEM).
fn package_signed_by(
    revision: u64,
    reports: Vec<ReportSetting>,
    key: &Issued,
    revocation_lists: Vec<String>,
) -> Vec<u8> {
    let content = Content {
        files: file_entries(&members()).unwrap(),
        ballot_designs: Vec::new(),
        reports,
    };
    let mut manifest = Manifest {
        format: ManifestFormat::V1,
        configuration: ConfigurationRevision {
            external_id: "ov-2028".to_string(),
            name: "Overseas Voting 2028".to_string(),
            revision,
            previous: None,
        },
        content_sha256: content.sha256().unwrap(),
        content,
        approvals: Vec::new(),
        produced: Produced {
            at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            custodian: "Key Custodian".to_string(),
            key_label: "configuration-signing".to_string(),
            producer: BTreeMap::new(),
        },
        revocation_lists,
    };
    let pki = Pki::get();
    manifest.approvals = vec![
        approval(
            &manifest,
            &pki.maria,
            "Maria Santos",
            SignatureAlgorithm::RsaPkcs1Sha256,
        ),
        approval(
            &manifest,
            &pki.jose,
            "Jose Reyes",
            SignatureAlgorithm::EcdsaP256Sha256,
        ),
    ];
    let bytes = manifest.to_bytes().unwrap();
    package(
        "ov-2028.zip",
        &members(),
        &bytes,
        &key.sign(&bytes),
        &chain(key),
    )
    .unwrap()
    .bytes
}

fn file_of(bytes: &[u8]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(bytes).unwrap();
    file
}

async fn tenant(tx: &Transaction<'_>, policy: &str) -> String {
    let id = Uuid::new_v4();
    let root = Pki::get().root.pem();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug, settings) VALUES ($1, $2, $3)",
        &[
            &id,
            &format!("tenant-{id}"),
            &json!({"configuration_signing": {
                "policy": policy,
                "package_roots": root,
                "staff_roots": root,
                "required_approvals": 2,
            }}),
        ],
    )
    .await
    .unwrap();
    id.to_string()
}

fn ids(error: &anyhow::Error) -> Vec<String> {
    problems_of(error)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|problem| problem.id)
        .collect()
}

#[tokio::test]
async fn a_signed_package_is_replaced_by_its_importable_archive() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx, "required").await;

    let (file, package) = admit_document(&tx, &tenant, file_of(&signed_package(1)))
        .await
        .unwrap();
    let package = package.expect("a verified package");
    assert_eq!(std::fs::read(file.path()).unwrap(), members()[0].bytes);
    assert_eq!(package.approvers.len(), 2);
    assert_eq!(package.manifest.configuration.revision, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn the_same_package_twice_and_an_older_revision_are_refused() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx, "required").await;
    let event = Uuid::new_v4().to_string();

    // One package, offered twice: signatures and times differ between
    // packages built separately.
    let second_bytes = signed_package(2);
    let (_, second) = admit_document(&tx, &tenant, file_of(&second_bytes))
        .await
        .unwrap();
    record(&tx, &tenant, &event, &second.unwrap())
        .await
        .unwrap();

    let again = admit_document(&tx, &tenant, file_of(&second_bytes))
        .await
        .unwrap_err();
    assert_eq!(ids(&again), vec!["package.already-imported"]);
    let older = admit_document(&tx, &tenant, file_of(&signed_package(1)))
        .await
        .unwrap_err();
    assert_eq!(ids(&older), vec!["package.rollback"]);
    assert!(admit_document(&tx, &tenant, file_of(&signed_package(3)))
        .await
        .is_ok());

    let (manifest, digest) = configuration_packages::manifest_of_event(&tx, &tenant, &event)
        .await
        .unwrap()
        .expect("the event's package");
    assert_eq!(manifest.configuration.revision, 2);
    assert_eq!(digest.len(), 64);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_revocation_list_the_tenant_has_seen_applies_to_later_imports() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx, "required").await;
    let ca = &Pki::get().individual_ca;
    configuration_packages::remember_revocation_lists(
        &tx,
        &tenant,
        &[crl_with(
            ca,
            &ca.key,
            &[SIGNING_KEY_SERIAL],
            pki_now() - 86_400,
            Some(pki_now() + 6 * 86_400),
            &[crl_number(1)],
        )],
    )
    .await
    .unwrap();

    let refused = admit_document(&tx, &tenant, file_of(&signed_package(1)))
        .await
        .unwrap_err();
    assert_eq!(ids(&refused), vec!["package.revoked-signer"]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_revocation_arrives_in_the_next_package_and_the_revoked_keys_packages_are_refused() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    // The tenant starts with the root certificate and nothing else.
    let tenant = tenant(&tx, "required").await;
    let ca = &Pki::get().individual_ca;
    let replacement = issued(
        &Spec::signer(
            &[
                ("O", "Election Commission"),
                ("CN", "Configuration Signing Key 2"),
            ],
            SIGNING_KEY_SERIAL + 1,
        ),
        ec_key(),
        Some(ca),
    );

    let first = signed_package(1);
    let (_, admitted) = admit_document(&tx, &tenant, file_of(&first)).await.unwrap();
    record(
        &tx,
        &tenant,
        &Uuid::new_v4().to_string(),
        &admitted.unwrap(),
    )
    .await
    .unwrap();
    assert!(configuration_packages::revocation_lists(&tx, &tenant)
        .await
        .unwrap()
        .is_empty());

    // The next package is signed with the replacement key and carries the
    // list that revokes the first key's certificate.
    let list = crl_with(
        ca,
        &ca.key,
        &[SIGNING_KEY_SERIAL],
        pki_now() - 86_400,
        Some(pki_now() + 6 * 86_400),
        &[crl_number(1)],
    );
    let pem = String::from_utf8(X509Crl::from_der(&list).unwrap().to_pem().unwrap()).unwrap();
    let second = package_signed_by(2, Vec::new(), &replacement, vec![pem]);
    let (_, admitted) = admit_document(&tx, &tenant, file_of(&second))
        .await
        .unwrap();
    let admitted = admitted.expect("a verified package");
    assert_eq!(admitted.manifest.configuration.revision, 2);

    // Until that package is imported the tenant has not seen the list.
    assert!(admit_document(&tx, &tenant, file_of(&signed_package(3)))
        .await
        .is_ok());
    record(&tx, &tenant, &Uuid::new_v4().to_string(), &admitted)
        .await
        .unwrap();
    assert_eq!(
        configuration_packages::revocation_lists(&tx, &tenant)
            .await
            .unwrap(),
        vec![list]
    );

    let later = admit_document(&tx, &tenant, file_of(&signed_package(3)))
        .await
        .unwrap_err();
    assert_eq!(ids(&later), vec!["package.revoked-signer"]);
    let earlier = admit_document(&tx, &tenant, file_of(&first))
        .await
        .unwrap_err();
    assert_eq!(ids(&earlier), vec!["package.revoked-signer"]);

    // The replacement key keeps signing, and another tenant that has not
    // imported the list is not affected.
    let next = package_signed_by(3, Vec::new(), &replacement, Vec::new());
    assert!(admit_document(&tx, &tenant, file_of(&next)).await.is_ok());
    let elsewhere = self::tenant(&tx, "required").await;
    assert!(admit_document(&tx, &elsewhere, file_of(&first))
        .await
        .is_ok());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_revocation_list_that_cannot_be_read_is_refused_not_skipped() {
    // Without a CRL number, which RFC 5280 requires: skipping it would let
    // a key it revokes keep signing without anyone being told.
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx, "required").await;
    configuration_packages::remember_revocation_lists(
        &tx,
        &tenant,
        &[crl(&Pki::get().individual_ca, &[SIGNING_KEY_SERIAL])],
    )
    .await
    .unwrap();

    let refused = admit_document(&tx, &tenant, file_of(&signed_package(1)))
        .await
        .unwrap_err();
    assert_eq!(ids(&refused), vec!["package.unreadable-revocation-list"]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn an_unsigned_file_imports_only_where_signatures_are_optional() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let plain = zip(&members()).unwrap();

    let optional = tenant(&tx, "optional").await;
    let (file, package) = admit_document(&tx, &optional, file_of(&plain))
        .await
        .unwrap();
    assert!(package.is_none());
    assert_eq!(std::fs::read(file.path()).unwrap(), plain);

    let required = tenant(&tx, "required").await;
    let refused = admit_document(&tx, &required, file_of(&plain))
        .await
        .unwrap_err();
    assert_eq!(ids(&refused), vec!["package.unsigned"]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_tenant_without_the_setting_imports_unsigned_files_as_before() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&id, &format!("tenant-{id}")],
    )
    .await
    .unwrap();
    let plain = zip(&members()).unwrap();
    let (_, package) = admit_document(&tx, &id.to_string(), file_of(&plain))
        .await
        .unwrap();
    assert!(package.is_none());

    // A signed package is still checked, and with no trust it is refused.
    let refused = admit_document(&tx, &id.to_string(), file_of(&signed_package(1)))
        .await
        .unwrap_err();
    assert_eq!(ids(&refused), vec!["package.untrusted-signer"]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn an_import_waits_for_the_tenants_import_in_progress() {
    // Two connections, as two import tasks have. The tenant is committed so
    // both see it.
    let pool = schema::pool().await;
    let mut setup = pool.get().await.unwrap();
    let setup_tx = setup.transaction().await.unwrap();
    let tenant = tenant(&setup_tx, "required").await;
    setup_tx.commit().await.unwrap();

    let mut first = pool.get().await.unwrap();
    let mut second = pool.get().await.unwrap();
    let first_tx = first.transaction().await.unwrap();
    let (_, newer) = admit_document(&first_tx, &tenant, file_of(&signed_package(2)))
        .await
        .unwrap();

    let second_tx = second.transaction().await.unwrap();
    let older = admit_document(&second_tx, &tenant, file_of(&signed_package(1)));
    tokio::pin!(older);
    assert!(
        tokio::time::timeout(Duration::from_millis(500), &mut older)
            .await
            .is_err(),
        "the second import was admitted while the first had not finished"
    );

    record(
        &first_tx,
        &tenant,
        &Uuid::new_v4().to_string(),
        &newer.unwrap(),
    )
    .await
    .unwrap();
    first_tx.commit().await.unwrap();

    let refused = older.await.unwrap_err();
    assert_eq!(ids(&refused), vec!["package.rollback"]);
}

#[tokio::test]
async fn an_unsigned_import_does_not_wait_for_a_package_import() {
    let pool = schema::pool().await;
    let mut setup = pool.get().await.unwrap();
    let setup_tx = setup.transaction().await.unwrap();
    let tenant = tenant(&setup_tx, "optional").await;
    setup_tx.commit().await.unwrap();

    let mut first = pool.get().await.unwrap();
    let mut second = pool.get().await.unwrap();
    let first_tx = first.transaction().await.unwrap();
    admit_document(&first_tx, &tenant, file_of(&signed_package(1)))
        .await
        .unwrap();

    let second_tx = second.transaction().await.unwrap();
    let plain = zip(&members()).unwrap();
    let (_, package) = tokio::time::timeout(
        Duration::from_secs(5),
        admit_document(&second_tx, &tenant, file_of(&plain)),
    )
    .await
    .expect("an unsigned import reads nothing the lock protects")
    .unwrap();
    assert!(package.is_none());
    first_tx.rollback().await.unwrap();
}

async fn event(tx: &Transaction<'_>, tenant: &str) -> String {
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&id, &Uuid::parse_str(tenant).unwrap()],
    )
    .await
    .unwrap();
    id.to_string()
}

async fn document(tx: &Transaction<'_>, tenant: &str, event: Option<&str>, name: &str) -> String {
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.document (id, tenant_id, election_event_id, name)
         VALUES ($1, $2, $3, $4)",
        &[
            &id,
            &Uuid::parse_str(tenant).unwrap(),
            &event.map(|event| Uuid::parse_str(event).unwrap()),
            &name,
        ],
    )
    .await
    .unwrap();
    id.to_string()
}

#[tokio::test]
async fn an_imported_candidate_image_is_found_among_the_tenants_documents() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx, "required").await;
    let other_tenant = self::tenant(&tx, "required").await;
    let event = event(&tx, &tenant).await;
    let other_event = self::event(&tx, &tenant).await;

    // The importer stores a candidate's image with no event.
    let imported = document(&tx, &tenant, None, "maria-santos.png").await;
    let uploaded = document(&tx, &tenant, Some(&event), "jose-reyes.png").await;
    let elsewhere = document(&tx, &tenant, Some(&other_event), "elsewhere.png").await;
    let foreign = document(&tx, &other_tenant, None, "foreign.png").await;
    let unknown = Uuid::new_v4().to_string();

    let names = get_event_or_tenant_document_names(
        &tx,
        &tenant,
        &event,
        &[
            imported.clone(),
            uploaded.clone(),
            elsewhere,
            foreign,
            unknown,
        ],
    )
    .await
    .unwrap();
    assert_eq!(names.len(), 2);
    assert_eq!(names[&imported], "maria-santos.png");
    assert_eq!(names[&uploaded], "jose-reyes.png");

    assert!(
        get_event_or_tenant_document_names(&tx, &tenant, &event, &[])
            .await
            .unwrap()
            .is_empty()
    );
    tx.rollback().await.unwrap();
}

const APPROVED_TEMPLATE: &str = "<h1>{{election_name}}</h1>";

#[tokio::test]
async fn a_report_is_stamped_only_with_the_template_its_configuration_approved() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx, "required").await;
    let event = Uuid::new_v4().to_string();
    let reports = vec![
        ReportSetting {
            report_type: ReportType::ELECTORAL_RESULTS.to_string(),
            formats: vec![ReportFormat::Pdf],
            copies: 1,
            template: Some("results".to_string()),
            template_sha256: Some(sha256_hex(APPROVED_TEMPLATE.as_bytes())),
        },
        ReportSetting {
            report_type: ReportType::ACTIVITY_LOGS.to_string(),
            formats: vec![ReportFormat::Csv],
            copies: 1,
            template: None,
            template_sha256: None,
        },
    ];
    let (_, package) = admit_document(
        &tx,
        &tenant,
        file_of(&signed_package_with_reports(4, reports)),
    )
    .await
    .unwrap();
    let package = package.expect("a verified package");
    record(&tx, &tenant, &event, &package).await.unwrap();

    let stamp = configuration_stamp(
        &tx,
        &tenant,
        &event,
        &ReportType::ELECTORAL_RESULTS,
        APPROVED_TEMPLATE,
    )
    .await
    .unwrap()
    .expect("the event's stamp");
    assert_eq!(stamp.external_id, "ov-2028");
    assert_eq!(stamp.revision, 4);
    assert_eq!(stamp.manifest_sha256, package.manifest_sha256);
    assert_eq!(
        stamp.template_sha256,
        sha256_hex(APPROVED_TEMPLATE.as_bytes())
    );

    // The tally's results report with another template, or with none.
    for changed in ["<h1>Results</h1>", ""] {
        let refused = configuration_stamp(
            &tx,
            &tenant,
            &event,
            &ReportType::ELECTORAL_RESULTS,
            changed,
        )
        .await
        .unwrap_err();
        assert!(
            refused
                .to_string()
                .contains("not the one the signed configuration approved"),
            "{refused}"
        );
    }

    // A report the configuration sets no design for takes any template.
    for report_type in [ReportType::ACTIVITY_LOGS, ReportType::BALLOT_RECEIPT] {
        let stamp = configuration_stamp(&tx, &tenant, &event, &report_type, "<p>any</p>")
            .await
            .unwrap()
            .expect("the event's stamp");
        assert_eq!(stamp.revision, 4);
        assert_eq!(stamp.template_sha256, sha256_hex(b"<p>any</p>"));
    }

    let without = configuration_stamp_without_template(&tx, &tenant, &event)
        .await
        .unwrap()
        .expect("the event's stamp");
    assert_eq!(without.revision, 4);
    assert_eq!(without.manifest_sha256, package.manifest_sha256);
    assert_eq!(without.template_sha256, sha256_hex(b""));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn an_event_without_a_package_has_no_stamp() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx, "optional").await;
    let event = Uuid::new_v4().to_string();

    assert!(configuration_stamp(
        &tx,
        &tenant,
        &event,
        &ReportType::ELECTORAL_RESULTS,
        "<h1>Results</h1>",
    )
    .await
    .unwrap()
    .is_none());
    assert!(configuration_stamp_without_template(&tx, &tenant, &event)
        .await
        .unwrap()
        .is_none());
    tx.rollback().await.unwrap();
}
