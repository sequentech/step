// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic tests of the Keycloak policy observation seam. Every envelope
//! is synthetic, built here; nothing is read from a live realm.

use super::super::tests::{fixture, ISSUER, START, SUBJECT, TENANT};
use super::super::{
    FlowPolicyProof, SecondFactorPolicy, MAX_MFA_AGE, PASSWORD_REFERENCE,
    TOTP_REFERENCE,
};
use super::*;
use crate::device::{DeviceScope, ScopeInput};
use serde_json::{json, Value};

const REALM_ID: &str = "00000000-0000-4000-8000-0000000000d4";
const REALM_NAME: &str = "tenant-00000000-0000-4000-8000-00000000000a";
const CLIENT: &str = "synthetic-device-client";
const PORTAL: &str = "synthetic-portal";
const FLOW: &str = "synthetic-device-flow";
const OBSERVED_AT: UnixSeconds = 1_800_000_070;
const OTHER_SUBJECT: &str = "00000000-0000-4000-8000-0000000000e6";
/// The nonce of the synthetic after-tokens request the template answers.
const NONCE: &str = "synthetic-observation-request-nonce-0000000";
/// The fixture tokens' session: an earlier ordinary SSO session the device
/// login reused, as the stock fixture showed.
const SESSION: &str = "synthetic-ordinary-sso-session";
/// The nonce this runner sent with the challenge's device request.
const DEVICE_NONCE: &str = "synthetic-device-request-nonce-000000000000";

/// The request the template answers: after the tokens, challenge 1.
fn request() -> ObservationRequest {
    ObservationRequest {
        binding: RequestBinding {
            nonce: NONCE.to_owned(),
            phase: Phase::AfterTokens,
            challenge: ChallengeId(1),
        },
    }
}

fn expected() -> ExpectedInstall {
    ExpectedInstall {
        issuer: ISSUER.to_owned(),
        realm_id: REALM_ID.to_owned(),
        client_id: CLIENT.to_owned(),
        flow_alias: FLOW.to_owned(),
        priorities: [10, 20],
    }
}

fn policy() -> ClaimPolicy {
    let scope = DeviceScope::new(ScopeInput {
        issuer: ISSUER,
        audience: "synthetic-ceremony-audience",
        client_id: CLIENT,
        subject: SUBJECT,
        tenant: TENANT,
        trustee_name: "synthetic-trustee-a",
        required_role: "trustee-ceremony",
    })
    .unwrap();
    ClaimPolicy::from_scope(&scope).unwrap()
}

fn amr_config(reference: &str, max_age: &str) -> Value {
    json!({
        "default.reference.value": reference,
        "default.reference.maxAge": max_age,
    })
}

/// A complete observation of the qualifying installation, with a portal
/// client on the stock browser flow and its own unrelated mappers.
fn envelope() -> Value {
    json!({
        "schema": SCHEMA,
        "request": { "nonce": NONCE, "phase": "after_tokens", "challenge": 1 },
        "collector": { "observed_at": OBSERVED_AT, "keycloak_release": "26.7.5" },
        "realm": {
            "issuer": ISSUER,
            "id": REALM_ID,
            "name": REALM_NAME,
            "browser_flow": "browser",
            "otp_policy": {
                "kind": "totp",
                "algorithm": "HmacSHA1",
                "digits": 6,
                "period": 30,
                "look_ahead_window": 1,
                "code_reusable": false,
            },
        },
        "required_actions": {
            "completeness": "complete",
            "items": [
                {
                    "alias": "UPDATE_PASSWORD",
                    "provider_id": "UPDATE_PASSWORD",
                    "enabled": true,
                    "default_action": false,
                    "priority": 30,
                },
                {
                    "alias": "CONFIGURE_TOTP",
                    "provider_id": "CONFIGURE_TOTP",
                    "enabled": true,
                    "default_action": false,
                    "priority": 10,
                },
            ],
        },
        "subject": {
            "completeness": "complete",
            "user_id": SUBJECT,
            "otp_credentials": [
                { "id": "synthetic-otp-credential-1", "created_at": OBSERVED_AT - 86_400 },
            ],
            "pending_required_actions": [],
        },
        "clients": {
            "completeness": "complete",
            "items": [
                {
                    "client_id": CLIENT,
                    "public_client": false,
                    "bearer_only": false,
                    "grants": ["device_authorization"],
                    "browser_flow_override": FLOW,
                },
                {
                    "client_id": PORTAL,
                    "public_client": true,
                    "bearer_only": false,
                    "grants": ["standard_flow"],
                    "browser_flow_override": null,
                },
            ],
        },
        "flows": {
            "completeness": "complete",
            "items": [
                {
                    "alias": "browser",
                    "top_level": true,
                    "provider_id": "basic-flow",
                    "executions": [{
                        "priority": 10,
                        "requirement": "ALTERNATIVE",
                        "authenticator": "auth-cookie",
                    }],
                },
                {
                    "alias": FLOW,
                    "top_level": true,
                    "provider_id": "basic-flow",
                    "executions": [
                        {
                            "priority": 10,
                            "requirement": "REQUIRED",
                            "authenticator": "auth-username-password-form",
                            "config": amr_config(PASSWORD_REFERENCE, "60"),
                        },
                        {
                            "priority": 20,
                            "requirement": "REQUIRED",
                            "authenticator": "auth-otp-form",
                            "config": amr_config(TOTP_REFERENCE, "60"),
                        },
                    ],
                },
            ],
        },
        "mappers": {
            "completeness": "complete",
            "items": [
                {
                    "client_id": CLIENT,
                    "provider_id": "oidc-amr-mapper",
                    "host": "client",
                    "claim_name": "amr",
                    "id_token": true,
                    "access_token": true,
                },
                {
                    "client_id": CLIENT,
                    "provider_id": "oidc-usermodel-attribute-mapper",
                    "host": { "client_scope": "profile" },
                    "claim_name": "preferred_username",
                    "id_token": true,
                    "access_token": true,
                },
                {
                    "client_id": PORTAL,
                    "provider_id": "oidc-hardcoded-claim-mapper",
                    "host": "client",
                    "claim_name": "amr",
                    "id_token": true,
                    "access_token": true,
                },
            ],
        },
    })
}

