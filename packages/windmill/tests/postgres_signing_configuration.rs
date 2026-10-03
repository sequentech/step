// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The signing configuration of an election event against the database: what
//! an import saves and logs, what an export reads back (and a second event
//! imports identically), the `miru:area-threshold` mapping, and the
//! transmission threshold. Two configurations (the janitor's client preset
//! and the student-council fixture) are imported side by side, with every
//! expectation taken from their files.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing_pki.rs"]
mod signing_pki;

use deadpool_postgres::Transaction;
use sequent_core::election_config::ImportElectionEventSchema;
use sequent_core::signing::{
    RequesterSigning, SigningAction, SigningChecks, SigningRequirement, SigningRule,
};
use sequent_core::types::hasura::core::Area;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::postgres::signing::{get_signing_checks, get_signing_rule};
use windmill::services::electoral_log::ElectoralLogAdminContext;
use windmill::services::import::import_election_event::get_election_event_schema;
use windmill::services::signing::configuration::{
    event_transmission_threshold, export_bundle_signing, export_signing_configuration,
    import_bundle_signing, import_bundle_staff_issuers, import_signing_configuration,
};
use windmill::services::signing::log::Actor;

const JANITOR_PRESET: &str = include_str!("../external-bin/janitor/templates/COMELEC/signing.json");
const STUDENT_COUNCIL: &str =
    include_str!("../../../scripts/dev/scenario/signing-organizations/student-council.json");

#[derive(Deserialize)]
struct Preset {
    signing_rules: Vec<SigningRule>,
    signing_checks: SigningChecks,
}

#[derive(Clone, Copy)]
struct Scope {
    tenant: Uuid,
    event: Uuid,
}

fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

fn importer() -> Actor {
    Actor {
        user_id: "importer-id".to_owned(),
        username: "importer".to_owned(),
    }
}

async fn tenant(tx: &Transaction<'_>, tenant: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
}

async fn event(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
}

async fn scope(tx: &Transaction<'_>, seed: u32) -> Scope {
    let s = Scope {
        tenant: id(seed, 1),
        event: id(seed, 2),
    };
    tenant(tx, s.tenant).await;
    event(tx, s.tenant, s.event).await;
    s
}

/// Another event of the same tenant.
async fn second_event(tx: &Transaction<'_>, s: Scope, seed: u32) -> Scope {
    let other = Scope {
        event: id(seed, 12),
        ..s
    };
    event(tx, s.tenant, other.event).await;
    other
}

fn area(s: Scope, threshold: &str) -> Area {
    serde_json::from_value(json!({
        "id": Uuid::new_v4().to_string(),
        "tenant_id": s.tenant.to_string(),
        "election_event_id": s.event.to_string(),
        "annotations": {"miru:area-threshold": threshold},
    }))
    .unwrap()
}

fn rules(value: Value) -> Vec<SigningRule> {
    serde_json::from_value(value).unwrap()
}

/// The outbox entries of `kind` for the event, as (entry, user_id, body).
async fn logged(tx: &Transaction<'_>, s: Scope, kind: &str) -> Vec<(i16, Option<String>, Value)> {
    tx.query(
        "SELECT entry, user_id, body FROM sequent_backend.signing_log_outbox
         WHERE tenant_id = $1 AND election_event_id = $2 AND statement_kind = $3
         ORDER BY id",
        &[&s.tenant, &s.event, &kind],
    )
    .await
    .unwrap()
    .into_iter()
    .map(|row| {
        let body: Value = row.get("body");
        (row.get("entry"), row.get("user_id"), body)
    })
    .collect()
}

