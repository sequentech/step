// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Authentication audit records keep authentication outcomes, never network
//! attribution. The Java producer applies the same boundary before enqueueing.
use serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nested_network_attributes_never_reach_authentication_audit_records() {
        let mut input = json!({"client_id":"portal","error":"invalid_user_credentials",
            "ip_address":"198.51.100.7","details":{"browser":"Safari","country":"TR",
            "nested":[{"userAgent":"private device","remote-ip":"198.51.100.9"}], "reason":"from 2001:db8::1",
            "auth_method":"openid-connect"}});
        redact_network(&mut input);
        assert_eq!(
            input,
            json!({"client_id":"portal","error":"invalid_user_credentials",
            "details":{"nested":[{}],"reason":"[redacted]","auth_method":"openid-connect"}})
        );
    }

    #[test]
    fn only_successful_registration_of_a_real_event_identity_triggers_allocation() {
        use crate::tasks::electoral_log::{LogEventBody, LogEventInput, LogMessageType};
        let mut input = LogEventInput {
            delivery_id: None,
            tenant_id: "00000000-0000-4000-8000-000000000001".into(),
            election_event_id: "00000000-0000-4000-8000-000000000002".into(),
            user_id: Some("00000000-0000-4000-8000-000000000003".into()),
            username: None,
            message_type: LogMessageType::KeycloakEvent("REGISTER".into()),
            body: LogEventBody::Plain("null".into()),
            event_time_ms: None,
            details: None,
            admin_event: None,
        };
        assert!(registration_identity(&input).is_some());
        for kind in ["REGISTER_ERROR", "LOGIN", "UPDATE_PROFILE"] {
            input.message_type = LogMessageType::KeycloakEvent(kind.into());
            assert!(registration_identity(&input).is_none());
        }
        input.message_type = LogMessageType::KeycloakEvent("REGISTER".into());
        input.election_event_id = "null".into();
        assert!(registration_identity(&input).is_none());
    }

    #[test]
    fn raw_error_messages_cannot_smuggle_network_attribution() {
        assert_eq!(
            safe_error_text("invalid_user_credentials"),
            "invalid_user_credentials"
        );
        assert_eq!(safe_error_text("null"), "null");
        assert_eq!(
            safe_error_text(r#"{"error":{"ip_address":"198.51.100.7","reason":"safe"}}"#),
            r#"{"error":{"reason":"safe"}}"#
        );
        assert_eq!(
            safe_error_text("Login from 198.51.100.7 failed"),
            "authentication_error"
        );
        assert_eq!(
            safe_error_text(r#"{"error":"bad","ip_address":"198.51.100.7"}"#),
            r#"{"error":"bad"}"#
        );
    }
}

fn network_key(key: &str) -> bool {
    let name: String = key
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    matches!(
        name.as_str(),
        "ip" | "ipaddress"
            | "remoteip"
            | "clientip"
            | "remoteaddress"
            | "forwardedfor"
            | "xforwardedfor"
            | "country"
            | "countrycode"
            | "useragent"
            | "browser"
            | "device"
            | "deviceid"
            | "gps"
            | "geolocation"
    )
}

pub fn redact_network(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.retain(|name, _| !network_key(name));
            for (name, value) in fields.iter_mut() {
                if name == "error" {
                    if let Some(text) = value.as_str() {
                        if text != "null" && !is_error_code(text) {
                            *value = "authentication_error".into();
                        }
                    }
                }
                redact_network(value);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact_network),
        Value::String(text) if contains_network_address(text) => *text = "[redacted]".into(),
        _ => {}
    }
}

fn contains_network_address(text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_hexdigit() && c != ':' && c != '.')
        .any(|part| part.parse::<std::net::IpAddr>().is_ok())
}

fn is_error_code(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 128
        && text.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
}

pub fn safe_error_text(text: &str) -> String {
    if let Ok(mut value) = serde_json::from_str::<Value>(text) {
        if value.is_object() || value.is_array() {
            redact_network(&mut value);
            return value.to_string();
        }
        if value.is_null() {
            return "null".into();
        }
    }
    if is_error_code(text) {
        text.to_owned()
    } else {
        "authentication_error".into()
    }
}

pub fn sanitized_event(
    input: &crate::tasks::electoral_log::LogEventInput,
) -> crate::tasks::electoral_log::LogEventInput {
    use crate::tasks::electoral_log::{LogEventBody, LogMessageType};
    let mut clean = input.clone();
    if let Some(details) = &mut clean.details {
        redact_network(details);
    }
    if let Some(admin) = &mut clean.admin_event {
        // Administrative representations describe configuration, not the
        // operator's network. Preserve their audit diff while cleaning actor.
        if let Some(actor) = admin.get_mut("actor") {
            redact_network(actor);
        }
        if let Some(error) = admin.get_mut("error") {
            if let Some(text) = error.as_str() {
                *error = Value::String(safe_error_text(text));
            }
        }
    }
    if let LogMessageType::KeycloakEvent(kind) = &mut clean.message_type {
        if kind.is_empty()
            || kind.len() > 64
            || !kind.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
        {
            *kind = "UNKNOWN_EVENT".into();
        }
        if let LogEventBody::Plain(text) = &clean.body {
            clean.body = LogEventBody::Plain(safe_error_text(text));
        }
    }
    clean
}