fn authenticated(body: Vec<u8>) -> AuthenticatedObservation {
    AuthenticatedObservation {
        issuer_binding: ISSUER.to_owned(),
        body,
    }
}

fn observed(
    edit: impl FnOnce(&mut Value),
) -> Result<DriftEvidence, ObservationError> {
    let mut value = envelope();
    edit(&mut value);
    observe(
        &expected(),
        &authenticated(serde_json::to_vec(&value).unwrap()),
        &request(),
    )
}

fn refusal(edit: impl FnOnce(&mut Value)) -> Option<ObservationError> {
    observed(edit).err()
}

type Edit = (&'static str, Box<dyn FnOnce(&mut Value)>);

/// A named edit of the envelope.
fn case(name: &'static str, edit: impl FnOnce(&mut Value) + 'static) -> Edit {
    (name, Box::new(edit))
}

/// The dedicated flow's executions.
fn executions(value: &mut Value) -> &mut Vec<Value> {
    value["flows"]["items"][1]["executions"]
        .as_array_mut()
        .unwrap()
}

#[test]
fn an_exact_complete_observation_is_the_qualifying_installed_shape() {
    let evidence = observed(|_| {}).unwrap();
    assert!(evidence.readback == Readback::Accepted);
    assert_eq!(evidence.checked_at, OBSERVED_AT);
    let shape = &evidence.observed;
    assert_eq!(shape.qualifying_max_age(&policy()), Some(MAX_MFA_AGE));
    assert_eq!(shape.client.browser_flow, FLOW);
    assert_eq!(shape.client.flow_bound_clients, vec![CLIENT.to_owned()]);
    // Only the device client's `amr` writer: the portal's is not its.
    assert_eq!(shape.amr_mappers.len(), 1);

    // Installed as observed: it binds as proof, with the flow's own age.
    let proof = FlowPolicyProof {
        receipt: evidence.observed.clone(),
        drift: evidence,
    };
    assert_eq!(
        proof.bind(&policy()).map(|bound| bound.max_age).ok(),
        Some(MAX_MFA_AGE)
    );

    // The order the collector lists things in never matters.
    let reordered = observed(|v| {
        v["clients"]["items"].as_array_mut().unwrap().reverse();
        v["flows"]["items"].as_array_mut().unwrap().reverse();
        v["mappers"]["items"].as_array_mut().unwrap().reverse();
    })
    .unwrap();
    let exact = observed(|_| {}).unwrap();
    assert!(reordered.observed == exact.observed);
    assert_eq!(
        reordered.observed.fingerprint(),
        exact.observed.fingerprint()
    );
}

#[test]
fn a_partial_or_unreadable_section_is_no_evidence() {
    for section in ["required_actions", "clients", "flows", "mappers"] {
        assert_eq!(
            refusal(|v| v[section]["completeness"] = json!("partial")),
            Some(ObservationError::Incomplete),
            "{section}"
        );
        assert_eq!(
            refusal(|v| v[section]["completeness"] = json!("read_denied")),
            Some(ObservationError::ReadDenied),
            "{section}"
        );
        assert_eq!(
            refusal(|v| v[section]["completeness"] = json!("stale")),
            Some(ObservationError::Schema),
            "{section}"
        );
        assert_eq!(
            refusal(|v| drop(
                v[section].as_object_mut().unwrap().remove("completeness")
            )),
            Some(ObservationError::Schema),
            "{section}"
        );
    }
}

#[test]
fn refuses_anything_but_the_closed_schema() {
    let edits: Vec<Edit> = vec![
        case("schema id", |v| v["schema"] = json!("synthetic.other.v1")),
        case("top-level extra", |v| v["synthetic_extra"] = json!(true)),
        case("collector extra", |v| v["collector"]["note"] = json!("x")),
        case("collector missing", |v| {
            v.as_object_mut().unwrap().remove("collector");
        }),
        case("client extra", |v| {
            v["clients"]["items"][0]["secret"] = json!("x")
        }),
        case("unknown grant", |v| {
            v["clients"]["items"][0]["grants"] = json!(["password"]);
        }),
        case("execution extra", |v| {
            executions(v)[0]["flow_id"] = json!("x")
        }),
        case("unknown requirement", |v| {
            executions(v)[1]["requirement"] = json!("OPTIONAL");
        }),
        case("numeric age", |v| {
            executions(v)[1]["config"]["default.reference.maxAge"] = json!(60);
        }),
        case("unknown host", |v| {
            v["mappers"]["items"][0]["host"] = json!("realm")
        }),
        case("mapper extra", |v| {
            v["mappers"]["items"][0]["config"] = json!({})
        }),
        case("unknown otp algorithm", |v| {
            v["realm"]["otp_policy"]["algorithm"] = json!("HmacMD5");
        }),
        case("unknown otp kind", |v| {
            v["realm"]["otp_policy"]["kind"] = json!("sms")
        }),
        case("otp policy extra", |v| {
            v["realm"]["otp_policy"]["initial_counter"] = json!(0);
        }),
        case("otp policy missing", |v| {
            v["realm"].as_object_mut().unwrap().remove("otp_policy");
        }),
        case("required action extra", |v| {
            v["required_actions"]["items"][0]["config"] = json!({});
        }),
        case("required actions missing", |v| {
            v.as_object_mut().unwrap().remove("required_actions");
        }),
        case("subject extra", |v| v["subject"]["email"] = json!("x")),
    ];
    for (name, edit) in edits {
        assert_eq!(refusal(edit), Some(ObservationError::Schema), "{name}");
    }

    // A repeated member, whatever order the serializer keeps.
    let body = serde_json::to_string(&envelope()).unwrap();
    let duplicate = body.replacen('{', "{\"schema\":\"x\",", 1);
    assert_ne!(duplicate, body);
    for raw in [duplicate.into_bytes(), b"synthetic".to_vec(), Vec::new()] {
        assert_eq!(
            observe(&expected(), &authenticated(raw), &request()).err(),
            Some(ObservationError::Schema)
        );
    }
    assert_eq!(
        observe(
            &expected(),
            &authenticated(vec![b' '; BODY_LIMIT + 1]),
            &request()
        )
        .err(),
        Some(ObservationError::TooLarge)
    );
}

#[test]
fn binds_the_observation_to_the_expected_issuer_and_realm() {
    let other_issuer =
        format!("{ISSUER}-event-00000000-0000-4000-8000-0000000000e5");
    let mut wrong_channel =
        authenticated(serde_json::to_vec(&envelope()).unwrap());
    wrong_channel.issuer_binding = other_issuer.clone();
    assert_eq!(
        observe(&expected(), &wrong_channel, &request()).err(),
        Some(ObservationError::Attestation)
    );
    assert_eq!(
        refusal(|v| v["realm"]["issuer"] = json!(other_issuer)),
        Some(ObservationError::Issuer)
    );
    for (field, value) in [
        ("id", "00000000-0000-4000-8000-0000000000e5"),
        ("id", "synthetic-realm"),
        ("id", "00000000-0000-4000-8000-0000000000D4"),
        ("name", "tenant-00000000-0000-4000-8000-0000000000e5"),
        ("name", "master"),
    ] {
        assert_eq!(
            refusal(|v| v["realm"][field] = json!(value)),
            Some(ObservationError::Realm),
            "{field} {value}"
        );
    }
}

#[test]
fn requires_the_device_client_and_flow_exactly_once() {
    let edits: Vec<Edit> = vec![
        case("no device client", |v| {
            v["clients"]["items"].as_array_mut().unwrap().remove(0);
        }),
        case("device client twice", |v| {
            let device = v["clients"]["items"][0].clone();
            v["clients"]["items"].as_array_mut().unwrap().push(device);
        }),
        case("portal twice", |v| {
            let portal = v["clients"]["items"][1].clone();
            v["clients"]["items"].as_array_mut().unwrap().push(portal);
        }),
        case("no dedicated flow", |v| {
            v["flows"]["items"].as_array_mut().unwrap().remove(1);
        }),
        case("dedicated flow twice", |v| {
            let flow = v["flows"]["items"][1].clone();
            v["flows"]["items"].as_array_mut().unwrap().push(flow);
        }),
    ];
    for (name, edit) in edits {
        assert_eq!(refusal(edit), Some(ObservationError::Inventory), "{name}");
    }
}

#[test]
fn requires_the_installed_priorities_in_order() {
    let edits: Vec<Edit> = vec![
        case("moved otp", |v| executions(v)[1]["priority"] = json!(30)),
        case("moved password", |v| {
            executions(v)[0]["priority"] = json!(5)
        }),
        case("equal", |v| executions(v)[1]["priority"] = json!(10)),
        case("reversed", |v| {
            executions(v)[0]["priority"] = json!(20);
            executions(v)[1]["priority"] = json!(10);
        }),
    ];
    for (name, edit) in edits {
        assert_eq!(refusal(edit), Some(ObservationError::Priority), "{name}");
    }
}

#[test]
fn refuses_contradictory_or_out_of_range_values() {
    let edits: Vec<Edit> = vec![
        case("public and bearer-only", |v| {
            v["clients"]["items"][0]["public_client"] = json!(true);
            v["clients"]["items"][0]["bearer_only"] = json!(true);
        }),
        case("repeated grant", |v| {
            v["clients"]["items"][0]["grants"] =
                json!(["device_authorization", "device_authorization"]);
        }),
        case("authenticator and subflow", |v| {
            executions(v)[0]["subflow_alias"] = json!("browser forms");
        }),
        case("neither", |v| {
            executions(v)[0]
                .as_object_mut()
                .unwrap()
                .remove("authenticator");
        }),
        case("age not digits", |v| {
            executions(v)[1]["config"] = amr_config(TOTP_REFERENCE, "6o");
        }),
        case("age too long", |v| {
            executions(v)[1]["config"] = amr_config(TOTP_REFERENCE, "1234567");
        }),
        case("empty age", |v| {
            executions(v)[1]["config"] = amr_config(TOTP_REFERENCE, "");
        }),
        case("unknown flow engine", |v| {
            v["flows"]["items"][1]["provider_id"] = json!("form-flow");
        }),
        case("spaced client id", |v| {
            v["clients"]["items"][1]["client_id"] = json!("synthetic portal");
        }),
        // The dedicated flow's configs hold the two reference keys only.
        case("dedicated config extra", |v| {
            executions(v)[1]["config"]["default.reference.extra"] = json!("x");
        }),
        case("dedicated config without age", |v| {
            executions(v)[1]["config"] =
                json!({ "default.reference.value": TOTP_REFERENCE });
        }),
    ];
    for (name, edit) in edits {
        assert_eq!(refusal(edit), Some(ObservationError::Malformed), "{name}");
    }
}

#[test]
fn observes_installed_shapes_that_do_not_qualify() {
    let receipt = observed(|_| {}).unwrap().observed;
    let edits: Vec<Edit> = vec![
        case("other release", |v| {
            v["collector"]["keycloak_release"] = json!("26.7.4");
        }),
        case("inherits the realm browser flow", |v| {
            v["clients"]["items"][0]["browser_flow_override"] = Value::Null;
        }),
        case("portal bound to the flow", |v| {
            v["clients"]["items"][1]["browser_flow_override"] = json!(FLOW);
        }),
        case("realm browser flow is the flow", |v| {
            v["realm"]["browser_flow"] = json!(FLOW);
        }),
        case("public device client", |v| {
            v["clients"]["items"][0]["public_client"] = json!(true);
        }),
        case("direct grant too", |v| {
            v["clients"]["items"][0]["grants"] =
                json!(["direct_access", "device_authorization"]);
        }),
        case("client flow", |v| {
            v["flows"]["items"][1]["provider_id"] = json!("client-flow");
        }),
        case("nested", |v| {
            v["flows"]["items"][1]["top_level"] = json!(false)
        }),
        case("cookie first", |v| {
            executions(v).insert(
                0,
                json!({
                    "priority": 5,
                    "requirement": "ALTERNATIVE",
                    "authenticator": "auth-cookie",
                }),
            );
        }),
        case("disabled cookie last", |v| {
            executions(v).push(json!({
                "priority": 30,
                "requirement": "DISABLED",
                "authenticator": "auth-cookie",
            }));
        }),
        case("identity provider", |v| {
            executions(v).push(json!({
                "priority": 30,
                "requirement": "ALTERNATIVE",
                "authenticator": "identity-provider-redirector",
            }));
        }),
        case("inherited forms", |v| {
            executions(v).push(json!({
                "priority": 30,
                "requirement": "ALTERNATIVE",
                "subflow_alias": "browser forms",
            }));
        }),
        case("otp skipped", |v| {
            executions(v).pop();
        }),
        case("otp alternative", |v| {
            executions(v)[1]["requirement"] = json!("ALTERNATIVE");
        }),
        case("otp conditional", |v| {
            executions(v)[1]["requirement"] = json!("CONDITIONAL");
        }),
        case("unknown authenticator", |v| {
            executions(v)[1]["authenticator"] =
                json!("synthetic-authenticator");
        }),
        case("no otp reference", |v| {
            executions(v)[1].as_object_mut().unwrap().remove("config");
        }),
        case("age over 60", |v| {
            executions(v)[0]["config"] = amr_config(PASSWORD_REFERENCE, "61");
            executions(v)[1]["config"] = amr_config(TOTP_REFERENCE, "61");
        }),
        case("generic reference", |v| {
            executions(v)[0]["config"] = amr_config("pwd", "60");
        }),
        case("mapper on a scope", |v| {
            v["mappers"]["items"][0]["host"] = json!({ "client_scope": "acr" });
        }),
        case("hard-coded amr on the device client", |v| {
            v["mappers"]["items"][2]["client_id"] = json!(CLIENT);
        }),
        case("attribute mapper writing amr", |v| {
            v["mappers"]["items"][1]["claim_name"] = json!("amr");
        }),
        case("amr not in access token", |v| {
            v["mappers"]["items"][0]["access_token"] = json!(false);
        }),
        case("no amr mapper", |v| {
            v["mappers"]["items"].as_array_mut().unwrap().remove(0);
        }),
        case("configure otp a default action", |v| {
            v["required_actions"]["items"][1]["default_action"] = json!(true);
        }),
        case("configure otp disabled", |v| {
            v["required_actions"]["items"][1]["enabled"] = json!(false);
        }),
        case("no configure otp", |v| {
            v["required_actions"]["items"]
                .as_array_mut()
                .unwrap()
                .remove(1);
        }),
        case("update password a default action", |v| {
            v["required_actions"]["items"][0]["default_action"] = json!(true);
        }),
        case("eight-digit otp", |v| {
            v["realm"]["otp_policy"]["digits"] = json!(8)
        }),
        case("hotp", |v| v["realm"]["otp_policy"]["kind"] = json!("hotp")),
        case("sha-256 otp", |v| {
            v["realm"]["otp_policy"]["algorithm"] = json!("HmacSHA256");
        }),
        case("long otp period", |v| {
            v["realm"]["otp_policy"]["period"] = json!(60)
        }),
        case("wide look-ahead", |v| {
            v["realm"]["otp_policy"]["look_ahead_window"] = json!(5);
        }),
        case("reusable otp codes", |v| {
            v["realm"]["otp_policy"]["code_reusable"] = json!(true);
        }),
    ];
    for (name, edit) in edits {
        let mut value = envelope();
        edit(&mut value);
        let observation = authenticated(serde_json::to_vec(&value).unwrap());
        let evidence = observe(&expected(), &observation, &request())
            .unwrap_or_else(|err| {
                panic!("{name}: {err:?}");
            });
        assert_eq!(
            evidence.observed.qualifying_max_age(&policy()),
            None,
            "{name}"
        );
        assert_eq!(
            facts(&expected(), &observation, &request(), &policy(), &receipt),
            PolicyFacts {
                observed: true,
                qualifies: false,
                matches_receipt: false,
            },
            "{name}"
        );
        // Nor does it bind, read back against the installed receipt.
        let proof = FlowPolicyProof {
            receipt: receipt.clone(),
            drift: evidence,
        };
        assert!(proof.bind(&policy()).is_err(), "{name}");
    }
}

#[test]
fn knows_the_amr_mapper_by_provider_id_alone() {
    // Keycloak's AMR mapper counts whatever its claim label says.
    let relabelled = observed(|v| {
        v["mappers"]["items"][0]["claim_name"] = json!("synthetic-label");
    })
    .unwrap();
    assert_eq!(
        relabelled.observed.qualifying_max_age(&policy()),
        Some(MAX_MFA_AGE)
    );
    // Another provider labelled `amr` is a foreign writer, never the mapper.
    let impostor = observed(|v| {
        v["mappers"]["items"][0]["provider_id"] =
            json!("oidc-hardcoded-claim-mapper");
    })
    .unwrap();
    assert!(impostor.observed.amr_mappers.is_empty());
    assert_eq!(
        impostor.observed.foreign_amr_writers,
        vec!["oidc-hardcoded-claim-mapper".to_owned()]
    );
    assert_eq!(impostor.observed.qualifying_max_age(&policy()), None);
}

#[test]
fn records_required_actions_by_unique_alias() {
    let evidence = observed(|_| {}).unwrap();
    let aliases: Vec<&str> = evidence
        .observed
        .required_actions
        .iter()
        .map(|action| action.alias.as_str())
        .collect();
    assert_eq!(aliases, ["CONFIGURE_TOTP", "UPDATE_PASSWORD"]);
    assert_eq!(
        refusal(|v| {
            let again = v["required_actions"]["items"][1].clone();
            v["required_actions"]["items"]
                .as_array_mut()
                .unwrap()
                .push(again);
        }),
        Some(ObservationError::Inventory)
    );
}

fn subject_of(
    edit: impl FnOnce(&mut Value),
) -> Result<SubjectFactors, ObservationError> {
    let mut value = envelope();
    edit(&mut value);
    let observation = authenticated(serde_json::to_vec(&value).unwrap());
    observe_subject(&expected(), &observation, SUBJECT, &request())
}

#[test]
fn observes_the_trustees_own_factors_for_that_subject_only() {
    let factors = subject_of(|_| {}).unwrap();
    assert_eq!(factors.subject, SUBJECT);
    assert_eq!(factors.observed_at, OBSERVED_AT);
    assert_eq!(factors.otp_credentials.len(), 1);
    assert_eq!(factors.otp_credentials[0].created_at, OBSERVED_AT - 86_400);
    assert!(factors.pending_actions.is_empty());

    let pending = subject_of(|v| {
        v["subject"]["pending_required_actions"] = json!(["CONFIGURE_TOTP"]);
    })
    .unwrap();
    assert_eq!(pending.pending_actions, vec!["CONFIGURE_TOTP".to_owned()]);

    let refusals: Vec<(Edit, ObservationError)> = vec![
        (
            case("no subject section", |v| {
                v.as_object_mut().unwrap().remove("subject");
            }),
            ObservationError::Inventory,
        ),
        (
            case("another user", |v| {
                v["subject"]["user_id"] = json!(OTHER_SUBJECT);
            }),
            ObservationError::Inventory,
        ),
        (
            case("partial", |v| {
                v["subject"]["completeness"] = json!("partial")
            }),
            ObservationError::Incomplete,
        ),
        (
            case("read denied", |v| {
                v["subject"]["completeness"] = json!("read_denied");
            }),
            ObservationError::ReadDenied,
        ),
        (
            case("repeated credential", |v| {
                let again = v["subject"]["otp_credentials"][0].clone();
                v["subject"]["otp_credentials"]
                    .as_array_mut()
                    .unwrap()
                    .push(again);
            }),
            ObservationError::Malformed,
        ),
    ];
    for ((name, edit), error) in refusals {
        assert_eq!(subject_of(edit).err(), Some(error), "{name}");
    }
}

/// Regression: Keycloak's stock nested browser flow is `browser forms`,
/// with a space. An inherited step naming it, or a flow so named, must be
/// observed (and then refused by policy), not taken as malformed.
#[test]
fn observes_stock_aliases_with_inner_spaces() {
    let inherited = observed(|v| {
        executions(v).push(json!({
            "priority": 30,
            "requirement": "ALTERNATIVE",
            "subflow_alias": "browser forms",
        }));
    })
    .unwrap();
    assert!(
        inherited.observed.flow.executions.last().unwrap().step
            == Step::Subflow("browser forms".to_owned())
    );
    assert_eq!(inherited.observed.qualifying_max_age(&policy()), None);

    let nested_flow = observed(|v| {
        v["flows"]["items"].as_array_mut().unwrap().push(json!({
            "alias": "browser forms",
            "top_level": false,
            "provider_id": "basic-flow",
            "executions": [],
        }));
    })
    .unwrap();
    assert_eq!(
        nested_flow.observed.qualifying_max_age(&policy()),
        Some(MAX_MFA_AGE)
    );

    // Spaces only inside, single, and nothing unprintable: anything else is
    // refused as malformed, deliberately, not classified.
    for bad in [
        " browser forms",
        "browser forms ",
        "browser  forms",
        "browser\tforms",
        "browser\u{b7}forms",
        "",
    ] {
        assert_eq!(
            refusal(|v| {
                executions(v).push(json!({
                    "priority": 30,
                    "requirement": "ALTERNATIVE",
                    "subflow_alias": bad,
                }));
            }),
            Some(ObservationError::Malformed),
            "{bad:?}"
        );
    }
}

#[test]
fn records_another_flow_reusing_a_dedicated_reference() {
    let collision = |reference: &'static str| {
        move |v: &mut Value| {
            v["flows"]["items"].as_array_mut().unwrap().push(json!({
                "alias": "browser forms",
                "top_level": false,
                "provider_id": "basic-flow",
                "executions": [{
                    "priority": 10,
                    "requirement": "REQUIRED",
                    "authenticator": "auth-otp-form",
                    "config": amr_config(reference, "60"),
                }],
            }));
        }
    };
    for reference in [PASSWORD_REFERENCE, TOTP_REFERENCE] {
        let evidence = observed(collision(reference)).unwrap();
        assert_eq!(
            evidence.observed.reference_collisions,
            vec!["browser forms".to_owned()]
        );
        assert_eq!(evidence.observed.qualifying_max_age(&policy()), None);
    }
    // Another reference value in another flow is no collision.
    let other =
        observed(collision("https://synthetic.invalid/amr/other")).unwrap();
    assert!(other.observed.reference_collisions.is_empty());
}