/// Rules and checks imported into one event export as they were imported,
/// and a second event importing that export exports the same again.
#[tokio::test]
async fn an_exported_configuration_imports_into_another_event_unchanged() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    let s = scope(tx, 0x5c13_0001).await;
    let imported = rules(json!([
        {"action": "open-voting", "requirement": "required", "signatures": 2,
         "requester_signing": "allowed"},
        {"action": "approve-voter", "requirement": "required", "signatures": 1,
         "requester_signing": "not-allowed", "expires_minutes": null, "revision": 9},
        {"action": "transmit-results", "requirement": "not-required",
         "signatures": 1, "requester_signing": "not-allowed", "expires_minutes": 30}
    ]));
    let checks: SigningChecks = serde_json::from_value(json!({
        "revocation_check": "dont-check", "crl_unavailable": "accept-unchecked",
        "registration": "security-officer-only", "post_binding": "any-post"
    }))
    .unwrap();
    import_signing_configuration(
        tx,
        s.tenant,
        s.event,
        Some(&imported),
        Some(&checks),
        &[],
        &importer(),
    )
    .await
    .unwrap();

    let first = export_signing_configuration(tx, s.tenant, s.event)
        .await
        .unwrap();
    let exported_rules = first.rules.clone().unwrap();
    assert_eq!(exported_rules.len(), 3);
    let open = exported_rules
        .iter()
        .find(|rule| rule.action == SigningAction::OpenVoting)
        .unwrap();
    // Absent expiry: the default hour. The event starts at revision 1
    // whatever the file said.
    assert_eq!(open.expires_minutes, Some(60));
    assert_eq!(open.revision, 1);
    let voter = exported_rules
        .iter()
        .find(|rule| rule.action == SigningAction::ApproveVoter)
        .unwrap();
    assert_eq!(voter.expires_minutes, None);
    assert_eq!(voter.revision, 1);
    let exported_checks = first.checks.clone().unwrap();
    assert_eq!(
        SigningChecks {
            revision: 0,
            ..exported_checks.clone()
        },
        checks
    );

    // Through the bundle's JSON, into a second event.
    let text = serde_json::to_string(&json!({
        "signing_rules": first.rules, "signing_checks": first.checks,
    }))
    .unwrap();
    let parsed: Preset = serde_json::from_str(&text).unwrap();
    let other = second_event(tx, s, 0x5c13_0001).await;
    import_signing_configuration(
        tx,
        other.tenant,
        other.event,
        Some(&parsed.signing_rules),
        Some(&parsed.signing_checks),
        &[],
        &importer(),
    )
    .await
    .unwrap();
    let second = export_signing_configuration(tx, other.tenant, other.event)
        .await
        .unwrap();
    assert_eq!(second, first);
}

/// Every imported rule and the checks leave their USER and SYSTEM entries,
/// attributed to the importer.
#[tokio::test]
async fn an_import_logs_each_rule_and_the_checks_as_the_importer() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    let s = scope(tx, 0x5c13_0002).await;
    let imported = rules(json!([
        {"action": "close-voting", "requirement": "required", "signatures": 2,
         "requester_signing": "allowed"},
        {"action": "generate-reports", "requirement": "required", "signatures": 2,
         "requester_signing": "allowed"}
    ]));
    import_signing_configuration(
        tx,
        s.tenant,
        s.event,
        Some(&imported),
        Some(&SigningChecks::default()),
        &[],
        &importer(),
    )
    .await
    .unwrap();
    let rule_entries = logged(tx, s, "SigningRuleChanged").await;
    assert_eq!(rule_entries.len(), 4);
    assert_eq!(
        rule_entries
            .iter()
            .map(|(entry, _, _)| *entry)
            .collect::<Vec<_>>(),
        vec![0, 1, 0, 1]
    );
    for (entry, user, body) in &rule_entries {
        // The USER entry names the importer; the SYSTEM one nobody.
        let expected = (*entry == 0).then(|| "importer-id".to_owned());
        assert_eq!(user, &expected);
        assert_eq!(body["details"]["source"], "import");
    }
    assert_eq!(rule_entries[0].2["details"]["action"], "close-voting");
    assert_eq!(rule_entries[0].2["details"]["new"]["signatures"], 2);
    assert_eq!(logged(tx, s, "SigningChecksChanged").await.len(), 2);
    let checks = get_signing_checks(tx, s.tenant, s.event)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(checks.updated_by, "importer-id");
}

