// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::services::browser_package_policy::admit_with_policy;
use crate::services::managed_webauthn_roster::Binding as ReceiverBinding;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::digest::{digest, SHA256};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
use sequent_core::election_config::{
    archive::zip,
    manifest::MANIFEST_MEMBER,
    manifest::{
        file_entries, ConfigurationRevision, Content, Manifest, PreviousRevision, Produced,
    },
    package_verify::Admission,
    webauthn_package::{ENVELOPE_MEMBER, ENVELOPE_V1},
    webauthn_proof::{
        package_seal_sha256, Action, Binding, Bundle, PinnedRoster, Proof, Role, Roster,
        SealedManifest, BUNDLE_FORMAT, ROSTER_FORMAT,
    },
};
const TENANT: &str = "00000000-0000-4000-8000-00000000000a";
const RP: &str = "architect.ems.example";
const ORIGIN: &str = "https://architect.ems.example";
const CONFIGURATION: &str = "00000000-0000-0000-0000-000000000007";
const EXTERNAL_ID: &str = "ov-2028";
const REVISION: u64 = 3;
const PREVIOUS: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

struct Official {
    subject: &'static str,
    role: Role,
    pair: EcdsaKeyPair,
}

fn official(subject: &'static str, role: Role) -> Official {
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
    let pair =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng).unwrap();
    Official {
        subject,
        role,
        pair,
    }
}

impl Official {
    fn key(&self) -> String {
        format!("key-{}", self.subject)
    }
    fn credential(&self) -> String {
        b64(format!("credential-{}", self.subject).as_bytes())
    }
    fn public(&self) -> String {
        b64(self.pair.public_key().as_ref())
    }

    /// An honest assertion over `binding`, as a browser and key would make.
    fn assert(&self, binding: Binding) -> Proof {
        let nonce = [9u8; 32];
        let challenge = binding.challenge(&nonce);
        let client_data = format!(
            "{{\"type\":\"webauthn.get\",\"challenge\":\"{}\",\"origin\":\"{ORIGIN}\"}}",
            b64(&challenge)
        );
        let mut authenticator_data = digest(&SHA256, RP.as_bytes()).as_ref().to_vec();
        authenticator_data.push(0x05);
        authenticator_data.extend_from_slice(&1u32.to_be_bytes());
        let mut message = authenticator_data.clone();
        message.extend_from_slice(digest(&SHA256, client_data.as_bytes()).as_ref());
        let signature = self.pair.sign(&SystemRandom::new(), &message).unwrap();
        Proof {
            binding,
            nonce: b64(&nonce),
            credential_id: self.credential(),
            authenticator_data: b64(&authenticator_data),
            client_data_json: b64(client_data.as_bytes()),
            signature: b64(signature.as_ref()),
        }
    }

    fn binding(&self, action: Action, content: &str) -> Binding {
        Binding {
            rp_id: RP.to_string(),
            tenant: TENANT.to_string(),
            subject: self.subject.to_string(),
            action,
            configuration: CONFIGURATION.to_string(),
            revision: REVISION as i64,
            content_sha256: content.to_string(),
            signer: self.key(),
        }
    }
}

struct World {
    a: Official,
    signer: Official,
}

fn world() -> World {
    World {
        a: official("u-a", Role::Approver),
        signer: official("u-signer", Role::FinalSigner),
    }
}

fn roster_json(tenant: &str, keys: Vec<serde_json::Value>) -> String {
    serde_json::json!({
        "format": ROSTER_FORMAT,
        "rp_id": RP,
        "origins": [ORIGIN],
        "tenant": tenant,
        "keys": keys,
    })
    .to_string()
}

fn entry(official: &Official) -> serde_json::Value {
    serde_json::json!({
        "key": official.key(),
        "credential_id": official.credential(),
        "subject": official.subject,
        "role": official.role,
        "algorithm": "es256",
        "public_key": official.public(),
        "record_sha256": "b".repeat(64),
    })
}

impl World {
    fn roster_text(&self) -> String {
        roster_json(
            TENANT,
            [&self.a, &self.signer].iter().map(|o| entry(o)).collect(),
        )
    }

    /// The verifier's own provisioning: this roster, pinned, for this
    /// tenant, needing `required` approvals.
    fn pinned(&self, required: usize) -> PinnedRoster {
        let text = self.roster_text();
        PinnedRoster::load(&text, &Roster::sha256(&text), TENANT, required).unwrap()
    }
}

