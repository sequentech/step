// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Browser-signed packages: dispatch, the envelope, the pinned roster and
//! the binding of the whole manifest. Software P-256 keys stand in for the
//! officials' security keys; nothing here is trusted anywhere.

use std::collections::BTreeMap;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ring::digest::{digest, SHA256};
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};

use super::*;
use crate::election_config::archive::zip;
use crate::election_config::manifest::{
    file_entries, ConfigurationRevision, Content, Manifest, PreviousRevision,
    Produced,
};
use crate::election_config::package_verify::{
    admit, admit_for_tenant_at, verify_package, Admission,
    ConfigurationSigning, ConfigurationSigningPolicy, Freshness, LastImport,
    PackageTrust, VerificationScheme,
};
use crate::election_config::webauthn_proof::{
    package_seal_sha256, Action, Binding, Proof, Role, Roster, BUNDLE_FORMAT,
    ROSTER_FORMAT,
};

const TENANT: &str = "tenant-a";
const RP: &str = "architect.ems.example";
const ORIGIN: &str = "https://architect.ems.example";
const CONFIGURATION: &str = "00000000-0000-0000-0000-000000000007";
const EXTERNAL_ID: &str = "ov-2028";
const REVISION: u64 = 3;
const PREVIOUS: &str =
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

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
    let pkcs8 =
        EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
            .unwrap();
    let pair = EcdsaKeyPair::from_pkcs8(
        &ECDSA_P256_SHA256_ASN1_SIGNING,
        pkcs8.as_ref(),
        &rng,
    )
    .unwrap();
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
        let mut authenticator_data =
            digest(&SHA256, RP.as_bytes()).as_ref().to_vec();
        authenticator_data.push(0x05);
        authenticator_data.extend_from_slice(&1u32.to_be_bytes());
        let mut message = authenticator_data.clone();
        message.extend_from_slice(
            digest(&SHA256, client_data.as_bytes()).as_ref(),
        );
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
    b: Official,
    c: Official,
    signer: Official,
}

fn world() -> World {
    World {
        a: official("u-a", Role::Approver),
        b: official("u-b", Role::Approver),
        c: official("u-c", Role::Approver),
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
            [&self.a, &self.b, &self.c, &self.signer]
                .iter()
                .map(|o| entry(o))
                .collect(),
        )
    }

    /// The verifier's own provisioning: this roster, pinned, for this
    /// tenant, needing `required` approvals.
    fn pinned(&self, required: usize) -> PinnedRoster {
        let text = self.roster_text();
        PinnedRoster::load(&text, &Roster::sha256(&text), TENANT, required)
            .unwrap()
    }
}

fn delivery() -> Vec<Artifact> {
    vec![Artifact {
        name: "settings.json".to_string(),
        bytes: b"{\"election\":\"ov-2028\"}".to_vec(),
    }]
}