#[test]
fn accepts_only_the_reply_to_this_request() {
    let edits: Vec<Edit> = vec![
        case("other nonce", |v| {
            v["request"]["nonce"] =
                json!("synthetic-observation-request-nonce-0000001");
        }),
        case("other phase", |v| {
            v["request"]["phase"] = json!("before_authentication");
        }),
        case("other challenge", |v| v["request"]["challenge"] = json!(2)),
    ];
    for (name, edit) in edits {
        assert_eq!(refusal(edit), Some(ObservationError::Binding), "{name}");
    }
    let edits: Vec<Edit> = vec![
        case("no request", |v| {
            v.as_object_mut().unwrap().remove("request");
        }),
        case("unknown phase", |v| v["request"]["phase"] = json!("later")),
        case("request extra", |v| v["request"]["header"] = json!("x")),
    ];
    for (name, edit) in edits {
        assert_eq!(refusal(edit), Some(ObservationError::Schema), "{name}");
    }
    // The evidence carries the request it answered.
    let evidence = observed(|_| {}).unwrap();
    assert!(evidence.binding == Some(request().binding));
}

#[test]
fn reads_no_user_sessions() {
    // A session's shape is no evidence (the device login reuses an SSO
    // session), so the subject section has no sessions member at all.
    assert_eq!(
        subject_of(|v| v["subject"]["sessions"] = json!([])).err(),
        Some(ObservationError::Schema)
    );
}