/// A bundle without signing configuration (and without thresholds) saves
/// nothing, logs nothing, and exports nothing.
#[tokio::test]
async fn a_bundle_without_configuration_imports_nothing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    let s = scope(tx, 0x5c13_0003).await;
    import_signing_configuration(
        tx,
        s.tenant,
        s.event,
        None,
        None,
        &[area(s, "0")],
        &importer(),
    )
    .await
    .unwrap();
    let exported = export_signing_configuration(tx, s.tenant, s.event)
        .await
        .unwrap();
    assert_eq!(exported.rules, None);
    assert_eq!(exported.checks, None);
    assert!(logged(tx, s, "SigningRuleChanged").await.is_empty());
}

/// 2025-style bundles keep their minimum: the largest threshold becomes the
/// transmit-results rule, logged as coming from the thresholds.
#[tokio::test]
async fn area_thresholds_become_the_transmit_rule_on_import() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    let s = scope(tx, 0x5c13_0004).await;
    let areas = [area(s, "3"), area(s, "3"), area(s, "0")];
    import_signing_configuration(tx, s.tenant, s.event, None, None, &areas, &importer())
        .await
        .unwrap();
    let saved = get_signing_rule(tx, s.tenant, s.event, SigningAction::TransmitResults)
        .await
        .unwrap()
        .unwrap()
        .rule;
    assert_eq!(saved.requirement, SigningRequirement::Required);
    assert_eq!(saved.signatures, 3);
    assert_eq!(saved.requester_signing, RequesterSigning::Allowed);
    assert_eq!(saved.expires_minutes, None);
    let entries = logged(tx, s, "SigningRuleChanged").await;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].2["details"]["source"], "area-threshold");
    // And the send reads it.
    assert_eq!(
        event_transmission_threshold(tx, s.tenant, s.event, 0)
            .await
            .unwrap(),
        3
    );
}

/// Posts asking for different counts keep their own: no rule is saved and
/// each package keeps its Post's threshold.
#[tokio::test]
async fn differing_area_thresholds_keep_each_posts_own() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    let s = scope(tx, 0x5c13_000b).await;
    let areas = [area(s, "2"), area(s, "3")];
    import_signing_configuration(tx, s.tenant, s.event, None, None, &areas, &importer())
        .await
        .unwrap();
    assert!(
        get_signing_rule(tx, s.tenant, s.event, SigningAction::TransmitResults)
            .await
            .unwrap()
            .is_none()
    );
    assert!(logged(tx, s, "SigningRuleChanged").await.is_empty());
    for threshold in [2, 3] {
        assert_eq!(
            event_transmission_threshold(tx, s.tenant, s.event, threshold)
                .await
                .unwrap(),
            threshold
        );
    }
}

/// An imported transmit rule wins over the thresholds; zero thresholds map
/// to nothing.
#[tokio::test]
async fn an_explicit_rule_or_zero_thresholds_leave_the_mapping_out() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    let s = scope(tx, 0x5c13_0005).await;
    let explicit = rules(json!([
        {"action": "transmit-results", "requirement": "not-required",
         "signatures": 1, "requester_signing": "not-allowed"}
    ]));
    import_signing_configuration(
        tx,
        s.tenant,
        s.event,
        Some(&explicit),
        None,
        &[area(s, "3")],
        &importer(),
    )
    .await
    .unwrap();
    let saved = get_signing_rule(tx, s.tenant, s.event, SigningAction::TransmitResults)
        .await
        .unwrap()
        .unwrap()
        .rule;
    assert_eq!(saved.requirement, SigningRequirement::NotRequired);
    // The rule is off, so the Post's annotation still decides.
    assert_eq!(
        event_transmission_threshold(tx, s.tenant, s.event, 3)
            .await
            .unwrap(),
        3
    );

    let other = second_event(tx, s, 0x5c13_0005).await;
    import_signing_configuration(
        tx,
        other.tenant,
        other.event,
        Some(&[]),
        None,
        &[area(other, "0"), area(other, "0")],
        &importer(),
    )
    .await
    .unwrap();
    assert!(get_signing_rule(
        tx,
        other.tenant,
        other.event,
        SigningAction::TransmitResults
    )
    .await
    .unwrap()
    .is_none());
    assert_eq!(
        event_transmission_threshold(tx, other.tenant, other.event, 0)
            .await
            .unwrap(),
        0
    );
}

