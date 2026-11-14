// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The standalone flow end to end, with the builders the browser and the
//! signing companion use: request, approvals, manifest, package — then the
//! importer's own verification of the result.

use super::*;
use crate::election_config::manifest::{approval_payload_v2, RevisionContent};
use crate::election_config::standalone_signing::{
    approval_request, assemble_manifest, assemble_verified_package,
    check_approval, check_request, ApprovalRequest, CheckedRequest,
    FinalSigner, InstalledSigning,
};

fn installed(v2: &V2) -> InstalledSigning {
    InstalledSigning {
        tenant_id: TENANT.to_string(),
        signing: policy(v2),
    }
}

fn revision() -> RevisionContent {
    let manifest = unsigned_manifest(3);
    RevisionContent {
        delivery_name: "comelec-demo.zip".to_string(),
        members: members(),
        content: manifest.content,
    }
}

fn delivery() -> Vec<u8> {
    crate::election_config::archive::zip(&members()).unwrap()
}

fn configuration() -> ConfigurationRevision {
    unsigned_manifest(3).configuration
}

fn approval(
    request: &crate::election_config::standalone_signing::ApprovalRequest,
    key: &Key,
    id: &str,
) -> Approval {
    let payload = approval_payload_v2(&request.binding, id).unwrap();
    Approval {
        name: format!("DEMO {id}"),
        role: "approver".to_string(),
        signed_at: "2026-11-13T10:00:00Z".to_string(),
        algorithm: key.algorithm,
        certificate_chain: key.chain_pem.clone(),
        signature: STANDARD.encode((key.sign)(payload.as_bytes())),
        signer_id: Some(id.to_string()),
    }
}

fn custodian() -> FinalSigner {
    FinalSigner {
        signer_id: "custodian".to_string(),
        key_label: "p12".to_string(),
        producer: [("election-architect".to_string(), "test".to_string())]
            .into_iter()
            .collect(),
    }
}

#[test]
fn the_standalone_flow_produces_a_package_the_importer_verifies() {
    let v2 = v2_world();
    let installed = installed(&v2);
    let request =
        approval_request(&installed, configuration(), &revision()).unwrap();
    // Every approver and the final signer check the request first.
    let checked = check_request(&installed, &request, &delivery()).unwrap();
    assert_eq!(checked.members(), members().as_slice());
    let approvals = vec![
        approval(&request, &v2.approver_1, "approver-1"),
        approval(&request, &v2.approver_2, "approver-2"),
    ];
    let manifest =
        assemble_manifest(&installed, &checked, approvals, &custodian(), now())
            .unwrap();
    let bytes = manifest.to_bytes().unwrap();
    let package = assemble_verified_package(
        &installed,
        &checked,
        &bytes,
        &(v2.custodian.sign)(&bytes),
        &v2.custodian.chain_pem,
        now(),
    )
    .unwrap();
    assert_eq!(package.name, "comelec-demo-r3.zip");
    // The importer, with the tenant's own (identical) policy.
    let trust = installed.signing.trust_for(TENANT, Vec::new()).unwrap();
    let verified = verify_package_at(&package.bytes, &trust, now()).unwrap();
    assert_eq!(verified.signer_id.as_deref(), Some("custodian"));
    assert_eq!(verified.approver_ids, vec!["approver-1", "approver-2"]);
}