#[test]
fn observes_stock_configs_in_other_flows() {
    // Other flows' executions carry ordinary authenticator configs.
    let stock = observed(|v| {
        v["flows"]["items"][0]["executions"][0]["config"] = json!({
            "synthetic.stock.option": "true",
            "default.reference.maxAge": "0",
        });
    })
    .unwrap();
    assert_eq!(
        stock.observed.qualifying_max_age(&policy()),
        Some(MAX_MFA_AGE)
    );
    assert!(stock.observed.reference_collisions.is_empty());

    // A repeated config key, anywhere, is refused.
    let body = serde_json::to_string(&envelope()).unwrap();
    let repeated = body.replacen(
        "\"default.reference.maxAge\"",
        "\"default.reference.maxAge\":\"60\",\"default.reference.maxAge\"",
        1,
    );
    assert_ne!(repeated, body);
    assert_eq!(
        observe(
            &expected(),
            &authenticated(repeated.into_bytes()),
            &request()
        )
        .err(),
        Some(ObservationError::Schema)
    );
}

/// How the synthetic collector answers.
#[derive(Clone, Copy)]
enum Answer {
    Echo,
    WrongNonce,
    Silent,
}

/// A synthetic collector: answers every request with the template,
/// echoing the request and stamped with that phase's time: `after_at` for
/// replies after the tokens.
struct SyntheticCollector {
    template: Value,
    answer: Answer,
    after_at: UnixSeconds,
}