/// The send reads a required transmit rule's count, not the annotation.
#[tokio::test]
async fn the_transmission_threshold_is_read_from_the_saved_rule() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    let s = scope(tx, 0x5c13_0006).await;
    assert_eq!(
        event_transmission_threshold(tx, s.tenant, s.event, 4)
            .await
            .unwrap(),
        4
    );
    let required = rules(json!([
        {"action": "transmit-results", "requirement": "required", "signatures": 2,
         "requester_signing": "allowed"}
    ]));
    import_signing_configuration(
        tx,
        s.tenant,
        s.event,
        Some(&required),
        None,
        &[],
        &importer(),
    )
    .await
    .unwrap();
    assert_eq!(
        event_transmission_threshold(tx, s.tenant, s.event, 4)
            .await
            .unwrap(),
        2
    );
}

/// Invalid rules refuse the import rather than being saved or skipped.
#[tokio::test]
async fn invalid_rules_refuse_the_import() {
    for (seed, bad) in [
        (
            0x5c13_0007,
            json!([
                {"action": "open-voting", "requirement": "required", "signatures": 2,
                 "requester_signing": "allowed"},
                {"action": "open-voting", "requirement": "required", "signatures": 3,
                 "requester_signing": "allowed"}
            ]),
        ),
        (
            0x5c13_0008,
            json!([{"action": "open-voting", "requirement": "required", "signatures": 0,
                    "requester_signing": "allowed"}]),
        ),
        (
            0x5c13_0009,
            json!([{"action": "open-voting", "requirement": "required", "signatures": 1,
                    "requester_signing": "allowed", "expires_minutes": 0}]),
        ),
    ] {
        let mut client = schema::pool().await.get().await.unwrap();
        let tx = &client.transaction().await.unwrap();
        let s = scope(tx, seed).await;
        let error = import_signing_configuration(
            tx,
            s.tenant,
            s.event,
            Some(&rules(bad)),
            None,
            &[],
            &importer(),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("open-voting"), "{error:#}");
    }
}

/// The janitor's client preset and the student council, imported into two
/// events: each exports what its own file says.
#[tokio::test]
async fn two_organizations_import_their_own_configuration() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    let s = scope(tx, 0x5c13_000a).await;
    let council_event = second_event(tx, s, 0x5c13_000a).await;
    for (scope, text) in [(s, JANITOR_PRESET), (council_event, STUDENT_COUNCIL)] {
        let preset: Preset = serde_json::from_str(text).unwrap();
        import_signing_configuration(
            tx,
            scope.tenant,
            scope.event,
            Some(&preset.signing_rules),
            Some(&preset.signing_checks),
            &[],
            &importer(),
        )
        .await
        .unwrap();
        let exported = export_signing_configuration(tx, scope.tenant, scope.event)
            .await
            .unwrap();
        let mut expected: Vec<SigningRule> = preset
            .signing_rules
            .iter()
            .map(|rule| SigningRule {
                revision: 1,
                ..rule.clone()
            })
            .collect();
        expected.sort_by_key(|rule| rule.action.to_string());
        assert_eq!(exported.rules.unwrap(), expected);
        assert_eq!(
            exported.checks.map(|checks| SigningChecks {
                revision: 0,
                ..checks
            }),
            Some(preset.signing_checks)
        );
    }
    // The two differ, so neither passes by a value written in code.
    let preset_ers = get_signing_rule(
        tx,
        s.tenant,
        s.event,
        SigningAction::GenerateElectionReturns,
    )
    .await
    .unwrap()
    .unwrap()
    .rule;
    let council_ers = get_signing_rule(
        tx,
        council_event.tenant,
        council_event.event,
        SigningAction::GenerateElectionReturns,
    )
    .await
    .unwrap()
    .unwrap()
    .rule;
    assert_ne!(preset_ers.signatures, council_ers.signatures);
}