#[test]
fn a_request_that_is_not_what_it_says_is_refused() {
    let v2 = v2_world();
    let installed = installed(&v2);
    let request =
        approval_request(&installed, configuration(), &revision()).unwrap();
    let ids_of =
        |result: Result<CheckedRequest, Report>| ids(&result.unwrap_err());
    // The delivery was changed.
    let mut other = members();
    other[1].bytes = b"{\"version\":5}".to_vec();
    let tampered = crate::election_config::archive::zip(&other).unwrap();
    assert!(ids_of(check_request(&installed, &request, &tampered))
        .contains(&"request.files-differ".to_string()));
    // Another installed policy (an administrator changed it).
    let mut changed = installed.clone();
    changed.signing.required_approvals = 1;
    assert!(ids_of(check_request(&changed, &request, &delivery()))
        .contains(&"request.policy-changed".to_string()));
    // Another tenant.
    let mut elsewhere = installed.clone();
    elsewhere.tenant_id = "00000000-0000-0000-0000-000000000000".to_string();
    assert!(ids_of(check_request(&elsewhere, &request, &delivery()))
        .contains(&"request.wrong-tenant".to_string()));
    // A forged code.
    let mut forged = request.clone();
    forged.code = "AAAA-AAAA".to_string();
    assert!(ids_of(check_request(&installed, &forged, &delivery()))
        .contains(&"request.code".to_string()));
}

#[test]
fn the_final_assembly_refuses_what_the_importer_would() {
    let v2 = v2_world();
    let installed = installed(&v2);
    let request =
        approval_request(&installed, configuration(), &revision()).unwrap();
    let checked = check_request(&installed, &request, &delivery()).unwrap();
    let two = || {
        vec![
            approval(&request, &v2.approver_1, "approver-1"),
            approval(&request, &v2.approver_2, "approver-2"),
        ]
    };
    let refused = |approvals: Vec<Approval>, signer: FinalSigner| {
        ids(
            &assemble_manifest(&installed, &checked, approvals, &signer, now())
                .unwrap_err(),
        )
    };
    // One approval only.
    assert!(refused(two()[..1].to_vec(), custodian())
        .contains(&"package.too-few-approvals".to_string()));
    // An approver can't make the final signature (role).
    let as_approver = FinalSigner {
        signer_id: "approver-1".to_string(),
        ..custodian()
    };
    assert!(refused(two(), as_approver)
        .contains(&"package.final-signer-role".to_string()));
    // Nobody enrolled.
    let stranger = FinalSigner {
        signer_id: "nobody".to_string(),
        ..custodian()
    };
    assert!(refused(two(), stranger)
        .contains(&"package.final-signer-not-enrolled".to_string()));
    // The same person approving twice.
    let twice = vec![
        approval(&request, &v2.approver_1, "approver-1"),
        approval(&request, &v2.approver_1, "approver-1"),
    ];
    assert!(refused(twice, custodian())
        .contains(&"package.duplicate-signer".to_string()));
}

#[test]
fn the_installed_policy_must_be_a_complete_format_2_one() {
    let v2 = v2_world();
    let good = serde_json::to_string(&installed(&v2)).unwrap();
    assert!(InstalledSigning::read(&good).is_ok());
    let mut legacy = installed(&v2);
    legacy.signing.minimum_package_format = None;
    legacy.signing.signers.clear();
    let found = ids(&InstalledSigning::read(
        &serde_json::to_string(&legacy).unwrap(),
    )
    .unwrap_err());
    assert!(
        found.contains(&"signing-policy.not-v2".to_string()),
        "{found:?}"
    );
    assert!(InstalledSigning::read(
        "{\"tenant_id\": \"x\", \"signing\": {}, \"extra\": 1}"
    )
    .is_err());
}

