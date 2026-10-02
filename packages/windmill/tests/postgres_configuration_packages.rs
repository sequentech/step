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
use sequent_core::election_config::archive::{zip, Artifact};
use sequent_core::election_config::manifest::{
    approval_payload, file_entries, package, Approval, ConfigurationRevision, Content, Manifest,
    ManifestFormat, Produced,
};
use sequent_core::signing::SignatureAlgorithm;
use serde_json::json;
use signing_pki::{crl, crl_number, crl_with, ec_key, issued, pki_now, Issued, Pki, Spec};
use tempfile::NamedTempFile;
use uuid::Uuid;
use windmill::postgres::configuration_packages;
use windmill::services::import::configuration_package::{admit_document, record};
use windmill::services::import::rejection::problems_of;

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
    let content = Content {
        files: file_entries(&members()).unwrap(),
        ballot_designs: Vec::new(),
        reports: Vec::new(),
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
        revocation_lists: Vec::new(),
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
        &signing_key().sign(&bytes),
        &chain(signing_key()),
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