/// Handle historical queue entries during rollout. Fresh producers no longer
/// put network details in this queue. Never allocate a missing pseudonym here.
pub async fn retain_queued_signin_network(
    tx: &deadpool_postgres::Transaction<'_>,
    events: &[crate::tasks::electoral_log::LogEventInput],
) -> anyhow::Result<()> {
    use crate::postgres::election_event::get_election_event_by_id_if_exist;
    use crate::services::electoral_log_board::get_event_store;
    use crate::tasks::electoral_log::LogMessageType;
    use electoral_log::adapters::privacy::ForensicNetworkRecord;
    use sequent_core::types::vote_privacy::{BallotIdentity, NetworkDataPolicy};
    let mut policies = std::collections::HashMap::new();
    for input in events {
        if !matches!(&input.message_type, LogMessageType::KeycloakEvent(kind) if kind == "LOGIN" || kind == "LOGIN_ERROR")
        {
            continue;
        }
        let Some(details) = &input.details else {
            continue;
        };
        let Some(ip) = details
            .get("ip_address")
            .and_then(Value::as_str)
            .filter(|ip| ip.parse::<std::net::IpAddr>().is_ok())
        else {
            continue;
        };
        let Some(voter) = input
            .user_id
            .as_deref()
            .filter(|v| *v != "null" && !v.is_empty())
        else {
            continue;
        };
        let Some(delivery) = input.delivery_id.as_deref() else {
            continue;
        };
        if uuid::Uuid::parse_str(&input.tenant_id).is_err()
            || uuid::Uuid::parse_str(&input.election_event_id).is_err()
        {
            continue;
        }
        let scope = (input.tenant_id.clone(), input.election_event_id.clone());
        if !policies.contains_key(&scope) {
            let event =
                get_election_event_by_id_if_exist(tx, &input.tenant_id, &input.election_event_id)
                    .await?;
            let keep = event
                .map(|event| event.get_presentation())
                .transpose()?
                .flatten()
                .is_some_and(|p| {
                    p.ballot_identity == Some(BallotIdentity::Pseudonym)
                        && p.network_data_policy == Some(NetworkDataPolicy::ForensicStore)
                });
            policies.insert(scope.clone(), keep);
        }
        if policies.get(&scope) != Some(&true) {
            continue;
        }
        let store = get_event_store(&input.election_event_id).await?;
        let Some(pseudonym) = store
            .voter_pseudonym(&input.election_event_id, voter)
            .await?
        else {
            continue;
        };
        let Some(at) = input
            .event_time_ms
            .and_then(chrono::DateTime::from_timestamp_millis)
        else {
            continue;
        };
        store
            .record_forensic_network(&ForensicNetworkRecord {
                election_event_id: &input.election_event_id,
                election_id: None,
                area_id: None,
                event_kind: "sign-in",
                delivery_id: delivery,
                pseudonym: &pseudonym,
                ballot_id: None,
                ip,
                country: details.get("country").and_then(Value::as_str),
                user_agent: details.get("user_agent").and_then(Value::as_str),
                recorded_at: at,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Cannot retain private sign-in network evidence"))?;
    }
    Ok(())
}

fn registration_identity(
    input: &crate::tasks::electoral_log::LogEventInput,
) -> Option<(&str, &str, &str)> {
    use crate::tasks::electoral_log::LogMessageType;
    if !matches!(&input.message_type, LogMessageType::KeycloakEvent(kind) if kind == "REGISTER") {
        return None;
    }
    let voter = input.user_id.as_deref()?;
    if [
        input.tenant_id.as_str(),
        input.election_event_id.as_str(),
        voter,
    ]
    .iter()
    .any(|id| uuid::Uuid::parse_str(id).is_err())
    {
        return None;
    }
    Some((&input.tenant_id, &input.election_event_id, voter))
}

/// Successful registration can create a voter through Keycloak's smart-link
/// flow as well as Harvest. Wait for its account transaction to be visible,
/// verify the authoritative voter group, then share the publication lock.
/// No allocation occurs in receive/cast or from caller-supplied group claims.
pub async fn allocate_registered_voters(
    hasura: &deadpool_postgres::Transaction<'_>,
    keycloak: &deadpool_postgres::Transaction<'_>,
    events: &[crate::tasks::electoral_log::LogEventInput],
) -> anyhow::Result<()> {
    use anyhow::Context;
    let registrations: std::collections::BTreeSet<_> =
        events.iter().filter_map(registration_identity).collect();
    for (tenant, event_id, voter) in registrations {
        let Some(event) = crate::postgres::election_event::get_election_event_by_id_if_exist(
            hasura, tenant, event_id,
        )
        .await?
        else {
            continue;
        };
        if crate::services::vote_privacy::identity_mode(&event)?
            != sequent_core::types::vote_privacy::BallotIdentity::Pseudonym
        {
            continue;
        }
        let realm = sequent_core::services::keycloak::get_event_realm(tenant, event_id);
        let group = std::env::var("KEYCLOAK_VOTER_GROUP_NAME")
            .context("Voter group configuration unavailable")?;
        let row=keycloak.query_opt(
            "SELECT u.service_account_client_link IS NULL AND EXISTS (SELECT 1 FROM user_group_membership m JOIN keycloak_group g ON g.id=m.group_id AND g.realm_id=u.realm_id WHERE m.user_id=u.id AND g.name=$3) AS voter_group \
             FROM user_entity u JOIN realm r ON r.id=u.realm_id WHERE r.name=$1 AND u.id=$2",
            &[&realm,&voter,&group]).await?.context("Registration account has not committed; retry event batch")?;
        if row.get::<_, bool>(0) {
            crate::services::vote_privacy::allocate_added_voter(hasura, tenant, event_id, voter)
                .await?;
        }
    }
    Ok(())
}