impl AuthenticatedCollector for SyntheticCollector {
    fn collect(
        &self,
        request: &ObservationRequest,
    ) -> Option<AuthenticatedObservation> {
        let asked = &request.binding;
        let nonce = match self.answer {
            Answer::Silent => return None,
            Answer::Echo => asked.nonce.clone(),
            Answer::WrongNonce => {
                "synthetic-observation-request-nonce-0000009".to_owned()
            }
        };
        let (phase, observed_at) = match asked.phase {
            Phase::BeforeAuthentication => ("before_authentication", START + 5),
            Phase::AfterTokens => ("after_tokens", self.after_at),
        };
        let mut value = self.template.clone();
        value["request"] = json!({ "nonce": nonce, "phase": phase, "challenge": asked.challenge.0 });
        value["collector"]["observed_at"] = json!(observed_at);
        Some(authenticated(serde_json::to_vec(&value).unwrap()))
    }
}

/// A verifier for `machine` proven by the installed shape the template
/// shows.
fn proven(
    machine: &DeviceLogin,
    keys: super::super::TrustedKeyRing,
) -> NativeVerifier {
    let receipt = observed(|_| {}).unwrap().observed;
    let proof = FlowPolicyProof {
        receipt: receipt.clone(),
        drift: DriftEvidence {
            observed: receipt,
            checked_at: START,
            readback: Readback::Accepted,
            binding: None,
        },
    };
    NativeVerifier::with_flow_proof(machine, keys, proof).unwrap()
}

