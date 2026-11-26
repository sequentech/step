// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The check an election event import runs before reading anything, against
//! the migrated schema: a signed package verified with the tenant's trust,
//! anti-rollback against what the tenant imported, revocation lists it has
//! seen, and an unsigned file where the tenant requires signatures. The
//! certificates come from the signing tests' OpenSSL PKI: an RSA and an EC
//! approver, and an EC package signing key.

#[path = "support/browser_admission_fixture.rs"]
mod browser_admission_fixture;
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
        signer_id: None,
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
    package_signed_with_design_algorithm(
        revision,
        reports,
        key,
        revocation_lists,
        Default::default(),
    )
}

fn package_signed_with_design_algorithm(
    revision: u64,
    reports: Vec<ReportSetting>,
    key: &Issued,
    revocation_lists: Vec<String>,
    ballot_design_digest_algorithm: sequent_core::election_config::design::DesignDigestAlgorithm,
) -> Vec<u8> {
    let content = Content {
        files: file_entries(&members()).unwrap(),
        ballot_design_digest_algorithm,
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
        signing: None,
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
    // Only the setter's transaction may write configuration_signing.
    windmill::postgres::tenant::allow_configuration_signing_change(tx, &id.to_string())
        .await
        .unwrap();
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

/// An unsigned import takes the tenant's import lock too: it can't read the
/// signing policy while another import, or a change of the policy, is under
/// way, so it is judged by the policy in force once it runs.
#[tokio::test]
async fn an_unsigned_import_waits_for_the_import_before_it_and_reads_the_policy_after_it() {
    let pool = schema::pool().await;
    let mut setup = pool.get().await.unwrap();
    let setup_tx = setup.transaction().await.unwrap();
    let tenant = tenant(&setup_tx, "optional").await;
    setup_tx.commit().await.unwrap();

    let mut first = pool.get().await.unwrap();
    let first_tx = first.transaction().await.unwrap();
    admit_document(&first_tx, &tenant, file_of(&signed_package(1)))
        .await
        .unwrap();

    // Under the optional policy an unsigned file would import; it waits.
    let mut second = pool.get().await.unwrap();
    let waiting_tenant = tenant.clone();
    let waiting = tokio::spawn(async move {
        let tx = second.transaction().await.unwrap();
        let plain = zip(&members()).unwrap();
        admit_document(&tx, &waiting_tenant, file_of(&plain))
            .await
            .map(|(_, package)| package.is_some())
            .map_err(|error| ids(&error))
    });
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert!(!waiting.is_finished(), "the unsigned import did not wait");

    // Meanwhile the tenant's policy becomes required, through the setter's
    // own write, in the transaction that holds the lock.
    let required = json!({
        "policy": "required",
        "package_roots": Pki::get().root.pem(),
        "staff_roots": Pki::get().root.pem(),
        "required_approvals": 2,
    });
    windmill::postgres::tenant::write_configuration_signing(&first_tx, &tenant, &required)
        .await
        .unwrap();
    first_tx.commit().await.unwrap();

    let outcome = tokio::time::timeout(Duration::from_secs(10), waiting)
        .await
        .expect("the unsigned import ran once the lock was free")
        .unwrap();
    assert_eq!(outcome, Err(vec!["package.unsigned".to_string()]));
}

/// While an unsigned import runs, the tenant's policy can't change.
#[tokio::test]
async fn the_policy_does_not_change_under_a_running_unsigned_import() {
    use sequent_core::election_config::package_verify::{
        ConfigurationSigning, ConfigurationSigningPolicy,
    };
    use windmill::services::configuration_signing::{save, SaveOutcome};
    let pool = schema::pool().await;
    let mut setup = pool.get().await.unwrap();
    let setup_tx = setup.transaction().await.unwrap();
    let tenant = tenant(&setup_tx, "optional").await;
    setup_tx.commit().await.unwrap();

    let mut importer = pool.get().await.unwrap();
    let import_tx = importer.transaction().await.unwrap();
    let (_, package) = admit_document(&import_tx, &tenant, file_of(&zip(&members()).unwrap()))
        .await
        .unwrap();
    assert!(package.is_none());

    let mut admin = pool.get().await.unwrap();
    let admin_tx = admin.transaction().await.unwrap();
    let strict = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Required,
        package_roots: Pki::get().root.pem(),
        staff_roots: Pki::get().root.pem(),
        required_approvals: 2,
        ..ConfigurationSigning::default()
    };
    let outcome = save(&admin_tx, &tenant, &strict, chrono::Utc::now())
        .await
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::Busy), "{outcome:?}");
    drop(admin_tx);
    import_tx.rollback().await.unwrap();
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
async fn private_configuration_staging_is_withheld_without_affecting_ordinary_documents() {
    use windmill::postgres::document::get_document;
    use windmill::services::documents::{get_document_url, ordinary_document_url_allowed};

    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant_id = tenant(&tx, "optional").await;
    let other_tenant = tenant(&tx, "optional").await;
    let staged = document(&tx, &tenant_id, None, "delivery.zip").await;
    let ordinary = document(&tx, &tenant_id, None, "delivery.zip").await;
    tx.execute(
        "INSERT INTO sequent_backend.configuration_package_staging (tenant_id, document_id)
         VALUES ($1, $2)",
        &[
            &Uuid::parse_str(&tenant_id).unwrap(),
            &Uuid::parse_str(&staged).unwrap(),
        ],
    )
    .await
    .unwrap();

    let ordinary = get_document(&tx, &tenant_id, None, &ordinary)
        .await
        .unwrap()
        .unwrap();
    assert!(ordinary_document_url_allowed(&tx, &tenant_id, &ordinary)
        .await
        .unwrap());
    // No S3 client is contacted for the protected row.
    assert!(get_document_url(&tx, &tenant_id, None, &staged)
        .await
        .unwrap()
        .is_none());
    assert!(get_document_url(&tx, &other_tenant, None, &staged)
        .await
        .unwrap()
        .is_none());
}

