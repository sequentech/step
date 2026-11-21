// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Honest software-only WebAuthn producer fixture for receiver admission tests.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::{
    digest::{digest, SHA256},
    rand::SystemRandom,
    signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING},
};
use sequent_core::election_config::{
    archive::{zip, Artifact},
    manifest::{
        file_entries, sha256_hex, ConfigurationRevision, Content, Manifest, ManifestFormat,
        Produced, MANIFEST_MEMBER,
    },
    webauthn_package::{ENVELOPE_MEMBER, ENVELOPE_V1},
    webauthn_proof::{
        package_seal_sha256, Action, Binding, Bundle, Proof, Roster, SealedManifest, BUNDLE_FORMAT,
        ROSTER_FORMAT,
    },
};
use serde_json::{json, Value};

pub(super) const RP: &str = "architect.example";
pub(super) const ORIGIN: &str = "https://architect.example";

struct Official {
    index: u8,
    pair: EcdsaKeyPair,
}

impl Official {
    fn new(index: u8) -> Self {
        let rng = SystemRandom::new();
        let algorithm = &ECDSA_P256_SHA256_ASN1_SIGNING;
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(algorithm, &rng).unwrap();
        let pair = EcdsaKeyPair::from_pkcs8(algorithm, pkcs8.as_ref(), &rng).unwrap();
        Self { index, pair }
    }

    fn id(&self) -> String {
        format!("00000000-0000-4000-8000-{:012x}", self.index)
    }

    fn subject(&self) -> String {
        format!("synthetic-official-{}", self.index)
    }

    fn credential(&self) -> String {
        URL_SAFE_NO_PAD.encode([self.index; 24])
    }

    fn roster_entry(&self, role: &str) -> Value {
        json!({
            "key": self.id(), "subject": self.subject(), "credential_id": self.credential(),
            "role": role, "algorithm": "es256",
            "public_key": URL_SAFE_NO_PAD.encode(self.pair.public_key().as_ref()),
            "record_sha256": "ab".repeat(32)
        })
    }

    fn assert(&self, tenant: &str, config: &str, action: Action, content: &str) -> Proof {
        let binding = Binding {
            rp_id: RP.into(),
            tenant: tenant.into(),
            subject: self.subject(),
            action,
            configuration: config.into(),
            revision: 1,
            content_sha256: content.into(),
            signer: self.id(),
        };
        let nonce = [self.index; 32];
        let client = serde_json::to_vec(&json!({
            "type": "webauthn.get", "origin": ORIGIN,
            "challenge": URL_SAFE_NO_PAD.encode(binding.challenge(&nonce)),
        }))
        .unwrap();
        let mut authenticator = digest(&SHA256, RP.as_bytes()).as_ref().to_vec();
        authenticator.push(0x05);
        authenticator.extend_from_slice(&1u32.to_be_bytes());
        let mut signed = authenticator.clone();
        signed.extend_from_slice(digest(&SHA256, &client).as_ref());
        let signature = self.pair.sign(&SystemRandom::new(), &signed).unwrap();
        Proof {
            binding,
            nonce: URL_SAFE_NO_PAD.encode(nonce),
            credential_id: self.credential(),
            authenticator_data: URL_SAFE_NO_PAD.encode(authenticator),
            client_data_json: URL_SAFE_NO_PAD.encode(client),
            signature: URL_SAFE_NO_PAD.encode(signature.as_ref()),
        }
    }
}

pub(super) struct Fixture {
    pub roster: String,
    pub package: Vec<u8>,
}

pub(super) fn fixture(tenant: &str) -> Fixture {
    let approvers = [Official::new(1), Official::new(2)];
    let signer = Official::new(3);
    let roster = json!({
        "format": ROSTER_FORMAT, "tenant": tenant, "rp_id": RP, "origins": [ORIGIN],
        "keys": [approvers[0].roster_entry("approver"), approvers[1].roster_entry("approver"), signer.roster_entry("final-signer")],
    }).to_string();
    assert_eq!(Roster::read(&roster).unwrap().tenant, tenant);

    let mut members = vec![Artifact {
        name: "official_election_setup.zip".into(),
        bytes: zip(&[Artifact {
            name: "export_election_event-test.json".into(),
            bytes: b"{}".to_vec(),
        }])
        .unwrap(),
    }];
    let content = Content {
        files: file_entries(&members).unwrap(),
        ballot_designs: vec![],
        reports: vec![],
    };
    let manifest = Manifest {
        format: ManifestFormat::V1,
        configuration: ConfigurationRevision {
            external_id: "synthetic-browser-coverage".into(),
            name: "Synthetic election".into(),
            revision: 1,
            previous: None,
        },
        content_sha256: content.sha256().unwrap(),
        content,
        approvals: vec![],
        produced: Produced {
            at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            custodian: "Synthetic seal signer".into(),
            key_label: "software fixture".into(),
            producer: Default::default(),
        },
        revocation_lists: vec![],
        signing: None,
    };
    let manifest_bytes = manifest.to_bytes().unwrap();
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    let config = "00000000-0000-4000-8000-000000000042";
    let mut bundle = Bundle {
        format: BUNDLE_FORMAT.into(),
        configuration: config.into(),
        external_id: manifest.configuration.external_id.clone(),
        revision: 1,
        content_sha256: manifest.content_sha256.clone(),
        approvals: approvers
            .iter()
            .map(|official| {
                official.assert(
                    tenant,
                    config,
                    Action::ApproveRevision,
                    &manifest.content_sha256,
                )
            })
            .collect(),
        final_signature: signer.assert(tenant, config, Action::SignPackage, &manifest_sha256),
    };
    let seal = package_seal_sha256(
        &bundle,
        tenant,
        RP,
        SealedManifest {
            envelope: ENVELOPE_V1,
            external_id: &manifest.configuration.external_id,
            revision: 1,
            previous: None,
            content_sha256: &manifest.content_sha256,
            manifest_sha256: &manifest_sha256,
        },
    );
    bundle.final_signature = signer.assert(tenant, config, Action::SignPackage, &seal);
    members.push(Artifact {
        name: MANIFEST_MEMBER.into(),
        bytes: manifest_bytes,
    });
    members.push(Artifact {
        name: ENVELOPE_MEMBER.into(),
        bytes: serde_json::to_vec(&json!({
            "format": ENVELOPE_V1, "tenant_id": tenant, "manifest_sha256": manifest_sha256,
            "bundle": bundle,
        }))
        .unwrap(),
    });
    Fixture {
        roster,
        package: zip(&members).unwrap(),
    }
}