/// The tokens' second: both are issued at `START + 40`.
const ISSUED: UnixSeconds = START + 40;
/// A reply well after the tokens.
const LATER: UnixSeconds = super::super::tests::NOW - 5;

/// A poll in flight through the seam: its machine and ticket, a verifier,
/// the dedicated signed pair, and the factors seen before authentication.
struct SeamRun {
    machine: DeviceLogin,
    verifier: NativeVerifier,
    ticket: PollTicket,
    tokens: (String, String),
    before: Option<SubjectFactors>,
    challenge: crate::device::ChallengeId,
}

impl SeamRun {
    /// Tokens issued for this challenge's own device request.
    fn new(template: &Value, with_before: bool, prove: bool) -> SeamRun {
        SeamRun::issued_for(template, with_before, prove, Some(DEVICE_NONCE))
    }

    /// Tokens whose ID token carries `token_nonce` (none if absent), for a
    /// challenge whose device request sent `DEVICE_NONCE`. The tokens reuse
    /// an earlier ordinary SSO session's `sid`, as stock Keycloak does.
    fn issued_for(
        template: &Value,
        with_before: bool,
        prove: bool,
        token_nonce: Option<&str>,
    ) -> SeamRun {
        use super::super::tests::{access_claims, id_claims, set, Fixture};
        let mut fixture = fixture();
        let ticket = fixture.ticket();
        let challenge = fixture.challenge;
        let Fixture {
            signer,
            machine,
            verifier,
            ..
        } = fixture;
        let verifier = if prove {
            let NativeVerifier { keys, .. } = verifier;
            proven(&machine, keys)
        } else {
            verifier
        };
        let echo = SyntheticCollector {
            template: template.clone(),
            answer: Answer::Echo,
            after_at: LATER,
        };
        let before = with_before.then(|| {
            observe_before(&echo, &expected(), &verifier, &machine)
                .expect("before")
        });
        let dedicated = |mut claims: super::super::tests::ClaimSet| {
            set(
                &mut claims,
                "amr",
                json!([PASSWORD_REFERENCE, TOTP_REFERENCE]),
            );
            set(&mut claims, "iat", json!(ISSUED));
            if claims.contains_key("nbf") {
                set(&mut claims, "nbf", json!(ISSUED));
            }
            set(&mut claims, "sid", json!(SESSION));
            claims
        };
        // As stock Keycloak issues them: the access token, then the ID token
        // with the request's nonce and the access token's `at_hash`.
        let access = signer.sign(&dedicated(access_claims()));
        let mut id = dedicated(id_claims());
        if let Some(nonce) = token_nonce {
            set(&mut id, "nonce", json!(nonce));
        }
        set(
            &mut id,
            "at_hash",
            json!(super::super::rs256_access_hash(&access)),
        );
        SeamRun {
            tokens: (access, signer.sign(&id)),
            machine,
            verifier,
            ticket,
            before,
            challenge,
        }
    }