fn plain_manifest(members: &[Artifact], algorithm: DesignDigestAlgorithm) -> Manifest {
    let content = Content {
        files: file_entries(members).unwrap(),
        ballot_design_digest_algorithm: algorithm,
        ballot_designs: vec![],
        reports: vec![],
    };
    Manifest {
        format: ManifestFormat::V1,
        configuration: ConfigurationRevision {
            external_id: EXTERNAL_ID.to_string(),
            name: "Overseas Voting 2028".to_string(),
            revision: REVISION,
            previous: Some(PreviousRevision {
                revision: 2,
                manifest_sha256: PREVIOUS.to_string(),
            }),
        },
        content_sha256: content.sha256().unwrap(),
        content,
        approvals: vec![],
        produced: Produced {
            at: "2026-01-01T00:00:00Z".to_string(),
            custodian: "Ada Lovelace".to_string(),
            key_label: "security-keys".to_string(),
            producer: BTreeMap::new(),
        },
        revocation_lists: vec![],
        signing: None,
    }
}

/// What an honest producer writes: `approvers`' approvals of the content,
/// sealed by `signer` over the whole manifest for `TENANT`.
struct Built {
    members: Vec<Artifact>,
    manifest: Vec<u8>,
    envelope: serde_json::Value,
}

fn build_with_manifest(
    w: &World,
    approvers: &[&Official],
    signer: &Official,
    members: Vec<Artifact>,
    manifest: Manifest,
) -> Built {
    use sha2::Digest;
    let manifest_bytes = manifest.to_bytes().unwrap();
    let manifest_sha256 = format!("{:x}", sha2::Sha256::digest(&manifest_bytes));
    let mut bundle = Bundle {
        format: BUNDLE_FORMAT.to_string(),
        configuration: CONFIGURATION.to_string(),
        external_id: EXTERNAL_ID.to_string(),
        revision: REVISION as i64,
        content_sha256: manifest.content_sha256.clone(),
        approvals: approvers
            .iter()
            .map(|o| o.assert(o.binding(Action::ApproveRevision, &manifest.content_sha256)))
            .collect(),
        final_signature: w.a.assert(w.a.binding(Action::ApproveRevision, "x")),
    };
    let seal = package_seal_sha256(
        &bundle,
        TENANT,
        RP,
        SealedManifest {
            envelope: ENVELOPE_V1,
            external_id: EXTERNAL_ID,
            revision: REVISION,
            previous: Some((2, PREVIOUS)),
            content_sha256: &manifest.content_sha256,
            manifest_sha256: &manifest_sha256,
        },
    );
    bundle.final_signature = signer.assert(signer.binding(Action::SignPackage, &seal));
    Built {
        members,
        envelope: serde_json::json!({
            "format": ENVELOPE_V1,
            "tenant_id": TENANT,
            "manifest_sha256": manifest_sha256,
            "bundle": bundle,
        }),
        manifest: manifest_bytes,
    }
}

const NEW: &str = "sbei-bangkok-1";
const OLD: &str = "sbei-bangkok-2";
const NEW_PW: &str = "Synthetic-workbook-only!42";
const OLD_PW: &str = "Synthetic-existing-only!84";