async fn staged_row(tx: &Transaction<'_>, tenant: &str, uploaded_by: Option<&str>) -> Uuid {
    let document = Uuid::parse_str(&document(tx, tenant, None, "delivery.zip").await).unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.configuration_package_staging (tenant_id, document_id, uploaded_by)
         VALUES ($1, $2, $3)",
        &[&Uuid::parse_str(tenant).unwrap(), &document, &uploaded_by],
    )
    .await
    .unwrap();
    document
}

#[tokio::test]
async fn server_unwrap_is_bound_to_the_uploader_publishes_once_and_expires() {
    use windmill::services::documents::publish_unwrapped_pointer;
    use windmill::services::import::configuration_package_credentials::{
        lock_staging, sweep_expired, ALREADY_STAGED, NOT_STAGING, NOT_UPLOADER,
    };

    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant_id = tenant(&tx, "optional").await;
    let tenant = Uuid::parse_str(&tenant_id).unwrap();
    let other_tenant = Uuid::parse_str(&self::tenant(&tx, "optional").await).unwrap();
    let staged = staged_row(&tx, &tenant_id, Some("subject-a")).await;
    let legacy = staged_row(&tx, &tenant_id, None).await;
    let refused = |error: anyhow::Error| error.to_string();

    // Actor binding: only the recorded uploader, never another admin or tenant.
    lock_staging(&tx, &tenant, &staged, "subject-a")
        .await
        .unwrap();
    assert_eq!(
        refused(
            lock_staging(&tx, &tenant, &staged, "subject-b")
                .await
                .unwrap_err()
        ),
        NOT_UPLOADER
    );
    assert_eq!(
        refused(
            lock_staging(&tx, &other_tenant, &staged, "subject-a")
                .await
                .unwrap_err()
        ),
        NOT_STAGING
    );
    // Rows staged before uploaders were recorded stay browser-only (V1).
    assert_eq!(
        refused(
            lock_staging(&tx, &tenant, &legacy, "subject-a")
                .await
                .unwrap_err()
        ),
        NOT_UPLOADER
    );

    // The sibling pointer publishes once; never for an uploader-less row.
    let now = chrono::Utc::now();
    let later = now + chrono::Duration::hours(12);
    let sibling = Uuid::new_v4();
    publish_unwrapped_pointer(&tx, tenant, staged, sibling, later)
        .await
        .unwrap();
    assert!(
        publish_unwrapped_pointer(&tx, tenant, staged, Uuid::new_v4(), later)
            .await
            .is_err()
    );
    assert!(
        publish_unwrapped_pointer(&tx, tenant, legacy, Uuid::new_v4(), later)
            .await
            .is_err()
    );
    assert!(
        publish_unwrapped_pointer(&tx, other_tenant, staged, Uuid::new_v4(), later)
            .await
            .is_err()
    );
    // Once unwrapped, even the uploader can't unwrap again.
    assert_eq!(
        refused(
            lock_staging(&tx, &tenant, &staged, "subject-a")
                .await
                .unwrap_err()
        ),
        ALREADY_STAGED
    );

    // TTL cleanup: nothing before expiry; this tenant's entry at expiry.
    assert_eq!(sweep_expired(&tx, &tenant, now).await.unwrap(), 0);
    assert_eq!(sweep_expired(&tx, &other_tenant, later).await.unwrap(), 0);
    assert_eq!(sweep_expired(&tx, &tenant, later).await.unwrap(), 1);
    let row = tx
        .query_one(
            "SELECT unwrapped_id, credentials_expires_at
             FROM sequent_backend.configuration_package_staging WHERE document_id = $1",
            &[&staged],
        )
        .await
        .unwrap();
    // The signed ZIP stays published; only the credentials expired.
    assert_eq!(row.get::<_, Option<Uuid>>(0), Some(sibling));
    assert_eq!(row.get::<_, Option<chrono::DateTime<chrono::Utc>>>(1), None);
    assert_eq!(sweep_expired(&tx, &tenant, later).await.unwrap(), 0);
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

/// Paired producer/receiver contract, using the exact bytes downloaded from
/// Architect's synthetic WebAuthn route test. Run alone against a disposable
/// migrated database with the independently provisioned WINDMILL_WEBAUTHN_*
/// policy environment and ARCHITECT_SYNTHETIC_PACKAGE_FIXTURE_DIR.
/// No package member supplies trust, and no test changes process-wide policy.
#[tokio::test]
#[ignore = "requires the paired synthetic producer fixture and isolated database"]
async fn browser_producer_package_uses_real_admission_and_persisted_chain() {
    use sequent_core::election_config::package_verify::VerificationScheme;
    use windmill::services::browser_package_policy::{BrowserPolicy, SWITCH_VALUE};
    let directory = std::path::PathBuf::from(
        std::env::var_os("ARCHITECT_SYNTHETIC_PACKAGE_FIXTURE_DIR")
            .expect("synthetic fixture directory"),
    );
    let policy: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("policy.json")).unwrap()).unwrap();
    let tenant = policy["tenant"].as_str().unwrap();
    let tenant_uuid = Uuid::parse_str(tenant).unwrap();
    assert_eq!(tenant_uuid.hyphenated().to_string(), tenant);
    assert_eq!(
        std::env::var("WINDMILL_WEBAUTHN_PACKAGES_PREVIEW").unwrap(),
        SWITCH_VALUE
    );
    assert!(matches!(
        BrowserPolicy::load(tenant),
        BrowserPolicy::BrowserOnly(_)
    ));
    let first = std::fs::read(directory.join("package.zip")).unwrap();
    let second = std::fs::read(directory.join("package-r2.zip")).unwrap();
    let expected = std::fs::read(directory.join("expected-importable.zip")).unwrap();
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant_uuid, &format!("synthetic-browser-{tenant}")],
    )
    .await
    .unwrap();

    // Unknown predecessor, unsigned downgrade and certificate downgrade fail
    // before importing any document, even with no tenant signing setting.
    let error = admit_document(&tx, tenant, file_of(&second))
        .await
        .unwrap_err();
    assert!(ids(&error).contains(&"package.webauthn-not-next".into()));
    assert!(admit_document(&tx, tenant, file_of(b"legacy unsigned"))
        .await
        .is_err());
    assert!(admit_document(&tx, tenant, file_of(&signed_package(1)))
        .await
        .is_err());

    // UUID aliases are the same policy and the same tenant lock.
    let alias = tenant_uuid.simple().to_string().to_uppercase();
    let (file, admitted) = admit_document(&tx, &alias, file_of(&first)).await.unwrap();
    let admitted = admitted.unwrap();
    assert!(matches!(
        admitted.scheme,
        VerificationScheme::Browser { .. }
    ));
    assert_eq!(
        admitted.manifest_sha256,
        policy["manifest_sha256"].as_str().unwrap()
    );
    assert_eq!(std::fs::read(file.path()).unwrap(), expected);
    record(&tx, tenant, &Uuid::new_v4().to_string(), &admitted)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // Fresh transaction: history is persisted, not just an in-memory result.
    let tx = client.transaction().await.unwrap();
    let replay = admit_document(&tx, tenant, file_of(&first))
        .await
        .unwrap_err();
    assert!(ids(&replay).contains(&"package.already-imported".into()));
    let (_, admitted_second) = admit_document(&tx, tenant, file_of(&second)).await.unwrap();
    let admitted_second = admitted_second.unwrap();
    let previous = admitted_second
        .manifest
        .configuration
        .previous
        .as_ref()
        .unwrap();
    assert_eq!(previous.revision, 1);
    assert_eq!(previous.manifest_sha256, admitted.manifest_sha256);
    record(&tx, tenant, &Uuid::new_v4().to_string(), &admitted_second)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let tx = client.transaction().await.unwrap();
    assert!(admit_document(&tx, tenant, file_of(&first)).await.is_err());
    let replay = admit_document(&tx, tenant, file_of(&second))
        .await
        .unwrap_err();
    assert!(ids(&replay).contains(&"package.already-imported".into()));
    tx.rollback().await.unwrap();
}