fn plain_manifest(members: &[Artifact]) -> Manifest {
    let content = Content {
        files: file_entries(members).unwrap(),
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

fn build(w: &World, approvers: &[&Official], signer: &Official) -> Built {
    let members = delivery();
    let manifest = plain_manifest(&members);
    build_with_manifest(w, approvers, signer, members, manifest)
}

fn build_with_manifest(
    w: &World,
    approvers: &[&Official],
    signer: &Official,
    members: Vec<Artifact>,
    manifest: Manifest,
) -> Built {
    let manifest_bytes = manifest.to_bytes().unwrap();
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    let mut bundle = Bundle {
        format: BUNDLE_FORMAT.to_string(),
        configuration: CONFIGURATION.to_string(),
        external_id: EXTERNAL_ID.to_string(),
        revision: REVISION as i64,
        content_sha256: manifest.content_sha256.clone(),
        approvals: approvers
            .iter()
            .map(|o| {
                o.assert(
                    o.binding(
                        Action::ApproveRevision,
                        &manifest.content_sha256,
                    ),
                )
            })
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
    bundle.final_signature =
        signer.assert(signer.binding(Action::SignPackage, &seal));
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

impl Built {
    /// The package, with `extra` members besides the delivery, manifest and envelope.
    fn zip_with(&self, extra: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut all = self.members.clone();
        all.push(Artifact {
            name: MANIFEST_MEMBER.to_string(),
            bytes: self.manifest.clone(),
        });
        all.push(Artifact {
            name: ENVELOPE_MEMBER.to_string(),
            bytes: serde_json::to_vec(&self.envelope).unwrap(),
        });
        for (name, bytes) in extra {
            all.push(Artifact {
                name: name.to_string(),
                bytes: bytes.clone(),
            });
        }
        zip(&all).unwrap()
    }

    fn zip(&self) -> Vec<u8> {
        self.zip_with(&[])
    }

    /// Replaces the manifest, and has the envelope name the new one, as an
    /// attacker who can rewrite the package but not the signatures would.
    fn with_manifest(mut self, manifest: &Manifest) -> Self {
        self.manifest = manifest.to_bytes().unwrap();
        self.envelope["manifest_sha256"] = sha256_hex(&self.manifest).into();
        self
    }
}

fn refusal(result: Result<VerifiedBrowserPackage, Report>) -> (Code, String) {
    let report = result.expect_err("the package must be refused");
    let problem = report.problems.first().expect("a reason").clone();
    (problem.code, problem.id.unwrap_or_default())
}

fn ids(report: &Report) -> Vec<String> {
    report
        .problems
        .iter()
        .filter_map(|p| p.id.clone())
        .collect()
}

#[test]
fn admits_a_package_sealed_over_its_whole_manifest_for_this_tenant() {
    let w = world();
    let built = build(&w, &[&w.a, &w.b], &w.signer);
    let verified =
        verify_browser_package(&built.zip(), &w.pinned(2), None).unwrap();
    assert_eq!(
        verified.approvers,
        vec!["u-a".to_string(), "u-b".to_string()]
    );
    assert_eq!(verified.final_signer, "u-signer");
    assert_eq!(verified.tenant_id, TENANT);
    assert_eq!(verified.manifest_sha256, sha256_hex(&built.manifest));
    assert_eq!(verified.manifest.configuration.revision, REVISION);
    let names: Vec<&str> =
        verified.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, vec!["settings.json"]);
}

#[test]
fn the_certificate_importer_never_takes_a_browser_package_for_unsigned() {
    let w = world();
    let package = build(&w, &[&w.a, &w.b], &w.signer).zip();
    // Every existing entry point, under the most permissive policy there is.
    let optional = ConfigurationSigning {
        policy: ConfigurationSigningPolicy::Optional,
        ..ConfigurationSigning::default()
    };
    for result in [
        admit(&package, None, vec![]),
        admit(&package, Some(&optional), vec![]),
        admit_for_tenant_at(
            &package,
            Some(&optional),
            TENANT,
            vec![],
            chrono::Utc::now(),
        ),
    ] {
        match result {
            Ok(Admission::Unsigned) => {
                panic!("a browser package was admitted as unsigned")
            }
            Ok(Admission::Verified(_)) => {
                panic!("a browser package was verified without its roster")
            }
            Err(report) => assert_eq!(
                ids(&report),
                vec!["package.webauthn-not-enabled".to_string()]
            ),
        }
    }
    let refused =
        verify_package(&package, &PackageTrust::default()).unwrap_err();
    assert_eq!(
        ids(&refused),
        vec!["package.webauthn-not-enabled".to_string()]
    );
}

#[test]
fn anything_named_like_an_envelope_is_never_unsigned_even_without_a_manifest() {
    // A delivery with a stray, malformed or renamed envelope and no other
    // signature member: before, it would have imported as unsigned.
    for name in [
        ENVELOPE_MEMBER,
        "MANIFEST.WEBAUTHN.JSON",
        "manifest.webauthn.json.bak",
        "nested/manifest.webauthn",
    ] {
        let mut members = delivery();
        members.push(Artifact {
            name: name.to_string(),
            bytes: b"not even json".to_vec(),
        });
        let package = zip(&members).unwrap();
        match admit(&package, None, vec![]) {
            Ok(_) => panic!("'{name}' was admitted"),
            Err(report) => assert_eq!(
                ids(&report),
                vec!["package.webauthn-not-enabled".to_string()]
            ),
        }
    }
    // And a delivery with nothing of the kind still imports as before.
    let plain = zip(&delivery()).unwrap();
    assert!(matches!(
        admit(&plain, None, vec![]),
        Ok(Admission::Unsigned)
    ));
}

#[test]
fn refuses_a_package_signed_both_ways_or_with_more_than_one_envelope() {
    let w = world();
    let built = build(&w, &[&w.a, &w.b], &w.signer);
    let pinned = w.pinned(2);
    for extra in [SIGNATURE_MEMBER, CHAIN_MEMBER] {
        let (code, id) = refusal(verify_browser_package(
            &built.zip_with(&[(extra, b"x".to_vec())]),
            &pinned,
            None,
        ));
        assert_eq!(
            (code, id.as_str()),
            (Code::InvalidValue, "package.webauthn-mixed")
        );
    }
    for extra in ["manifest.webauthn.json.old", "Manifest.WebAuthn.json"] {
        let (_, id) = refusal(verify_browser_package(
            &built.zip_with(&[(extra, b"{}".to_vec())]),
            &pinned,
            None,
        ));
        assert_eq!(id, "package.webauthn-unexpected-member");
    }
}

#[test]
fn refuses_an_envelope_with_no_manifest_to_seal() {
    let w = world();
    let built = build(&w, &[&w.a, &w.b], &w.signer);
    let mut all = built.members.clone();
    all.push(Artifact {
        name: ENVELOPE_MEMBER.to_string(),
        bytes: serde_json::to_vec(&built.envelope).unwrap(),
    });
    let (code, id) = refusal(verify_browser_package(
        &zip(&all).unwrap(),
        &w.pinned(2),
        None,
    ));
    assert_eq!(
        (code, id.as_str()),
        (Code::Unsigned, "package.webauthn-no-manifest")
    );
}

#[test]
fn reads_only_the_one_envelope_version_it_knows_and_nothing_extra_in_it() {
    let w = world();
    let pinned = w.pinned(2);
    let cases: [(&str, fn(&mut serde_json::Value), Code, &str); 4] = [
        (
            "a later version",
            |e| {
                e["format"] = "sequent.configuration-webauthn-envelope/2".into()
            },
            Code::IncompatibleVersion,
            "package.webauthn-unknown-format",
        ),
        (
            "no format",
            |e| {
                e.as_object_mut().unwrap().remove("format");
            },
            Code::Unreadable,
            "package.webauthn-unreadable",
        ),
        (
            "another family",
            |e| e["format"] = "sequent.configuration-manifest/1".into(),
            Code::Unreadable,
            "package.webauthn-unreadable",
        ),
        (
            "an extra field",
            |e| e["roster"] = serde_json::json!({"keys": []}),
            Code::Unreadable,
            "package.webauthn-unreadable",
        ),
    ];
    for (why, change, code, id) in cases {
        let mut built = build(&w, &[&w.a, &w.b], &w.signer);
        change(&mut built.envelope);
        let refused =
            refusal(verify_browser_package(&built.zip(), &pinned, None));
        assert_eq!((refused.0, refused.1.as_str()), (code, id), "{why}");
    }
}

#[test]
fn an_invalid_json_envelope_is_refused_even_with_a_manifest() {
    let w = world();
    let built = build(&w, &[&w.a, &w.b], &w.signer);
    let package = zip(&[
        Artifact {
            name: MANIFEST_MEMBER.to_string(),
            bytes: built.manifest,
        },
        Artifact {
            name: ENVELOPE_MEMBER.to_string(),
            bytes: b"{".to_vec(),
        },
    ])
    .unwrap();
    let (code, id) =
        refusal(verify_browser_package(&package, &w.pinned(2), None));
    assert_eq!(
        (code, id.as_str()),
        (Code::Unreadable, "package.webauthn-unreadable")
    );
}

#[test]
fn matching_manifest_digest_does_not_make_unreadable_json_valid() {
    let w = world();
    let mut built = build(&w, &[&w.a, &w.b], &w.signer);
    built.manifest = b"{".to_vec();
    built.envelope["manifest_sha256"] = sha256_hex(&built.manifest).into();

    assert_eq!(
        refusal(verify_browser_package(&built.zip(), &w.pinned(2), None)),
        (Code::Unreadable, "package.unreadable-manifest".to_string())
    );
}

#[test]
fn a_signed_package_still_needs_a_readable_production_time() {
    let w = world();
    let members = delivery();
    let mut manifest = plain_manifest(&members);
    manifest.produced.at = "not a timestamp".to_string();
    let built =
        build_with_manifest(&w, &[&w.a, &w.b], &w.signer, members, manifest);
    let bytes = built.zip();
    let pinned = w.pinned(2);
    // The clockless verifier can validate the signatures on the same bytes.
    assert!(verify_browser_package(&bytes, &pinned, None).is_ok());
    let now = chrono::DateTime::parse_from_rfc3339("2026-01-02T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert_eq!(
        refusal(verify_browser_package(&bytes, &pinned, Some(now))),
        (Code::InvalidValue, "package.invalid-time".to_string())
    );
}

#[test]
fn a_browser_manifest_requires_a_configuration_and_a_positive_revision() {
    let w = world();
    let pinned = w.pinned(2);
    let mut nameless = plain_manifest(&delivery());
    nameless.configuration.external_id.clear();
    let mut unnumbered = plain_manifest(&delivery());
    unnumbered.configuration.revision = 0;
    for manifest in [nameless, unnumbered] {
        let built =
            build(&w, &[&w.a, &w.b], &w.signer).with_manifest(&manifest);
        let (code, id) =
            refusal(verify_browser_package(&built.zip(), &pinned, None));
        assert_eq!(
            (code, id.as_str()),
            (Code::InvalidValue, "package.webauthn-revision")
        );
    }
}

#[test]
fn refuses_a_manifest_the_envelope_does_not_name() {
    let w = world();
    let mut built = build(&w, &[&w.a, &w.b], &w.signer);
    // One byte more: the same JSON, different bytes, a different digest.
    built.manifest.push(b'\n');
    let (code, id) =
        refusal(verify_browser_package(&built.zip(), &w.pinned(2), None));
    assert_eq!(
        (code, id.as_str()),
        (Code::IntegrityMismatch, "package.webauthn-manifest-changed")
    );
}

#[test]
fn the_final_signature_binds_the_whole_manifest_including_previous_and_name() {
    let w = world();
    let pinned = w.pinned(2);
    let original = plain_manifest(&delivery());
    let rewrites: [(&str, fn(&mut Manifest)); 5] = [
        ("previous revision", |m| {
            m.configuration.previous.as_mut().unwrap().revision = 1
        }),
        ("previous digest", |m| {
            m.configuration.previous.as_mut().unwrap().manifest_sha256 =
                "d".repeat(64)
        }),
        ("no previous", |m| m.configuration.previous = None),
        ("name", |m| {
            m.configuration.name = "Something Else".to_string()
        }),
        ("producer", |m| m.produced.custodian = "Mallory".to_string()),
    ];
    for (what, rewrite) in rewrites {
        let mut manifest = original.clone();
        rewrite(&mut manifest);
        // The envelope names the rewritten manifest; the final signature's
        // binding is still the seal over the original, so it is refused.
        let built =
            build(&w, &[&w.a, &w.b], &w.signer).with_manifest(&manifest);
        let (code, id) =
            refusal(verify_browser_package(&built.zip(), &pinned, None));
        assert_eq!(
            (code, id.as_str()),
            (Code::IntegrityMismatch, "package.webauthn-refused"),
            "{what}"
        );
    }
}

#[test]
fn rebinding_the_final_signature_to_a_rewritten_manifest_breaks_the_signature()
{
    // The attacker rewrites the manifest, names it in the envelope and
    // points the final proof's binding at the new seal: the key signed the
    // old challenge, so the signature no longer verifies.
    let w = world();
    let mut manifest = plain_manifest(&delivery());
    manifest.configuration.previous = None;
    let mut built =
        build(&w, &[&w.a, &w.b], &w.signer).with_manifest(&manifest);
    let bundle: Bundle =
        serde_json::from_value(built.envelope["bundle"].clone()).unwrap();
    let manifest_sha256 = sha256_hex(&built.manifest);
    let seal = package_seal_sha256(
        &bundle,
        TENANT,
        RP,
        SealedManifest {
            envelope: ENVELOPE_V1,
            external_id: EXTERNAL_ID,
            revision: REVISION,
            previous: None,
            content_sha256: &manifest.content_sha256,
            manifest_sha256: &manifest_sha256,
        },
    );
    built.envelope["bundle"]["final_signature"]["binding"]["content_sha256"] =
        seal.into();
    let (code, id) =
        refusal(verify_browser_package(&built.zip(), &w.pinned(2), None));
    assert_eq!(
        (code, id.as_str()),
        (Code::BadSignature, "package.webauthn-refused")
    );
}

#[test]
fn refuses_another_tenants_package_and_a_roster_pinned_for_another_tenant() {
    let w = world();
    let mut built = build(&w, &[&w.a, &w.b], &w.signer);
    built.envelope["tenant_id"] = "tenant-b".into();
    let (_, id) =
        refusal(verify_browser_package(&built.zip(), &w.pinned(2), None));
    assert_eq!(id, "package.webauthn-other-tenant");

    // The tenant-a roster can't be loaded as tenant-b's, pinned or not.
    let text = w.roster_text();
    assert!(matches!(
        PinnedRoster::load(&text, &Roster::sha256(&text), "tenant-b", 2),
        Err(ProofError::Policy(_))
    ));
}

#[test]
fn a_package_sealed_for_another_tenant_fails_even_under_its_own_name() {
    // Signed honestly for tenant-b's roster, then relabelled tenant-a: the
    // bindings and the seal name tenant-b, so tenant-a's roster refuses it.
    let w = world();
    let mut built = build(&w, &[&w.a, &w.b], &w.signer);
    let bundle = built.envelope["bundle"].as_object_mut().unwrap();
    let approvals = bundle["approvals"].as_array_mut().unwrap();
    approvals[0]["binding"]["tenant"] = "tenant-b".into();
    let (_, id) =
        refusal(verify_browser_package(&built.zip(), &w.pinned(2), None));
    assert_eq!(id, "package.webauthn-refused");
}

#[test]
fn refuses_a_manifest_carrying_certificate_signatures_or_a_format_two_binding()
{
    let w = world();
    let pinned = w.pinned(2);
    let mut with_lists = plain_manifest(&delivery());
    with_lists.revocation_lists = vec!["-----BEGIN X509 CRL-----".to_string()];
    let mut with_v2 = plain_manifest(&delivery());
    with_v2.format = ManifestFormat::V2;
    for manifest in [with_lists, with_v2] {
        let built =
            build(&w, &[&w.a, &w.b], &w.signer).with_manifest(&manifest);
        let (_, id) =
            refusal(verify_browser_package(&built.zip(), &pinned, None));
        assert_eq!(id, "package.webauthn-mixed");
    }
}

#[test]
fn refuses_an_incoherent_previous_revision() {
    let w = world();
    let mut later = plain_manifest(&delivery());
    later.configuration.previous.as_mut().unwrap().revision = REVISION;
    let mut unnamed = plain_manifest(&delivery());
    unnamed
        .configuration
        .previous
        .as_mut()
        .unwrap()
        .manifest_sha256 = "NOT-HEX".to_string();
    for manifest in [later, unnamed] {
        let built =
            build(&w, &[&w.a, &w.b], &w.signer).with_manifest(&manifest);
        let (_, id) =
            refusal(verify_browser_package(&built.zip(), &w.pinned(2), None));
        assert_eq!(id, "package.webauthn-previous");
    }
}

#[test]
fn a_valid_seal_does_not_excuse_an_incorrect_manifest_content_digest() {
    let w = world();
    let members = delivery();
    let mut manifest = plain_manifest(&members);
    manifest.content_sha256 = "a".repeat(64);
    let built =
        build_with_manifest(&w, &[&w.a, &w.b], &w.signer, members, manifest);
    let (code, id) =
        refusal(verify_browser_package(&built.zip(), &w.pinned(2), None));
    assert_eq!(
        (code, id.as_str()),
        (Code::IntegrityMismatch, "package.content-mismatch")
    );
}

#[test]
fn a_browser_envelope_must_name_the_same_revision_identity_as_the_manifest() {
    let w = world();
    let pinned = w.pinned(2);
    for (field, wrong, expected) in [
        (
            "external_id",
            serde_json::json!("other-demo"),
            "configuration",
        ),
        ("revision", serde_json::json!(9), "revision"),
        (
            "content_sha256",
            serde_json::json!("d".repeat(64)),
            "content",
        ),
    ] {
        let mut built = build(&w, &[&w.a, &w.b], &w.signer);
        built.envelope["bundle"][field] = wrong;
        let report =
            verify_browser_package(&built.zip(), &pinned, None).unwrap_err();
        let problem = report.errors().next().expect("the binding refusal");
        assert_eq!(problem.code, Code::IntegrityMismatch, "{expected}");
        assert_eq!(problem.id.as_deref(), Some("package.webauthn-refused"));
        assert_eq!(
            problem.details.get("reason"),
            Some(&format!("the binding is not for this package's {expected}"))
        );
    }
}

#[test]
fn checks_every_delivered_member_after_the_signatures() {
    let w = world();
    let pinned = w.pinned(2);
    let mut changed = build(&w, &[&w.a, &w.b], &w.signer);
    changed.members[0].bytes = b"{\"election\":\"other\"}".to_vec();
    let report =
        verify_browser_package(&changed.zip(), &pinned, None).unwrap_err();
    assert!(ids(&report).contains(&"package.file-changed".to_string()));

    // A roster shipped in the package is just an unlisted file: refused,
    // and never read.
    let extra = build(&w, &[&w.a, &w.b], &w.signer);
    let report = verify_browser_package(
        &extra.zip_with(&[("roster.json", w.roster_text().into_bytes())]),
        &pinned,
        None,
    )
    .unwrap_err();
    assert!(ids(&report).contains(&"package.file-extra".to_string()));
}

#[test]
fn the_threshold_is_the_pinned_one() {
    let w = world();
    let package = build(&w, &[&w.a, &w.b], &w.signer).zip();
    let (code, _) =
        refusal(verify_browser_package(&package, &w.pinned(3), None));
    assert_eq!(code, Code::NotApproved);
}

#[test]
fn refuses_a_package_produced_in_the_future() {
    let w = world();
    let package = build(&w, &[&w.a, &w.b], &w.signer).zip();
    let long_ago = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let report = verify_browser_package(&package, &w.pinned(2), Some(long_ago))
        .unwrap_err();
    assert!(!report.problems.is_empty());
    assert!(verify_browser_package(
        &package,
        &w.pinned(2),
        Some(chrono::Utc::now())
    )
    .is_ok());
}

#[test]
fn a_pinned_roster_refuses_one_key_under_two_people() {
    let w = world();
    // u-b listed with u-a's public key, under its own id and credential.
    let mut b = entry(&w.b);
    b["public_key"] = w.a.public().into();
    let text = roster_json(TENANT, vec![entry(&w.a), b, entry(&w.signer)]);
    assert!(matches!(
        PinnedRoster::load(&text, &Roster::sha256(&text), TENANT, 1),
        Err(ProofError::Roster(_))
    ));
    // The same credential under two people, one of them spelled with padding.
    let mut c = entry(&w.c);
    c["credential_id"] = format!("{}=", w.a.credential()).into();
    let text = roster_json(TENANT, vec![entry(&w.a), c, entry(&w.signer)]);
    assert!(
        PinnedRoster::load(&text, &Roster::sha256(&text), TENANT, 1).is_err()
    );
}

#[test]
fn a_pinned_roster_is_the_provisioned_one_with_a_usable_threshold() {
    let w = world();
    let text = w.roster_text();
    let pin = Roster::sha256(&text);
    // Another pin, a pin that isn't a digest, an impossible threshold.
    assert!(PinnedRoster::load(&text, &"0".repeat(64), TENANT, 2).is_err());
    assert!(PinnedRoster::load(&text, &pin.to_uppercase(), TENANT, 2).is_err());
    for required in [0, 4, 17] {
        assert!(
            matches!(
                PinnedRoster::load(&text, &pin, TENANT, required),
                Err(ProofError::Policy(_))
            ),
            "{required}"
        );
    }
    // No final signer.
    let text = roster_json(TENANT, vec![entry(&w.a), entry(&w.b)]);
    assert!(
        PinnedRoster::load(&text, &Roster::sha256(&text), TENANT, 1).is_err()
    );
    // An origin that isn't exact.
    for origin in [
        "https://architect.ems.example/",
        "http://architect.ems.example",
        "https://Architect.ems.example",
    ] {
        let text = serde_json::json!({
            "format": ROSTER_FORMAT, "rp_id": RP, "origins": [origin], "tenant": TENANT,
            "keys": [entry(&w.a), entry(&w.signer)],
        })
        .to_string();
        assert!(
            PinnedRoster::load(&text, &Roster::sha256(&text), TENANT, 1)
                .is_err(),
            "{origin}"
        );
    }
    // A key whose registration digest isn't one.
    let mut a = entry(&w.a);
    a["record_sha256"] = "B".repeat(64).into();
    let text = roster_json(TENANT, vec![a, entry(&w.signer)]);
    assert!(
        PinnedRoster::load(&text, &Roster::sha256(&text), TENANT, 1).is_err()
    );
}

#[test]
fn the_same_physical_key_never_counts_twice_in_a_bundle() {
    // A roster that only `Roster::read` saw would refuse this already; the
    // bundle check refuses it too, by key bytes, whatever the ids say.
    let w = world();
    let built = build(&w, &[&w.a, &w.a], &w.signer);
    let (code, _) =
        refusal(verify_browser_package(&built.zip(), &w.pinned(2), None));
    assert_eq!(code, Code::NotApproved);
}

#[test]
fn a_browser_verified_package_is_recorded_as_one_and_never_as_a_certificate() {
    let w = world();
    let pinned = w.pinned(2);
    let built = build(&w, &[&w.a, &w.b], &w.signer);
    let verified = verify_browser_package(&built.zip(), &pinned, None)
        .unwrap()
        .into_verified_package();
    // The verifier's own typed mark, with the pin it checked against.
    assert_eq!(
        verified.scheme,
        VerificationScheme::Browser {
            roster_sha256: pinned.sha256().to_string()
        }
    );
    assert!(is_browser_verified(&verified));
    assert_eq!(verified.signer.subject, "u-signer");
    assert_eq!(verified.signer.fingerprint_sha256, pinned.sha256());
    assert_eq!(verified.signer_id.as_deref(), Some("u-signer"));
    assert_eq!(
        verified.approver_ids,
        vec!["u-a".to_string(), "u-b".to_string()]
    );
    assert_eq!(verified.manifest_sha256, sha256_hex(&built.manifest));
    // Names decide nothing: the scheme alone does, either way round.
    let mut relabelled = verified.clone();
    relabelled.signer.issuer = "CN=Test Package Root".to_string();
    assert!(is_browser_verified(&relabelled));
    let mut certificate = verified.clone();
    certificate.scheme = VerificationScheme::Certificate;
    assert!(!is_browser_verified(&certificate));
}

/// What a tenant imported last, for `browser_freshness`.
fn last(revision: u64, manifest_sha256: &str) -> LastImport {
    LastImport {
        revision,
        manifest_sha256: manifest_sha256.to_string(),
    }
}

#[test]
fn a_browser_package_follows_exactly_the_tenants_last_import() {
    let w = world();
    // Revision 3, naming revision 2 with digest PREVIOUS.
    let verified = verify_browser_package(
        &build(&w, &[&w.a, &w.b], &w.signer).zip(),
        &w.pinned(2),
        None,
    )
    .unwrap()
    .into_verified_package();
    assert_eq!(
        browser_freshness(&verified, Some(&last(2, PREVIOUS))),
        Ok(Freshness::New)
    );
    // The very package imported last.
    assert_eq!(
        browser_freshness(&verified, Some(&last(3, &verified.manifest_sha256))),
        Ok(Freshness::AlreadyImported)
    );
    let refused = |last: Option<LastImport>| {
        browser_freshness(&verified, last.as_ref())
            .expect_err("must not be next")
            .id
            .unwrap_or_default()
    };
    // Another revision 2, an older or newer last import, or none at all.
    assert_eq!(
        refused(Some(last(2, &"d".repeat(64)))),
        "package.webauthn-not-next"
    );
    assert_eq!(
        refused(Some(last(1, PREVIOUS))),
        "package.webauthn-not-next"
    );
    assert_eq!(
        refused(Some(last(3, &"e".repeat(64)))),
        "package.webauthn-not-next"
    );
    assert_eq!(
        refused(Some(last(4, &"e".repeat(64)))),
        "package.webauthn-not-next"
    );
    assert_eq!(refused(None), "package.webauthn-not-next");
}

#[test]
fn a_first_browser_package_names_no_previous_revision() {
    let w = world();
    let mut manifest = plain_manifest(&delivery());
    manifest.configuration.previous = None;
    // Sealed honestly over a manifest with no previous revision.
    let members = delivery();
    let manifest_bytes = manifest.to_bytes().unwrap();
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    let mut bundle = Bundle {
        format: BUNDLE_FORMAT.to_string(),
        configuration: CONFIGURATION.to_string(),
        external_id: EXTERNAL_ID.to_string(),
        revision: REVISION as i64,
        content_sha256: manifest.content_sha256.clone(),
        approvals: [&w.a, &w.b]
            .iter()
            .map(|o| {
                o.assert(
                    o.binding(
                        Action::ApproveRevision,
                        &manifest.content_sha256,
                    ),
                )
            })
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
            previous: None,
            content_sha256: &manifest.content_sha256,
            manifest_sha256: &manifest_sha256,
        },
    );
    bundle.final_signature = w
        .signer
        .assert(w.signer.binding(Action::SignPackage, &seal));
    let built = Built {
        members,
        manifest: manifest_bytes,
        envelope: serde_json::json!({
            "format": ENVELOPE_V1,
            "tenant_id": TENANT,
            "manifest_sha256": manifest_sha256,
            "bundle": bundle,
        }),
    };
    let verified = verify_browser_package(&built.zip(), &w.pinned(2), None)
        .unwrap()
        .into_verified_package();
    assert_eq!(browser_freshness(&verified, None), Ok(Freshness::New));
    // Once anything was imported, a package that names nothing is not next.
    assert!(browser_freshness(&verified, Some(&last(2, PREVIOUS))).is_err());
}

#[test]
fn malformed_zip_metadata_never_becomes_unsigned() {
    let bytes = b"PK\x03\x04truncated";
    assert!(admit(bytes, None, vec![]).is_err());
    let (code, id) =
        refusal(verify_browser_package(bytes, &world().pinned(2), None));
    assert_eq!(code, Code::Unreadable);
    assert_eq!(id, "package.unreadable-zip");
}

#[test]
fn valid_unsigned_zip_keeps_legacy_admission() {
    let bytes = zip(&delivery()).unwrap();
    assert!(matches!(
        admit(&bytes, None, vec![]),
        Ok(Admission::Unsigned)
    ));
}

#[test]
fn noncanonical_signature_metadata_cannot_be_unsigned() {
    for name in [
        "Manifest.json",
        "./manifest.sig",
        "nested/manifest.chain.pem",
    ] {
        let bytes = zip(&[Artifact {
            name: name.into(),
            bytes: b"metadata".to_vec(),
        }])
        .unwrap();
        assert!(admit(&bytes, None, vec![]).is_err(), "{name}");
    }
}

#[test]
fn pinned_policy_keeps_the_configured_minimum_and_exact_metadata() {
    let w = world();
    let mut text: Value = serde_json::from_str(&w.roster_text()).unwrap();
    // These are supported deployment shapes, not the signing origins used
    // by the P-256 fixture below.
    text["origins"] = serde_json::json!([
        "https://architect2.ems.example:8443",
        "http://localhost",
        "http://127.0.0.1:3000",
    ]);
    let text = text.to_string();
    let digest = Roster::sha256(&text);
    let pinned = PinnedRoster::load(&text, &digest, TENANT, 1).unwrap();
    assert_eq!(pinned.tenant(), TENANT);
    assert_eq!(pinned.rp_id(), RP);
    assert_eq!(pinned.required(), 1);
    assert_eq!(pinned.sha256(), digest);

    let built = build(&w, &[&w.a, &w.b], &w.signer);
    let verified = verify_browser_package(&built.zip(), &w.pinned(1), None)
        .expect("the receiver's independently pinned count is a minimum");
    assert_eq!(verified.approvers, [w.a.subject, w.b.subject]);
    assert_eq!(verified.final_signer, w.signer.subject);
}

#[test]
fn browser_detection_distinguishes_ordinary_delivery_from_browser_package() {
    let w = world();
    let ordinary = zip(&delivery()).unwrap();
    assert!(!has_browser_members(&ordinary));
    assert_eq!(
        refusal(verify_browser_package(&ordinary, &w.pinned(2), None)),
        (Code::Unsigned, "package.webauthn-no-envelope".to_owned())
    );
    assert!(has_browser_members(
        &build(&w, &[&w.a, &w.b], &w.signer).zip()
    ));
}

#[test]
fn browser_problem_preserves_the_typed_failure_and_localization_context() {
    use ProofError::*;
    let cases = [
        (Format("proof"), Code::Unreadable, "not a proof document"),
        (Roster("no usable keys".into()), Code::UntrustedSigner, "the roster is not usable: no usable keys"),
        (Encoding("key"), Code::Unreadable, "key is not base64url"),
        (Binding("tenant"), Code::IntegrityMismatch, "the binding is not for this tenant"),
        (UnknownKey("official-1".into()), Code::UntrustedSigner, "key official-1 is not on the roster"),
        (WrongKey("official-1".into()), Code::UntrustedSigner,
         "key official-1 is not this person's, this credential or for this role"),
        (AuthenticatorData("unavailable"), Code::BadSignature, "the authenticator data is not acceptable: unavailable"),
        (Signature, Code::BadSignature, "the signature does not verify"),
        (SignerApproved, Code::NotApproved, "the final signer approved this revision too"),
        (Policy("no tenant".into()), Code::UntrustedSigner, "the pinned signing policy is not usable: no tenant"),
    ];
    for (failure, code, reason) in cases {
        let report = proof_refused(&failure);
        let mut errors = report.errors();
        let problem = errors.next().expect("one refusal");
        assert!(errors.next().is_none());
        assert_eq!(problem.code, code);
        assert_eq!(problem.id.as_deref(), Some("package.webauthn-refused"));
        assert_eq!(problem.path, ENVELOPE_MEMBER);
        assert_eq!(
            problem.details.get("reason").map(String::as_str),
            Some(reason)
        );
    }
}