fn signed_fixture(algorithm: DesignDigestAlgorithm) -> (Vec<u8>, BrowserPolicy) {
    let world = world();
    let pinned = world.pinned(1);
    let policy = BrowserPolicy::ManagedOnly(
        pinned.clone(),
        ReceiverBinding {
            sha256: pinned.sha256().into(),
            generation: 1,
            required_approvals: pinned.required(),
        },
    );
    use sequent_core::election_config::{
        demo_credentials::extract_demo_workbook,
        package_contract::ADMIN_USERS_MEMBER,
        paths::Cell,
        sheet::{Sheet, Workbook},
        xlsx::read_xlsx,
        xlsx_write::write_xlsx,
    };
    let approved = default_demo_login_roster();
    let header = [
        "username",
        "group_name",
        "email",
        "trustee",
        "first_name",
        "last_name",
    ];
    let mut grid: Vec<Vec<Cell>> = vec![header
        .into_iter()
        .chain(["password"])
        .map(Cell::text)
        .collect()];
    for username in [NEW, OLD, "trustee1"] {
        let principal = approved.iter().find(|p| p.username == username).unwrap();
        grid.push(
            [
                username,
                &principal.group_name,
                principal.email.as_deref().unwrap_or(""),
                principal.trustee.as_deref().unwrap_or(""),
                "Synthetic",
                "Workbook",
                NEW_PW,
            ]
            .into_iter()
            .map(Cell::text)
            .collect(),
        );
    }
    let trustees = [
        vec![Cell::text("name"), Cell::text("password")],
        vec![Cell::text("trustee1"), Cell::text(NEW_PW)],
    ];
    let raw = write_xlsx(
        &Workbook::new(vec![
            Sheet::from_grid("AdminUsers", &grid).unwrap(),
            Sheet::from_grid("Trustees", &trustees).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let extracted = extract_demo_workbook(&raw, TENANT, &approved).unwrap();
    let sanitized = read_xlsx(&extracted.workbook).unwrap();
    for sheet in sanitized.sheets() {
        assert!(!sheet
            .headers
            .iter()
            .any(|name| name.eq_ignore_ascii_case("password")));
        assert!(sheet.rows.iter().all(|row| row
            .cells
            .iter()
            .all(|(_, cell)| cell.as_str() != Some(NEW_PW))));
    }
    let roster = sanitized
        .sheets()
        .iter()
        .find(|sheet| sheet.name == "AdminUsers")
        .unwrap();
    let mut csv = csv::Writer::from_writer(Vec::new());
    csv.write_record(header).unwrap();
    for row in &roster.rows {
        csv.write_record(header.map(|key| row.text(key).unwrap_or("")))
            .unwrap();
    }
    let verifiers = serde_json::to_vec(&extracted.manifest.unwrap()).unwrap();
    assert!(!String::from_utf8_lossy(&verifiers).contains(NEW_PW));
    let members = vec![
        Artifact {
            name: DEMO_LOGIN_VERIFIERS_MEMBER.into(),
            bytes: verifiers,
        },
        Artifact {
            name: ADMIN_USERS_MEMBER.into(),
            bytes: csv.into_inner().unwrap(),
        },
    ];
    let manifest = plain_manifest(&members, algorithm);
    let mut built = build_with_manifest(&world, &[&world.a], &world.signer, members, manifest);
    built.members.push(Artifact {
        name: MANIFEST_MEMBER.into(),
        bytes: built.manifest,
    });
    built.members.push(Artifact {
        name: ENVELOPE_MEMBER.into(),
        bytes: serde_json::to_vec(&built.envelope).unwrap(),
    });
    (zip(&built.members).unwrap(), policy)
}

#[test]
fn browser_workbook_admission_requires_verified_v2_design_on_exact_active_managed_roster() {
    let (bytes, policy) = signed_fixture(DesignDigestAlgorithm::V2);
    let checked =
        admit_with_policy(&bytes, None, TENANT, vec![], chrono::Utc::now(), &policy).unwrap();
    let Admission::Verified(checked) = checked else {
        panic!("fixture was not signed")
    };
    assert!(verified_for_tenant(&checked, &policy, TENANT));
    assert!(!verified_for_tenant(&checked, &policy, "another-tenant"));
    let permitted = ImportUsersPrivileges {
        group_assignment: GroupAssignmentPolicy::AnyGroup,
        ..Default::default()
    };
    let mut changed = checked.members.clone();
    changed
        .iter_mut()
        .find(|m| m.name == DEMO_LOGIN_VERIFIERS_MEMBER)
        .unwrap()
        .bytes
        .push(b' ');
    assert!(VerifiedDemoLoginAdmission::take_member(
        &mut changed,
        Some(&checked),
        TENANT,
        &policy,
        permitted
    )
    .is_err());
    let mut original = checked.members.clone();
    assert!(VerifiedDemoLoginAdmission::take_member(
        &mut original,
        Some(&checked),
        TENANT,
        &policy,
        ImportUsersPrivileges::default()
    )
    .is_err());

    let BrowserPolicy::ManagedOnly(pinned, mut binding) = policy.clone() else {
        unreachable!()
    };
    let preview = BrowserPolicy::BrowserOnly(pinned.clone());
    assert!(matches!(
        admit_with_policy(&bytes, None, TENANT, vec![], chrono::Utc::now(), &preview),
        Ok(Admission::Verified(_)),
    ));
    assert!(!verified_for_tenant(&checked, &preview, TENANT));
    binding.sha256 = "b".repeat(64);
    assert!(!verified_for_tenant(
        &checked,
        &BrowserPolicy::ManagedOnly(pinned, binding),
        TENANT
    ));

    let (legacy, policy) = signed_fixture(DesignDigestAlgorithm::V1);
    let Admission::Verified(legacy) =
        admit_with_policy(&legacy, None, TENANT, vec![], chrono::Utc::now(), &policy).unwrap()
    else {
        panic!("fixture was not signed")
    };
    assert!(!verified_for_tenant(&legacy, &policy, TENANT));
}

// Run alone against a disposable loopback fixture realm, with OLD precreated.
#[tokio::test]
#[ignore = "requires disposable loopback Keycloak fixture"]
async fn admitted_browser_workbook_authenticates_without_resetting_existing_user() {
    let url = std::env::var("KEYCLOAK_URL").expect("configure isolated loopback Keycloak");
    let parsed = reqwest::Url::parse(&url).unwrap();
    assert_eq!(parsed.scheme(), "http");
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    assert!(parsed.username().is_empty() && parsed.password().is_none());
    assert!(parsed.port().is_some());
    assert!(matches!(parsed.path(), "" | "/"));
    assert!(parsed.query().is_none() && parsed.fragment().is_none());
    let realm = "workbook-synthetic-proof";
    let token = format!(
        "{}/realms/{realm}/protocol/openid-connect/token",
        url.trim_end_matches('/')
    );
    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let login = |username, password| {
        let request = http.post(&token).form(&[
            ("grant_type", "password"),
            ("client_id", "workbook-proof-client"),
            ("username", username),
            ("password", password),
        ]);
        async move {
            let response = request.send().await.expect("loopback token request failed");
            if response.status() == reqwest::StatusCode::OK {
                assert!(response
                    .json::<serde_json::Value>()
                    .await
                    .unwrap()
                    .get("access_token")
                    .is_some());
                true
            } else {
                assert!(matches!(
                    response.status(),
                    reqwest::StatusCode::BAD_REQUEST | reqwest::StatusCode::UNAUTHORIZED
                ));
                let denial: serde_json::Value = response.json().await.unwrap();
                assert_eq!(denial["error"], "invalid_grant");
                assert_eq!(denial["error_description"], "Invalid user credentials");
                false
            }
        }
    };
    assert!(login(OLD, OLD_PW).await);
    let (bytes, policy) = signed_fixture(DesignDigestAlgorithm::V2);
    let Admission::Verified(proof) =
        admit_with_policy(&bytes, None, TENANT, vec![], chrono::Utc::now(), &policy).unwrap()
    else {
        panic!("fixture was not signed")
    };
    let permissions = ImportUsersPrivileges {
        group_assignment: GroupAssignmentPolicy::AnyGroup,
        ..Default::default()
    };
    let mut members = proof.members.clone();
    let admission = VerifiedDemoLoginAdmission::take_member(
        &mut members,
        Some(&proof),
        TENANT,
        &policy,
        permissions,
    )
    .unwrap()
    .expect("receiver must admit the signed verifier member");
    use crate::services::audit_log::AuditActor;
    use crate::services::import::configuration_package_sections::read_admin_rows;
    use sequent_core::election_config::package_contract::ADMIN_USERS_MEMBER;
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].name, ADMIN_USERS_MEMBER);
    let rows = read_admin_rows(&members[0].bytes).unwrap();
    admission.verify_roster(&rows).unwrap();
    let existing = BTreeMap::from([(OLD.into(), BTreeSet::new())]);
    let new = admission
        .authorized_new_usernames(&existing, || async { Ok(()) })
        .await
        .unwrap();
    assert_eq!(new, [NEW, "trustee1"]);
    let actor = AuditActor {
        user_id: Some("synthetic-workbook-operator".into()),
        ..Default::default()
    };
    let mut audits = Vec::new();
    let seeded = admission
        .seed_new_with_audit(TENANT, realm, &rows, &existing, Some(&actor), |event| {
            audits.push(event.extra);
            std::future::ready(Ok(()))
        })
        .await
        .unwrap();
    assert_eq!(seeded, new);
    assert_eq!(
        audits,
        [
            serde_json::json!({"username":NEW,"group":"sbei"}),
            serde_json::json!({"username":"trustee1","group":"trustee"}),
        ]
    );
    assert!(login(NEW, NEW_PW).await);
    assert!(!login(NEW, OLD_PW).await);
    assert!(login("trustee1", NEW_PW).await);
    assert!(!login("trustee1", OLD_PW).await);
    // A concurrent creation conflict cannot become a reset. This repeats
    // seeding against an intentionally stale snapshot of the former user.
    let stale = BTreeMap::from([
        (NEW.into(), BTreeSet::new()),
        ("trustee1".into(), BTreeSet::new()),
    ]);
    assert!(admission
        .seed_new_with_audit(TENANT, realm, &rows, &stale, Some(&actor), |_| {
            panic!("attempting an existing username should never record a new user");
            #[allow(unreachable_code)]
            std::future::ready(Ok(()))
        })
        .await
        .is_err());
    assert!(login(OLD, OLD_PW).await);
    assert!(!login(OLD, NEW_PW).await);
}