    /// One settlement attempt, with the collector answering `answer` from
    /// `template`, stamped `after_at`.
    fn settle(
        &mut self,
        template: &Value,
        answer: Answer,
        after_at: UnixSeconds,
    ) -> Result<Observed, DeviceError> {
        use super::super::tests::{clock, NOW};
        let collector = SyntheticCollector {
            template: template.clone(),
            answer,
            after_at,
        };
        settle_observed(
            &self.verifier,
            &mut self.machine,
            clock(5, NOW),
            &self.ticket,
            (self.tokens.0.as_str(), self.tokens.1.as_str()),
            &expected(),
            &collector,
            self.before.as_ref(),
            &DeviceNonce(DEVICE_NONCE.to_owned()),
        )
    }
}

/// One run through the seam, settled at once with a reply stamped later.
fn seam_run(
    template: Value,
    after: Answer,
    with_before: bool,
    prove: bool,
) -> (Result<Observed, DeviceError>, crate::device::ChallengeId) {
    let mut run = SeamRun::new(&template, with_before, prove);
    let status = run.settle(&template, after, LATER);
    (status, run.challenge)
}

fn denied(
    challenge: crate::device::ChallengeId,
) -> Result<Observed, DeviceError> {
    Ok(Observed::Settled(DeviceStatus::Closed {
        challenge,
        closure: crate::device::Closure::Denied,
    }))
}

fn approved(
    status: &Result<Observed, DeviceError>,
    challenge: crate::device::ChallengeId,
) -> bool {
    matches!(
        status,
        Ok(Observed::Settled(DeviceStatus::Approved { challenge: admitted, .. }))
            if *admitted == challenge
    )
}