const BUNDLE_VERSION: &str = "v10.0.0";

/// A small sound bundle for `s`'s event (one election, contest, two
/// candidates and an area), with `extra` top-level fields.
fn bundle(s: Scope, extra: Value) -> Value {
    let (tenant, event) = (s.tenant.to_string(), s.event.to_string());
    let election = "e1000000-0000-4000-8000-000000000000";
    let contest = "c1000000-0000-4000-8000-000000000000";
    let area = "a1000000-0000-4000-8000-000000000000";
    let candidate = |id: &str, external: &str| {
        json!({"id": id, "tenant_id": tenant, "election_event_id": event,
               "contest_id": contest, "external_id": external})
    };
    let mut bundle = json!({
        "tenant_id": tenant,
        "keycloak_event_realm": null,
        "election_event": {
            "id": event, "tenant_id": tenant,
            "is_archived": false, "encryption_protocol": "RSA256"
        },
        "elections": [{"id": election, "tenant_id": tenant, "election_event_id": event,
                       "external_id": "officers"}],
        "contests": [{"id": contest, "tenant_id": tenant, "election_event_id": event,
                      "election_id": election, "external_id": "president",
                      "min_votes": 0, "max_votes": 1, "winning_candidates_num": 1,
                      "voting_type": "non-preferential",
                      "counting_algorithm": "plurality-at-large"}],
        "candidates": [
            candidate("d1000000-0000-4000-8000-000000000000", "pres-a"),
            candidate("d2000000-0000-4000-8000-000000000000", "pres-b")
        ],
        "areas": [{"id": area, "tenant_id": tenant, "election_event_id": event,
                   "name": "North Region"}],
        "area_contests": [{"id": "b1000000-0000-4000-8000-000000000000",
                           "area_id": area, "contest_id": contest}],
        "scheduled_events": null, "reports": [], "keys_ceremonies": [],
        "applications": [],
        "version": BUNDLE_VERSION,
    });
    for (key, value) in extra.as_object().unwrap() {
        bundle[key] = value.clone();
    }
    bundle
}

/// Reads a bundle the way an import does: version check, validation, new ids.
async fn read_bundle(text: &str, into: Scope) -> anyhow::Result<ImportElectionEventSchema> {
    std::env::set_var("APP_VERSION", BUNDLE_VERSION);
    get_election_event_schema(text, Some(into.event.to_string()), into.tenant.to_string())
        .await
        .map(|(bundle, _)| bundle)
}

