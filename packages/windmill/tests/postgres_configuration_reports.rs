// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The reports of an event imported from a signed configuration package,
//! against the migrated schema: the copies and formats the Reports step set
//! reach the generation, and a report drawn with a template the package did
//! not approve is refused whatever its formats and copies, before anything
//! is written.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing_pki.rs"]
#[allow(dead_code)]
mod signing_pki;

use std::collections::{BTreeMap, HashMap};
use std::io::Write;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use deadpool_postgres::Transaction;
use sequent_core::election_config::archive::{zip, Artifact};
use sequent_core::election_config::manifest::{
    approval_payload, file_entries, package, report_manifest, sha256_hex, Approval,
    ConfigurationRevision, ConfigurationStamp, Content, Manifest, ManifestFormat, Produced,
    ReportSetting,
};
use sequent_core::election_config::{EReportEncryption, Report, ReportFormat};
use sequent_core::signing::SignatureAlgorithm;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::{
    DocumentAnnotations, ElectionEvent, ReportManifestFile, TallySession,
};
use sequent_core::types::templates::{
    ChannelSelection, EmailConfig, PrintToPdfOptionsLocal, ReportOptions, SendTemplateBody,
    SmsConfig,
};
use serde_json::json;
use signing_pki::{ec_key, issued, Issued, Pki, Spec};
use tempfile::NamedTempFile;
use uuid::Uuid;
use windmill::postgres::document::{get_document, insert_document, set_document_annotations};
use windmill::postgres::reports::{
    get_report_by_id, get_report_copies, insert_reports, ReportType,
};
use windmill::services::ceremonies::velvet_tally::run_velvet_tally;
use windmill::services::consolidation::package_manifest::{
    link_returns, package_manifest, StoredReturns,
};
use windmill::services::import::configuration_package::{admit_document, record};
use windmill::services::reports::activity_log::ActivityLogsTemplate;
use windmill::services::reports::generation::{link_report_manifest, manifest_of_document};
use windmill::services::reports::participation::ParticipationReportTemplate;
use windmill::services::reports::report_variables::configuration_stamp_without_template;
use windmill::services::reports::template_renderer::{
    GenerateReportMode, ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use windmill::services::signing::actions::reports::held_annotations;

const SIGNING_KEY_SERIAL: u32 = 4101;
const APPROVED_TEMPLATE: &str = "<h1>Approved design</h1>";
const CHANGED_TEMPLATE: &str = "<h1>Edited after the import</h1>";
const REFUSAL: &str = "not the one the signed configuration approved";

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
    vec![Artifact {
        name: "official_election_setup.zip".to_string(),
        bytes: zip(&[Artifact {
            name: "export_election_event-1.json".to_string(),
            bytes: b"{}\n".to_vec(),
        }])
        .unwrap(),
    }]
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

fn signed_package(reports: Vec<ReportSetting>) -> Vec<u8> {
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
            revision: 4,
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

async fn tenant(tx: &Transaction<'_>) -> String {
    let id = Uuid::new_v4();
    let root = Pki::get().root.pem();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug, settings) VALUES ($1, $2, $3)",
        &[
            &id,
            &format!("tenant-{id}"),
            &json!({"configuration_signing": {
                "policy": "required",
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

/// A tenant's event imported from a package that sets `settings`.
async fn imported_event(tx: &Transaction<'_>, settings: Vec<ReportSetting>) -> (String, String) {
    let tenant = tenant(tx).await;
    let event = event(tx, &tenant).await;
    let (_, package) = admit_document(tx, &tenant, file_of(&signed_package(settings)))
        .await
        .unwrap();
    record(tx, &tenant, &event, &package.expect("a verified package"))
        .await
        .unwrap();
    (tenant, event)
}

fn setting(
    report_type: ReportType,
    formats: Vec<ReportFormat>,
    copies: u32,
    template: &str,
) -> ReportSetting {
    ReportSetting {
        report_type: report_type.to_string(),
        formats,
        copies,
        template: Some("design".to_string()),
        template_sha256: Some(sha256_hex(template.as_bytes())),
    }
}

/// The organization's template `alias`, whose design is `document`. It sets
/// every option, so drawing with it reads no default.
async fn template(tx: &Transaction<'_>, tenant: &str, alias: &str, document: &str) {
    let body = SendTemplateBody {
        audience_selection: None,
        audience_voter_ids: None,
        communication_method: None,
        schedule_now: None,
        schedule_date: None,
        email: Some(EmailConfig::default()),
        sms: Some(SmsConfig::default()),
        whatsapp: None,
        viber: None,
        messenger: None,
        channel_selection: ChannelSelection::default(),
        send_id: None,
        document: Some(document.to_string()),
        name: None,
        alias: Some(alias.to_string()),
        pdf_options: Some(PrintToPdfOptionsLocal::default()),
        report_options: Some(ReportOptions::default()),
        secret_attribute_names: Vec::new(),
        schedule_local: None,
        schedule_timezone: None,
    };
    tx.execute(
        "INSERT INTO sequent_backend.template
             (tenant_id, alias, template, created_by, communication_method, type)
         VALUES ($1, $2, $3, 'admin', 'DOCUMENT', 'REPORT')",
        &[
            &Uuid::parse_str(tenant).unwrap(),
            &alias,
            &serde_json::to_value(body).unwrap(),
        ],
    )
    .await
    .unwrap();
}

fn report(
    tenant: &str,
    event: &str,
    election: Option<&str>,
    report_type: ReportType,
    copies: Option<u32>,
    formats: Option<Vec<ReportFormat>>,
) -> Report {
    Report {
        id: Uuid::new_v4().to_string(),
        election_event_id: event.to_string(),
        tenant_id: tenant.to_string(),
        election_id: election.map(str::to_string),
        report_type: report_type.to_string(),
        template_alias: Some("design".to_string()),
        encryption_policy: EReportEncryption::Unencrypted,
        cron_config: None,
        created_at: chrono::Utc::now(),
        permission_label: None,
        copies,
        output_formats: formats,
    }
}

fn origins(tenant: &str, event: &str) -> ReportOrigins {
    ReportOrigins {
        tenant_id: tenant.to_string(),
        election_event_id: event.to_string(),
        election_id: None,
        template_alias: Some("design".to_string()),
        voter_id: None,
        report_origin: ReportOriginatedFrom::ReportsTab,
        executer_username: None,
        tally_session_id: None,
    }
}

async fn documents_of(tx: &Transaction<'_>, event: &str) -> i64 {
    tx.query_one(
        "SELECT count(*) FROM sequent_backend.document WHERE election_event_id = $1",
        &[&Uuid::parse_str(event).unwrap()],
    )
    .await
    .unwrap()
    .get(0)
}

#[tokio::test]
async fn the_copies_and_formats_of_the_reports_step_reach_the_generation() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx).await;
    let event = event(&tx, &tenant).await;

    let logs = report(
        &tenant,
        &event,
        None,
        ReportType::ACTIVITY_LOGS,
        Some(2),
        Some(vec![
            ReportFormat::Csv,
            ReportFormat::Sql,
            ReportFormat::Pdf,
        ]),
    );
    let returns = report(
        &tenant,
        &event,
        None,
        ReportType::ELECTORAL_RESULTS,
        Some(7),
        Some(vec![ReportFormat::Pdf, ReportFormat::Xml]),
    );
    let plain = report(
        &tenant,
        &event,
        None,
        ReportType::PARTICIPATION_REPORT,
        None,
        None,
    );
    insert_reports(
        &tx,
        &tenant,
        &event,
        &[logs.clone(), returns.clone(), plain.clone()],
    )
    .await
    .unwrap();

    let read = |id: String| {
        let tx = &tx;
        let tenant = tenant.clone();
        async move {
            get_report_by_id(tx, &tenant, &id)
                .await
                .unwrap()
                .expect("the report")
        }
    };

    let logs = read(logs.id).await;
    assert_eq!(logs.copy_count(), 2);
    assert_eq!(
        ReportType::ACTIVITY_LOGS.generation_formats(logs.output_formats.as_deref()),
        Ok(vec![
            ReportFormat::Csv,
            ReportFormat::Sql,
            ReportFormat::Pdf
        ])
    );

    // The returns' XML is the transmission package's.
    let returns = read(returns.id).await;
    assert_eq!(returns.copy_count(), 7);
    assert_eq!(
        ReportType::ELECTORAL_RESULTS.generation_formats(returns.output_formats.as_deref()),
        Ok(vec![ReportFormat::Pdf])
    );

    // A report that sets neither is generated as before: once, in PDF.
    let plain = read(plain.id).await;
    assert_eq!(plain.copy_count(), 1);
    assert_eq!(
        ReportType::PARTICIPATION_REPORT.generation_formats(plain.output_formats.as_deref()),
        Ok(vec![ReportFormat::Pdf])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn the_tally_prints_the_copies_the_event_sets_for_its_reports() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx).await;
    let event = event(&tx, &tenant).await;
    let other_event = self::event(&tx, &tenant).await;
    let copies = |report_type: ReportType| {
        let tx = &tx;
        let (tenant, event) = (tenant.clone(), event.clone());
        async move {
            get_report_copies(tx, &tenant, &event, &report_type)
                .await
                .unwrap()
        }
    };

    assert_eq!(copies(ReportType::ELECTORAL_RESULTS).await, None);

    let first_election = Uuid::new_v4().to_string();
    let second_election = Uuid::new_v4().to_string();
    insert_reports(
        &tx,
        &tenant,
        &event,
        &[
            report(
                &tenant,
                &event,
                Some(&first_election),
                ReportType::ELECTORAL_RESULTS,
                Some(3),
                None,
            ),
            report(
                &tenant,
                &event,
                Some(&second_election),
                ReportType::ELECTORAL_RESULTS,
                Some(5),
                None,
            ),
            report(
                &tenant,
                &event,
                None,
                ReportType::INITIALIZATION_REPORT,
                None,
                None,
            ),
        ],
    )
    .await
    .unwrap();
    insert_reports(
        &tx,
        &tenant,
        &other_event,
        &[report(
            &tenant,
            &other_event,
            None,
            ReportType::ELECTORAL_RESULTS,
            Some(9),
            None,
        )],
    )
    .await
    .unwrap();

    // Without a setting of the event's own: the most an election asks for.
    assert_eq!(copies(ReportType::ELECTORAL_RESULTS).await, Some(5));
    assert_eq!(copies(ReportType::INITIALIZATION_REPORT).await, None);

    insert_reports(
        &tx,
        &tenant,
        &event,
        &[report(
            &tenant,
            &event,
            None,
            ReportType::ELECTORAL_RESULTS,
            Some(7),
            None,
        )],
    )
    .await
    .unwrap();
    assert_eq!(copies(ReportType::ELECTORAL_RESULTS).await, Some(7));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_report_drawn_with_another_template_is_refused_in_every_format_and_copy() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let mut keycloak_client = pool.get().await.unwrap();
    let keycloak_tx = keycloak_client.transaction().await.unwrap();

    let formats = vec![ReportFormat::Pdf, ReportFormat::Csv, ReportFormat::Sql];
    let (tenant, event) = imported_event(
        &tx,
        vec![setting(
            ReportType::ACTIVITY_LOGS,
            formats.clone(),
            3,
            APPROVED_TEMPLATE,
        )],
    )
    .await;
    template(&tx, &tenant, "design", CHANGED_TEMPLATE).await;
    let row = report(
        &tenant,
        &event,
        None,
        ReportType::ACTIVITY_LOGS,
        Some(3),
        Some(formats.clone()),
    );
    insert_reports(&tx, &tenant, &event, &[row.clone()])
        .await
        .unwrap();

    for asked in [
        formats.clone(),
        vec![ReportFormat::Csv],
        vec![ReportFormat::Sql],
        vec![ReportFormat::Pdf],
    ] {
        let refused = ActivityLogsTemplate::in_formats(origins(&tenant, &event), asked.clone())
            .execute_report_outcome(
                &Uuid::new_v4().to_string(),
                &tenant,
                &event,
                false,
                Vec::new(),
                GenerateReportMode::REAL,
                Some(row.clone()),
                &tx,
                &keycloak_tx,
                None,
                false,
            )
            .await
            .unwrap_err();
        assert!(
            refused.to_string().contains(REFUSAL),
            "{asked:?}: {refused}"
        );
    }
    assert_eq!(documents_of(&tx, &event).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_format_the_report_cannot_be_generated_in_is_refused_before_it_is_drawn() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let mut keycloak_client = pool.get().await.unwrap();
    let keycloak_tx = keycloak_client.transaction().await.unwrap();

    let (tenant, event) = imported_event(
        &tx,
        vec![setting(
            ReportType::PARTICIPATION_REPORT,
            vec![ReportFormat::Pdf],
            1,
            APPROVED_TEMPLATE,
        )],
    )
    .await;
    template(&tx, &tenant, "design", APPROVED_TEMPLATE).await;
    let row = report(
        &tenant,
        &event,
        None,
        ReportType::PARTICIPATION_REPORT,
        Some(2),
        Some(vec![ReportFormat::Pdf, ReportFormat::Csv]),
    );
    insert_reports(&tx, &tenant, &event, &[row.clone()])
        .await
        .unwrap();

    let refused = ParticipationReportTemplate::new(origins(&tenant, &event))
        .execute_report_outcome(
            &Uuid::new_v4().to_string(),
            &tenant,
            &event,
            false,
            Vec::new(),
            GenerateReportMode::REAL,
            Some(row),
            &tx,
            &keycloak_tx,
            None,
            false,
        )
        .await
        .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "the PARTICIPATION_REPORT report cannot be generated in csv: it supports pdf"
    );
    assert_eq!(documents_of(&tx, &event).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_tally_report_held_for_signatures_names_the_configuration_its_release_is_stamped_with() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx).await;
    let event = event(&tx, &tenant).await;
    let (tenant_id, event_id) = (
        Uuid::parse_str(&tenant).unwrap(),
        Uuid::parse_str(&event).unwrap(),
    );
    let base = b"%PDF-1.7 with its signature page";
    let held = insert_document(
        &tx,
        &tenant,
        Some(event.clone()),
        "report-to-sign.pdf",
        "application/pdf",
        base.len() as i64,
        false,
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        manifest_of_document(held.annotations.as_ref()).unwrap(),
        None
    );
    let stamp = ConfigurationStamp {
        external_id: "ov-2028".to_string(),
        revision: 3,
        manifest_sha256: "ab".repeat(32),
        template_sha256: "cd".repeat(32),
    };
    let folder = report_manifest("ELECTORAL_RESULTS", &stamp, &[]);

    let annotations = held_annotations(&folder, "report-to-sign.pdf", base).unwrap();
    let held_id = Uuid::parse_str(&held.id).unwrap();
    assert!(
        set_document_annotations(&tx, tenant_id, event_id, held_id, &annotations)
            .await
            .unwrap()
    );
    assert!(
        !set_document_annotations(&tx, tenant_id, event_id, Uuid::new_v4(), &annotations)
            .await
            .unwrap()
    );

    let stored = get_document(&tx, &tenant, Some(event.clone()), &held.id)
        .await
        .unwrap()
        .unwrap();
    let manifest = manifest_of_document(stored.annotations.as_ref())
        .unwrap()
        .unwrap();
    assert_eq!(manifest.report_type, "ELECTORAL_RESULTS");
    assert_eq!(manifest.configuration, stamp);
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].path, "report-to-sign.pdf");
    assert_eq!(manifest.files[0].sha256, sha256_hex(base));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn the_tally_refuses_a_template_the_configuration_did_not_approve() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = imported_event(
        &tx,
        vec![
            setting(
                ReportType::ELECTORAL_RESULTS,
                vec![ReportFormat::Pdf, ReportFormat::Xml],
                7,
                APPROVED_TEMPLATE,
            ),
            setting(
                ReportType::INITIALIZATION_REPORT,
                vec![ReportFormat::Pdf],
                2,
                APPROVED_TEMPLATE,
            ),
        ],
    )
    .await;
    insert_reports(
        &tx,
        &tenant,
        &event,
        &[report(
            &tenant,
            &event,
            None,
            ReportType::ELECTORAL_RESULTS,
            Some(7),
            Some(vec![ReportFormat::Pdf, ReportFormat::Xml]),
        )],
    )
    .await
    .unwrap();

    let election_event: ElectionEvent = serde_json::from_value(json!({
        "id": event,
        "tenant_id": tenant,
        "is_archived": false,
        "encryption_protocol": "RSA256",
    }))
    .unwrap();
    let tally_session: TallySession = serde_json::from_value(json!({
        "id": Uuid::new_v4().to_string(),
        "tenant_id": tenant,
        "election_event_id": event,
        "is_execution_completed": false,
        "keys_ceremony_id": Uuid::new_v4().to_string(),
        "threshold": 2,
    }))
    .unwrap();

    for tally_type in [
        TallyType::ELECTORAL_RESULTS,
        TallyType::INITIALIZATION_REPORT,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let refused = run_velvet_tally(
            directory.path().to_path_buf(),
            &Vec::new(),
            &Vec::new(),
            &Vec::new(),
            Some(CHANGED_TEMPLATE.to_string()),
            "<main>{{{rendered_user_template}}}</main>".to_string(),
            None,
            &Vec::new(),
            &tx,
            &election_event,
            &tally_session,
            tally_type,
            HashMap::new(),
        )
        .await
        .err()
        .expect("a refusal");
        assert!(refused.to_string().contains(REFUSAL), "{refused}");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
    assert_eq!(documents_of(&tx, &event).await, 0);
    tx.rollback().await.unwrap();
}

const RETURNS_XML: &[u8] = b"<EML><EMLHeader></EMLHeader></EML>";
const RETURNS_XZ: &[u8] = b"the returns, compressed";

fn stored_returns() -> StoredReturns<'static> {
    StoredReturns {
        transaction_id: "0000001234",
        eml: RETURNS_XML,
        compressed: RETURNS_XZ,
    }
}

/// The archive of one server's package.
fn servers_archive() -> Vec<u8> {
    let station = zip(&[Artifact {
        name: "er_901.exz".to_string(),
        bytes: b"the returns, encrypted".to_vec(),
    }])
    .unwrap();
    zip(&[Artifact {
        name: "ccs-1/er_901.zip".to_string(),
        bytes: station,
    }])
    .unwrap()
}

/// The three documents of a transmission package, without their files.
async fn package_documents(tx: &Transaction<'_>, tenant: &str, event: &str) -> [String; 3] {
    let mut ids = Vec::new();
    for (name, size) in [
        ("er_0000001234.xml", RETURNS_XML.len()),
        ("er_0000001234.xz", RETURNS_XZ.len()),
        ("all_servers.zip", servers_archive().len()),
    ] {
        let document = insert_document(
            tx,
            tenant,
            Some(event.to_string()),
            name,
            "application/octet-stream",
            size as i64,
            false,
            None,
        )
        .await
        .unwrap();
        ids.push(document.id);
    }
    ids.try_into().unwrap()
}

#[tokio::test]
async fn the_returns_xml_of_an_imported_event_is_stored_with_its_packages_hash_manifest() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = imported_event(&tx, Vec::new()).await;
    let [xml, xz, archive] = package_documents(&tx, &tenant, &event).await;

    let stamp = configuration_stamp_without_template(&tx, &tenant, &event)
        .await
        .unwrap()
        .expect("the stamp of an imported event");
    let written = package_manifest(Some(&stamp), &stored_returns(), &servers_archive())
        .unwrap()
        .expect("a manifest");
    assert_eq!(written.manifest.report_type, "ELECTORAL_RESULTS");
    assert_eq!(written.manifest.configuration, stamp);

    let mut annotations = DocumentAnnotations::default();
    let file = ReportManifestFile {
        document_id: Uuid::new_v4().to_string(),
        sha256: written.sha256.clone(),
    };
    link_report_manifest(&mut annotations, &written, file.clone()).unwrap();
    assert!(set_document_annotations(
        &tx,
        Uuid::parse_str(&tenant).unwrap(),
        Uuid::parse_str(&event).unwrap(),
        Uuid::parse_str(&archive).unwrap(),
        &annotations,
    )
    .await
    .unwrap());
    let stored_archive = get_document(&tx, &tenant, Some(event.clone()), &archive)
        .await
        .unwrap()
        .unwrap();

    link_returns(&tx, &tenant, &event, &stored_archive, [&xml, &xz])
        .await
        .unwrap();

    for id in [&xml, &xz, &archive] {
        let document = get_document(&tx, &tenant, Some(event.clone()), id)
            .await
            .unwrap()
            .unwrap();
        let linked: DocumentAnnotations =
            serde_json::from_value(document.annotations.clone().unwrap()).unwrap();
        assert_eq!(linked.report_manifest_file, Some(file.clone()));
        let manifest = manifest_of_document(document.annotations.as_ref())
            .unwrap()
            .unwrap();
        assert_eq!(manifest, written.manifest);
        let listed = manifest
            .files
            .iter()
            .find(|entry| entry.path == "er_0000001234.xml")
            .expect("the returns' XML");
        assert_eq!(listed.sha256, sha256_hex(RETURNS_XML));
    }

    let missing = Uuid::new_v4().to_string();
    let refused = link_returns(&tx, &tenant, &event, &stored_archive, [&xml, &missing])
        .await
        .unwrap_err();
    assert!(refused.to_string().contains(&missing), "{refused}");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn the_transmission_package_of_any_other_event_is_stored_as_it_was() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = tenant(&tx).await;
    let event = event(&tx, &tenant).await;
    let [xml, xz, archive] = package_documents(&tx, &tenant, &event).await;

    let stamp = configuration_stamp_without_template(&tx, &tenant, &event)
        .await
        .unwrap();
    assert_eq!(stamp, None);
    assert_eq!(
        package_manifest(stamp.as_ref(), &stored_returns(), &servers_archive()).unwrap(),
        None
    );

    let stored_archive = get_document(&tx, &tenant, Some(event.clone()), &archive)
        .await
        .unwrap()
        .unwrap();
    link_returns(&tx, &tenant, &event, &stored_archive, [&xml, &xz])
        .await
        .unwrap();

    for id in [&xml, &xz, &archive] {
        let document = get_document(&tx, &tenant, Some(event.clone()), id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(document.annotations, None);
    }
    assert_eq!(documents_of(&tx, &event).await, 3);
    tx.rollback().await.unwrap();
}