#[test]
fn the_seam_admits_with_bound_fresh_evidence_only() {
    let (status, challenge) = seam_run(envelope(), Answer::Echo, true, true);
    assert!(approved(&status, challenge));

    // No reply after the tokens, a reply to another request, or no
    // observation before the authentication: never a proven factor.
    for (after, with_before) in [
        (Answer::Silent, true),
        (Answer::WrongNonce, true),
        (Answer::Echo, false),
    ] {
        let (status, challenge) =
            seam_run(envelope(), after, with_before, true);
        assert_eq!(status, denied(challenge));
    }

    // The approved run above reused an earlier SSO session's `sid`; what
    // binds its tokens to this challenge is the ID token's signed nonce.
    // Tokens issued for another device request, or with no nonce, are not
    // this challenge's, whatever else they prove.
    for token_nonce in
        [Some("synthetic-other-device-request-nonce-000000"), None]
    {
        let mut run = SeamRun::issued_for(&envelope(), true, true, token_nonce);
        assert_eq!(
            run.settle(&envelope(), Answer::Echo, LATER),
            denied(run.challenge)
        );
    }

    // Another flow reusing a dedicated reference, read back after the
    // tokens: no longer the installed shape.
    let mut collided = envelope();
    collided["flows"]["items"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "alias": "browser forms",
            "top_level": false,
            "provider_id": "basic-flow",
            "executions": [{
                "priority": 10,
                "requirement": "REQUIRED",
                "authenticator": "auth-otp-form",
                "config": amr_config(TOTP_REFERENCE, "60"),
            }],
        }));
    let (status, challenge) = seam_run(collided, Answer::Echo, true, true);
    assert_eq!(status, denied(challenge));
}

#[test]
fn the_seam_proves_nothing_for_the_unproven_verifier() {
    let discarded = |challenge| {
        Ok(Observed::Settled(DeviceStatus::Closed {
            challenge,
            closure: crate::device::Closure::Discarded(
                crate::device::DiscardReason::MfaAbsent,
            ),
        }))
    };
    let (status, challenge) = seam_run(envelope(), Answer::Echo, true, false);
    assert_eq!(status, discarded(challenge));
    // Nor is anything retained for it: a same-second reply settles too.
    let mut run = SeamRun::new(&envelope(), true, false);
    assert_eq!(
        run.settle(&envelope(), Answer::Echo, ISSUED),
        discarded(run.challenge)
    );
}

/// Finding A: a collector asked at once after the tokens usually replies in
/// their own second. That reply is not admitted, but neither does it close
/// an otherwise legitimate challenge: the poll is retained, and a fresh
/// request answered in a later second decides.
#[test]
fn a_same_second_reply_retains_the_poll_until_a_later_one() {
    let mut run = SeamRun::new(&envelope(), true, true);
    assert_eq!(
        run.settle(&envelope(), Answer::Echo, ISSUED),
        Ok(Observed::Retained)
    );
    assert_eq!(
        run.settle(&envelope(), Answer::Echo, ISSUED),
        Ok(Observed::Retained)
    );
    // Still pending, the ticket in flight, no grant.
    assert!(matches!(
        run.machine.status(super::super::tests::clock(5, LATER)),
        DeviceStatus::Pending { .. }
    ));
    assert_eq!(
        run.machine.authorize(super::super::tests::clock(5, LATER)),
        Err(DeviceError::NoGrant)
    );
    let status = run.settle(&envelope(), Answer::Echo, ISSUED + 1);
    assert!(approved(&status, run.challenge));
}

#[test]
fn only_the_tokens_own_second_is_retained() {
    // Earlier than the tokens: genuinely before them, denied at once.
    let mut run = SeamRun::new(&envelope(), true, true);
    assert_eq!(
        run.settle(&envelope(), Answer::Echo, ISSUED - 1),
        denied(run.challenge)
    );

    // Tokens of another device request are not this transaction's: they
    // are denied at once, even with a same-second reply, never retained,
    // and the settled ticket cannot be tried again.
    let mut run = SeamRun::issued_for(
        &envelope(),
        true,
        true,
        Some("synthetic-other-device-request-nonce-000000"),
    );
    assert_eq!(
        run.settle(&envelope(), Answer::Echo, ISSUED),
        denied(run.challenge)
    );
    assert_eq!(
        run.settle(&envelope(), Answer::Echo, LATER),
        Err(DeviceError::AlreadySettled)
    );

    // A retained poll is no oracle: a reply to another request, even in a
    // later second, is never evidence and settles denied.
    let mut run = SeamRun::new(&envelope(), true, true);
    assert_eq!(
        run.settle(&envelope(), Answer::Echo, ISSUED),
        Ok(Observed::Retained)
    );
    assert_eq!(
        run.settle(&envelope(), Answer::WrongNonce, LATER),
        denied(run.challenge)
    );
    // Once settled, the ticket settles nothing again.
    assert_eq!(
        run.settle(&envelope(), Answer::Echo, LATER),
        Err(DeviceError::AlreadySettled)
    );
}

#[test]
fn health_facts_are_facts_only() {
    let receipt = observed(|_| {}).unwrap().observed;
    let exact = authenticated(serde_json::to_vec(&envelope()).unwrap());
    assert_eq!(
        facts(&expected(), &exact, &request(), &policy(), &receipt),
        PolicyFacts {
            observed: true,
            qualifies: true,
            matches_receipt: true,
        }
    );
    let mut partial = envelope();
    partial["mappers"]["completeness"] = json!("partial");
    let partial = authenticated(serde_json::to_vec(&partial).unwrap());
    assert_eq!(
        facts(&expected(), &partial, &request(), &policy(), &receipt),
        PolicyFacts {
            observed: false,
            qualifies: false,
            matches_receipt: false,
        }
    );
}

#[test]
fn the_verifier_made_for_a_machine_stays_unproven() {
    // An observation exists, yet the machine's verifier trusts no factor:
    // nothing here builds a proof outside the tests.
    assert!(observed(|_| {}).is_ok());
    let fixture = fixture();
    assert!(matches!(
        fixture.verifier.second_factor,
        SecondFactorPolicy::Unproven
    ));
}