/// Assembly re-checks the request against the installed policy it is given:
/// a request checked elsewhere, or altered, doesn't assemble or package.
#[test]
fn assembly_refuses_a_request_for_another_tenant_policy_configuration_or_content(
) {
    let v2 = v2_world();
    let installed = installed(&v2);
    let request =
        approval_request(&installed, configuration(), &revision()).unwrap();
    let checked = check_request(&installed, &request, &delivery()).unwrap();
    let approvals = || {
        vec![
            approval(&request, &v2.approver_1, "approver-1"),
            approval(&request, &v2.approver_2, "approver-2"),
        ]
    };
    let refused_by = |other: &InstalledSigning, id: &str| {
        let found = ids(&assemble_manifest(
            other,
            &checked,
            approvals(),
            &custodian(),
            now(),
        )
        .unwrap_err());
        assert!(found.contains(&id.to_string()), "{id}: {found:?}");
    };
    let mut elsewhere = installed.clone();
    elsewhere.tenant_id = "00000000-0000-0000-0000-000000000000".to_string();
    refused_by(&elsewhere, "request.wrong-tenant");
    let mut other_policy = installed.clone();
    other_policy.signing.required_approvals = 1;
    refused_by(&other_policy, "request.policy-changed");

    // A request altered after it was made: its configuration or its content.
    let check_altered = |altered: ApprovalRequest, id: &str| {
        let found =
            ids(&check_request(&installed, &altered, &delivery()).unwrap_err());
        assert!(found.contains(&id.to_string()), "{id}: {found:?}");
    };
    let mut moved = request.clone();
    moved.configuration.revision = 4;
    check_altered(moved, "request.configuration");
    let mut other_config = request.clone();
    other_config.configuration.external_id = "another".to_string();
    check_altered(other_config, "request.configuration");
    let mut other_content = request.clone();
    other_content.content.reports[0].copies = 9;
    check_altered(other_content, "request.content-digest");

    // Packaging: a signature over another manifest doesn't come out.
    let manifest = assemble_manifest(
        &installed,
        &checked,
        approvals(),
        &custodian(),
        now(),
    )
    .unwrap();
    let bytes = manifest.to_bytes().unwrap();
    let mut forged = manifest.clone();
    forged.produced.custodian = "someone else".to_string();
    let forged_bytes = forged.to_bytes().unwrap();
    assert!(assemble_verified_package(
        &installed,
        &checked,
        &forged_bytes,
        &(v2.custodian.sign)(&bytes),
        &v2.custodian.chain_pem,
        now(),
    )
    .is_err());
    // And packaging under another installed policy fails the full check.
    assert!(assemble_verified_package(
        &other_policy,
        &checked,
        &bytes,
        &(v2.custodian.sign)(&bytes),
        &v2.custodian.chain_pem,
        now(),
    )
    .is_err());
}

/// What an approver's page checks before handing out an approval file.
#[test]
fn one_approval_is_checked_as_the_importer_will_count_it() {
    let v2 = v2_world();
    let installed = installed(&v2);
    let request =
        approval_request(&installed, configuration(), &revision()).unwrap();
    let checked = check_request(&installed, &request, &delivery()).unwrap();
    let good = approval(&request, &v2.approver_1, "approver-1");
    assert_eq!(
        check_approval(&installed, &checked, &good, now()).unwrap(),
        "approver-1"
    );
    let refused = |a: Approval| {
        ids(&check_approval(&installed, &checked, &a, now()).unwrap_err())
    };
    // Signed with approver-1's key, naming approver-2.
    assert!(refused(approval(&request, &v2.approver_1, "approver-2"))
        .contains(&"package.approver-id-mismatch".to_string()));
    // The custodian's key isn't an approver's (its chain isn't under the staff root either).
    assert!(!refused(approval(&request, &v2.custodian, "custodian")).is_empty());
    // A signature over something else.
    let mut tampered = good.clone();
    tampered.signature = STANDARD.encode(b"not a signature");
    assert!(refused(tampered).contains(&"package.approval-invalid".to_string()));
    // Expired by now.
    let expired = ecdsa(
        "DEMO Approver Old",
        &v2.world.staff_root,
        81,
        (2026, 11, 13),
    );
    let mut with_expired = installed.clone();
    with_expired.signing.signers[0] =
        enrolled("approver-1", &[SignerRole::Approver], &[&expired]);
    let request2 =
        approval_request(&with_expired, configuration(), &revision()).unwrap();
    let checked2 =
        check_request(&with_expired, &request2, &delivery()).unwrap();
    let found = ids(&check_approval(
        &with_expired,
        &checked2,
        &approval(&request2, &expired, "approver-1"),
        now(),
    )
    .unwrap_err());
    assert!(
        found.contains(&"package.approval-invalid".to_string()),
        "{found:?}"
    );
}