/// Run the real section readmission against an owned migrated database. The
/// child has an independent policy environment and cannot inherit site trust.
#[tokio::test]
async fn managed_section_readmits_its_sealed_package_but_stops_after_replay_or_rotation() {
    const CHILD: &str = "STEP_RECEIVER_READMISSION_FIXTURE";
    const ACTOR: &str = "synthetic-independent-operator";
    let host = std::env::var("HASURA_DB__HOST").expect("disposable test PostgreSQL host");
    assert!(matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1"));
    assert_eq!(std::env::var("HASURA_DB__USER").unwrap(), "test");
    if let Some(directory) = std::env::var_os(CHILD) {
        use sequent_core::election_config::package_contract::Section;
        use sequent_core::election_config::package_verify::VerificationScheme;
        use std::sync::{atomic::AtomicBool, Arc};
        use windmill::services::{
            browser_package_policy::BrowserPolicy,
            database::get_hasura_pool,
            import::{
                configuration_package_apply::PackageTarget,
                configuration_package_target::{derived_event_id, ApplyContext, PlatformTarget},
            },
            managed_webauthn_roster,
        };

        let dir = std::path::PathBuf::from(directory);
        let expected_db = std::fs::read_to_string(dir.join("database")).unwrap();
        assert!(expected_db.starts_with("windmill_schema_"));
        assert_eq!(std::env::var("HASURA_DB__DBNAME").unwrap(), expected_db);
        let tenant = std::fs::read_to_string(dir.join("tenant")).unwrap();
        let roster = std::fs::read_to_string(dir.join("roster.json")).unwrap();
        let bytes = std::fs::read(dir.join("package.zip")).unwrap();
        let config = managed_webauthn_roster::config(&tenant).unwrap().unwrap();
        let pool = get_hasura_pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let refused = admit_document(&tx, &tenant, file_of(&bytes))
            .await
            .unwrap_err();
        assert!(ids(&refused).contains(&"package.webauthn-policy-unusable".into()));
        tx.rollback().await.unwrap();

        let tx = client.transaction().await.unwrap();
        assert!(configuration_packages::try_lock_package_work(&tx, &tenant)
            .await
            .unwrap());
        let active = managed_webauthn_roster::install(
            &tx,
            &config,
            &config.candidate(roster.as_bytes()).unwrap(),
            0,
            ACTOR,
        )
        .await
        .unwrap()
        .unwrap();
        tx.commit().await.unwrap();

        let tx = client.transaction().await.unwrap();
        let unsealed = zip(&members()).unwrap();
        let refused = admit_document(&tx, &tenant, file_of(&unsealed))
            .await
            .unwrap_err();
        assert!(ids(&refused).contains(&"package.webauthn-required".into()));
        let (_, verified) = admit_document(&tx, &tenant, file_of(&bytes)).await.unwrap();
        let verified = verified.unwrap();
        assert!(matches!(
            verified.scheme,
            VerificationScheme::Browser { .. }
        ));
        let policy = BrowserPolicy::load_with_db(&tx, &tenant).await;
        let binding = policy.managed_binding().cloned().unwrap();
        assert_eq!(binding.generation, active.generation);
        tx.rollback().await.unwrap();

        let package_sha256 = sha256_hex(&bytes);
        let event_id = derived_event_id(&tenant, &package_sha256);
        let target = PlatformTarget {
            context: ApplyContext {
                tenant_id: tenant.clone(),
                executed_by: ACTOR.into(),
                audit_actor: None,
                event_import: serde_json::from_value(json!({
                    "tenant_id": tenant, "document_id": Uuid::new_v4().to_string(),
                }))
                .unwrap(),
                package_sha256,
                signed_package: Some(bytes),
                protected_document_id: None,
                managed_roster_binding: Some(binding),
                managed_preview_operator_binding: None,
                package_groups: Default::default(),
                package_trustees: Default::default(),
                demo_login: None,
                package_messaging: None,
                verified: Some(Arc::new(*verified)),
            },
            event_imported_by_this_run: AtomicBool::new(false),
        };
        let section = [Artifact {
            name: "tenant-configuration.json".into(),
            bytes: b"{}".to_vec(),
        }];
        let admission_lock = target.hold_imports().await.unwrap();
        let preview = target
            .preview(Section::TenantConfiguration, &section)
            .await
            .unwrap();
        assert!(preview.blockers.is_empty(), "{:?}", preview.blockers);
        drop(admission_lock);

        let tx = client.transaction().await.unwrap();
        configuration_packages::lock_imports(&tx, &tenant)
            .await
            .unwrap();
        record(
            &tx,
            &tenant,
            &event_id,
            target.context.verified.as_deref().unwrap(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let admission_lock = target.hold_imports().await.unwrap();
        let blocked = target
            .preview(Section::TenantConfiguration, &section)
            .await
            .unwrap();
        assert!(blocked
            .blockers
            .iter()
            .any(|b| b.contains("may not be replayed")));
        // Only this run's verified successful event import may converge.
        target
            .event_imported_by_this_run
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let converged = target
            .preview(Section::TenantConfiguration, &section)
            .await
            .unwrap();
        assert!(converged.blockers.is_empty(), "{:?}", converged.blockers);
        drop(admission_lock);

        let tx = client.transaction().await.unwrap();
        // The previous hold closes its connection asynchronously. Retry
        // briefly instead of racing PostgreSQL's release of that test lock.
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if configuration_packages::try_lock_package_work(&tx, &tenant)
                    .await
                    .unwrap()
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("the previous test hold released its lock");
        // Same officials with a new independently approved byte-exact roster.
        let rotated = config.candidate(format!("{roster}\n").as_bytes()).unwrap();
        assert!(
            managed_webauthn_roster::install(&tx, &config, &rotated, active.generation, ACTOR)
                .await
                .unwrap()
                .is_some()
        );
        tx.commit().await.unwrap();

        let _admission_lock = target.hold_imports().await.unwrap();
        let blocked = target
            .preview(Section::TenantConfiguration, &section)
            .await
            .unwrap();
        assert!(blocked
            .blockers
            .iter()
            .any(|b| b.contains("roster changed")));
        return;
    }

    let mut client = schema::pool().await.get().await.unwrap();
    let database: String = client
        .query_one("SELECT current_database()", &[])
        .await
        .unwrap()
        .get(0);
    assert!(database.starts_with("windmill_schema_"));
    let tenant = Uuid::new_v4();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("synthetic-managed-{tenant}")],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let directory = tempfile::tempdir().unwrap();
    let fixture = browser_admission_fixture::fixture(&tenant.to_string());
    std::fs::write(directory.path().join("database"), &database).unwrap();
    std::fs::write(directory.path().join("tenant"), tenant.to_string()).unwrap();
    std::fs::write(directory.path().join("roster.json"), fixture.roster).unwrap();
    std::fs::write(directory.path().join("package.zip"), fixture.package).unwrap();
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .kill_on_drop(true)
        .env_clear()
        .args([
            "--exact",
            "managed_section_readmits_its_sealed_package_but_stops_after_replay_or_rotation",
            "--nocapture",
        ])
        .env(CHILD, directory.path())
        .env("HASURA_DB__DBNAME", database)
        .env("WINDMILL_WEBAUTHN_PACKAGES", "portal-managed")
        .env("WINDMILL_WEBAUTHN_PACKAGES_TENANT_ID", tenant.to_string())
        .env(
            "WINDMILL_WEBAUTHN_PACKAGES_EXPECTED_RP_ID",
            browser_admission_fixture::RP,
        )
        .env(
            "WINDMILL_WEBAUTHN_PACKAGES_EXPECTED_ORIGIN",
            browser_admission_fixture::ORIGIN,
        );
    for key in [
        "HASURA_DB__HOST",
        "HASURA_DB__PORT",
        "HASURA_DB__USER",
        "HASURA_DB__PASSWORD",
        "KEYCLOAK_DB__HOST",
        "KEYCLOAK_DB__PORT",
        "KEYCLOAK_DB__USER",
        "KEYCLOAK_DB__PASSWORD",
        "KEYCLOAK_DB__DBNAME",
        "LOW_SQL_LIMIT",
        "DEFAULT_SQL_LIMIT",
        "DEFAULT_SQL_BATCH_SIZE",
        "LLVM_PROFILE_FILE",
        "LD_LIBRARY_PATH",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .expect("isolated section readmission timed out")
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success()
            && stdout.contains("running 1 test")
            && stdout.contains("test result: ok. 1 passed; 0 failed;"),
        "isolated readmission did not run exactly one successful test:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn v2_design_algorithm_survives_verified_import_provenance() {
    use sequent_core::election_config::design::DesignDigestAlgorithm;

    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx, "required").await;
    let event = Uuid::new_v4().to_string();
    let bytes = package_signed_with_design_algorithm(
        1,
        Vec::new(),
        signing_key(),
        Vec::new(),
        DesignDigestAlgorithm::V2,
    );
    let (file, admitted) = admit_document(&tx, &tenant, file_of(&bytes)).await.unwrap();
    assert_eq!(std::fs::read(file.path()).unwrap(), members()[0].bytes);
    let admitted = admitted.unwrap();
    assert_eq!(
        admitted.manifest.content.ballot_design_digest_algorithm,
        DesignDigestAlgorithm::V2
    );
    record(&tx, &tenant, &event, &admitted).await.unwrap();
    let (stored, hash) = configuration_packages::manifest_of_event(&tx, &tenant, &event)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored, admitted.manifest);
    assert_eq!(hash, admitted.manifest_sha256);
    tx.rollback().await.unwrap();
}