/// Through the functions the export and the import call: an event's
/// configuration exported into a bundle's JSON, read back as an import reads
/// it, and saved on another event, exports the same.
#[tokio::test]
async fn a_bundle_carries_the_configuration_from_one_event_to_another() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    let s = scope(tx, 0x5c13_000c).await;
    let source = read_bundle(
        &bundle(
            s,
            json!({
                "signing_rules": [
                    {"action": "close-voting", "requirement": "required", "signatures": 2,
                     "requester_signing": "allowed"},
                    {"action": "approve-voter", "requirement": "required", "signatures": 1,
                     "requester_signing": "not-allowed", "expires_minutes": null}
                ],
                "signing_checks": {"revocation_check": "dont-check",
                    "crl_unavailable": "accept-unchecked",
                    "registration": "security-officer-only", "post_binding": "any-post"}
            }),
        )
        .to_string(),
        s,
    )
    .await
    .unwrap();
    let importer = ElectoralLogAdminContext {
        user_id: "importer-id".to_owned(),
        username: Some("importer".to_owned()),
        authorized_election_ids: None,
        area_id: None,
    };
    import_bundle_signing(tx, s.tenant, s.event, &source, Some(&importer))
        .await
        .unwrap();

    let mut exported: ImportElectionEventSchema =
        serde_json::from_value(bundle(s, json!({}))).unwrap();
    export_bundle_signing(tx, s.tenant, s.event, &mut exported)
        .await
        .unwrap();
    let text = serde_json::to_string(&exported).unwrap();
    assert!(text.contains("\"signing_rules\""));

    let other = second_event(tx, s, 0x5c13_000c).await;
    let read = read_bundle(&text, other).await.unwrap();
    assert_eq!(read.election_event.id, other.event.to_string());
    import_bundle_signing(tx, other.tenant, other.event, &read, None)
        .await
        .unwrap();
    assert_eq!(
        export_signing_configuration(tx, other.tenant, other.event)
            .await
            .unwrap(),
        export_signing_configuration(tx, s.tenant, s.event)
            .await
            .unwrap()
    );
    // Without an importer, the system is named.
    let entries = logged(tx, other, "SigningRuleChanged").await;
    assert_eq!(entries[0].1.as_deref(), Some("system"));
}

/// A bundle whose rules can't be saved is refused while it is read, before
/// the import writes anything, and says why.
#[tokio::test]
async fn a_bundle_with_unsavable_rules_is_refused_before_any_write() {
    let s = Scope {
        tenant: id(0x5c13_000d, 1),
        event: id(0x5c13_000d, 2),
    };
    for (rules, why) in [
        (
            json!([{"action": "open-voting", "requirement": "required", "signatures": 2,
                    "requester_signing": "allowed"},
                   {"action": "open-voting", "requirement": "required", "signatures": 3,
                    "requester_signing": "allowed"}]),
            "more than one signing rule",
        ),
        (
            json!([{"action": "open-voting", "requirement": "required", "signatures": 101,
                    "requester_signing": "allowed"}]),
            "between 1 and 100 signatures",
        ),
    ] {
        let error = read_bundle(&bundle(s, json!({"signing_rules": rules})).to_string(), s)
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains(why), "{error:#}");
    }
}

#[tokio::test]
async fn a_bundle_staff_issuer_import_names_its_importer() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = &client.transaction().await.unwrap();
    for (seed, identity) in [
        (0x5c13_0010, Some("importing-officer")),
        (0x5c13_0011, None),
    ] {
        let s = scope(tx, seed).await;
        let importer = identity.map(|name| ElectoralLogAdminContext {
            user_id: name.to_owned(),
            username: Some(format!("{name}-name")),
            authorized_election_ids: None,
            area_id: None,
        });
        let import = import_bundle_staff_issuers(
            tx,
            s.tenant,
            s.event,
            &[signing_pki::Pki::get().root.cert.clone()],
            importer.as_ref(),
            chrono::Utc::now(),
        )
        .await
        .unwrap();
        assert!(import.errors.is_empty(), "{:?}", import.errors);
        assert_eq!(import.imported.len(), 1);
        let entries = logged(tx, s, "SigningIssuerChanged").await;
        assert_eq!(entries.len(), 2);
        let expected = identity.unwrap_or("system");
        assert_eq!(entries[0].1.as_deref(), Some(expected));
        assert_eq!(entries[1].1, None);
        assert_eq!(
            entries[0].2["details"]["allowed_by"],
            json!(["election-event-import"])
        );
    }
}
