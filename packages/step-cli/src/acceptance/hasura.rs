// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Reads the voting records through Hasura with the tenant administrator's
//! session, the same data the Admin Portal shows, and the stored ballots from
//! the election event's ballot box.
use super::ballot_box;
use super::voting::{
    CastBallot, LoggedCast, PrivacyPolicy, StoredBallot, UserEvent, Voter, VotingRecords,
};
use crate::ports::graphql::GraphqlClient;
use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use electoral_log::adapters::privacy::{BallotBoxPrivacy, BallotIdentity};
use electoral_log::messages::statement::{StatementLogType, StatementType};
use sequent_core::types::{keycloak::Role, vote_privacy};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use windmill::services::users::FilterOption;

const GET_VOTER: &str = "query AcceptanceVoter($body: GetUsersInput!) {
  get_users(body: $body) { items { id username enabled area { id name } } }
}";

const LIST_LOG: &str = "query AcceptanceLog($event: String, $filter: ElectoralLogFilter, $limit: Int, $offset: Int) {
  listElectoralLog(election_event_id: $event, filter: $filter, limit: $limit, offset: $offset, order_by: {id: asc}) {
    items { statement_timestamp message }
  }
}";

const PUBLISHED_AREAS: &str =
    "query AcceptanceBallotStyles($tenant: uuid!, $event: uuid!, $areas: [uuid!]) {
  sequent_backend_ballot_style(distinct_on: area_id, where: {
    tenant_id: {_eq: $tenant}, election_event_id: {_eq: $event}, area_id: {_in: $areas},
    deleted_at: {_is_null: true},
    ballot_publication: {published_at: {_is_null: false}, deleted_at: {_is_null: true}}
  }) { area_id }
}";

const PRESENTATION: &str = "query AcceptancePresentation($tenant: uuid!, $event: uuid!) {
  sequent_backend_election_event(where: {tenant_id: {_eq: $tenant}, id: {_eq: $event}}) { presentation }
}";

const GET_ROLES: &str = "query AcceptanceRoles($body: GetRolesInput!) {
  get_roles(body: $body) { items { id name permissions } }
}";

/// The statement body of a cast under a pseudonym.
const PRIVACY_CAST_VOTE: &str = "PrivacyCastVote";

/// Rows per request of the election event's log.
const LOG_PAGE: usize = 100;

pub struct HasuraRecords<G> {
    pub graphql: G,
    pub tenant_id: String,
}

#[derive(Deserialize)]
struct Area {
    id: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
struct User {
    id: Option<String>,
    username: Option<String>,
    enabled: Option<bool>,
    area: Option<Area>,
}

#[derive(Deserialize)]
struct LogRow {
    statement_timestamp: i64,
    message: String,
}

impl<G: GraphqlClient> HasuraRecords<G> {
    /// The value at `path` in `data`, or the GraphQL errors.
    fn query<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: Value,
        path: &[&str],
    ) -> Result<T> {
        let response = self
            .graphql
            .post::<Value, _>(&json!({"query": query, "variables": variables}))
            .map_err(|error| anyhow!(error.to_string()))?;
        if let Some(errors) = response.errors.filter(|errors| !errors.is_empty()) {
            let messages: Vec<String> = errors.into_iter().map(|error| error.message).collect();
            bail!("{}", messages.join(", "));
        }
        let mut value = response.data.context("The response has no data")?;
        for key in path {
            value = value
                .get_mut(*key)
                .map(Value::take)
                .with_context(|| format!("The response has no {key}"))?;
        }
        Ok(serde_json::from_value(value)?)
    }

    /// Every entry of one kind about one user, oldest first.
    fn log(&self, event: &str, user_id: &str, kind: StatementType) -> Result<Vec<LogRow>> {
        self.log_matching(
            event,
            json!({"statement_kind": kind.to_string(), "user_id": user_id}),
        )
    }

    /// Every entry the filter matches, oldest first.
    fn log_matching(&self, event: &str, filter: Value) -> Result<Vec<LogRow>> {
        let mut rows = Vec::new();
        loop {
            let page: Vec<LogRow> = self.query(
                LIST_LOG,
                json!({
                    "event": event,
                    "filter": filter,
                    "limit": LOG_PAGE,
                    "offset": rows.len(),
                }),
                &["listElectoralLog", "items"],
            )?;
            let last = page.len() < LOG_PAGE;
            rows.extend(page);
            if last {
                return Ok(rows);
            }
        }
    }
}

fn time(seconds: i64) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp(seconds, 0).context("Log entry with an invalid timestamp")
}

/// The fields of a statement body, whatever its variant is called.
fn body_fields(message: &Value) -> Option<&Vec<Value>> {
    message["statement"]["body"]
        .as_object()?
        .values()
        .next()?
        .as_array()
}

impl<G: GraphqlClient> VotingRecords for HasuraRecords<G> {
    fn voter(&self, event: &str, username: &str) -> Result<Option<Voter>> {
        let users: Vec<User> = self.query(
            GET_VOTER,
            json!({"body": {
                "tenant_id": self.tenant_id,
                "election_event_id": event,
                "username": FilterOption::IsEqual(username.into()),
                "limit": LOG_PAGE,
            }}),
            &["get_users", "items"],
        )?;
        Ok(users
            .into_iter()
            .find(|user| user.username.as_deref() == Some(username))
            .and_then(|user| {
                let area = user.area;
                Some(Voter {
                    id: user.id?,
                    username: username.into(),
                    enabled: user.enabled.unwrap_or(false),
                    area_id: area.as_ref().and_then(|area| area.id.clone()),
                    area_name: area.and_then(|area| area.name),
                })
            }))
    }

    fn user_events(&self, event: &str, voter_id: &str) -> Result<Vec<UserEvent>> {
        self.log(event, voter_id, StatementType::KeycloakUserEvent)?
            .into_iter()
            .map(|row| {
                let message: Value = serde_json::from_str(&row.message)?;
                let kind = body_fields(&message)
                    .and_then(|fields| fields.get(1))
                    .and_then(Value::as_str)
                    .context("Log entry without a Keycloak event type")?;
                Ok(UserEvent {
                    at: time(row.statement_timestamp)?,
                    kind: kind.into(),
                    error: message["statement"]["head"]["log_type"]
                        == StatementLogType::ERROR.to_string(),
                })
            })
            .collect()
    }

    fn published_areas(&self, event: &str, areas: &[String]) -> Result<BTreeSet<String>> {
        #[derive(Deserialize)]
        struct Row {
            area_id: Option<String>,
        }
        let rows: Vec<Row> = self.query(
            PUBLISHED_AREAS,
            json!({"tenant": self.tenant_id, "event": event, "areas": areas}),
            &["sequent_backend_ballot_style"],
        )?;
        Ok(rows.into_iter().filter_map(|row| row.area_id).collect())
    }

    fn ballot_identity(&self, event: &str) -> Result<BallotIdentity> {
        ballot_box::ballot_identity(event)
    }

    fn stored_ballots(&self, event: &str, voter_ids: &[String]) -> Result<Vec<StoredBallot>> {
        ballot_box::stored_ballots(event, voter_ids)
    }

    fn participants(&self, event: &str, voter_ids: &[String]) -> Result<BTreeSet<String>> {
        ballot_box::participants(event, voter_ids)
    }

    fn cast_ballots(&self, event: &str, ballot_ids: &[String]) -> Result<Vec<CastBallot>> {
        ballot_box::cast_ballots(event, ballot_ids)
    }

    fn private_casts(&self, event: &str, since: DateTime<Utc>) -> Result<BTreeSet<String>> {
        // A pseudonymous cast is logged under the CastVote kind, by its pseudonym.
        let rows = self.log_matching(
            event,
            json!({
                "statement_kind": StatementType::CastVote.to_string(),
                "statement_timestamp_from": since.to_rfc3339(),
            }),
        )?;
        let mut ballot_ids = BTreeSet::new();
        for row in rows {
            let message: Value = serde_json::from_str(&row.message)?;
            if let Some(ballot_id) =
                message["statement"]["body"][PRIVACY_CAST_VOTE]["ballot_id"].as_str()
            {
                ballot_ids.insert(ballot_id.to_owned());
            }
        }
        Ok(ballot_ids)
    }

    fn privacy_policy(&self, event: &str) -> Result<PrivacyPolicy> {
        #[derive(Deserialize)]
        struct Presentation {
            ballot_identity: Option<vote_privacy::BallotIdentity>,
            network_data_policy: Option<vote_privacy::NetworkDataPolicy>,
        }
        #[derive(Deserialize)]
        struct Row {
            presentation: Option<Value>,
        }
        let rows: Vec<Row> = self.query(
            PRESENTATION,
            json!({"tenant": self.tenant_id, "event": event}),
            &["sequent_backend_election_event"],
        )?;
        let row = rows
            .into_iter()
            .next()
            .with_context(|| format!("Election event {event} not found"))?;
        let presentation: Presentation =
            serde_json::from_value(row.presentation.unwrap_or_else(|| json!({})))
                .context("Invalid privacy settings in the election event's presentation")?;
        Ok(PrivacyPolicy {
            ballot_identity: presentation.ballot_identity,
            network_data_policy: presentation.network_data_policy,
        })
    }

    fn ballot_box_privacy(&self, event: &str) -> Result<BallotBoxPrivacy> {
        ballot_box::ballot_box_privacy(event)
    }

    fn roles(&self) -> Result<Vec<Role>> {
        self.query(
            GET_ROLES,
            json!({"body": {"tenant_id": self.tenant_id}}),
            &["get_roles", "items"],
        )
    }

    fn logged_casts(&self, event: &str, voter_id: &str) -> Result<Vec<LoggedCast>> {
        let mut casts = Vec::new();
        for row in self.log(event, voter_id, StatementType::CastVote)? {
            let message: Value = serde_json::from_str(&row.message)?;
            let Some(ballot_id) = message["ballot_id"].as_str() else {
                continue;
            };
            // CastVote and CastVoteWithChannel both hold the cast vote hash third.
            let hash: Option<Vec<u8>> = body_fields(&message)
                .and_then(|fields| fields.get(2))
                .and_then(|hash| serde_json::from_value(hash.clone()).ok());
            if let Some(hash) = hash {
                casts.push(LoggedCast {
                    ballot_id: ballot_id.into(),
                    vote_hash: hex::encode(hash),
                });
            }
        }
        Ok(casts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::graphql::QueuedGraphql;

    fn records(graphql: QueuedGraphql) -> HasuraRecords<QueuedGraphql> {
        HasuraRecords {
            graphql,
            tenant_id: "tenant".into(),
        }
    }

    fn log_page(rows: Vec<Value>) -> Value {
        json!({"data": {"listElectoralLog": {"items": rows}}})
    }

    fn user_event(timestamp: i64, kind: &str, log_type: &str) -> Value {
        let message = json!({
            "statement": {
                "head": {"kind": "KeycloakUserEvent", "log_type": log_type},
                "body": {"KeycloakUserEvent": ["null", kind]}
            },
            "ballot_id": null
        });
        json!({"statement_timestamp": timestamp, "message": message.to_string()})
    }

    #[test]
    fn a_voter_is_found_by_exact_username_in_the_event() {
        let records = records(QueuedGraphql::default().respond(
            json!({"data": {"get_users": {"items": [
                {"id": "1", "username": "ana-maria", "enabled": true, "area": null},
                {"id": "2", "username": "ana", "enabled": true, "area": {"id": "a", "name": "A"}}
            ]}}}),
        ));
        let voter = records.voter("event", "ana").unwrap().unwrap();
        assert_eq!(
            voter,
            Voter {
                id: "2".into(),
                username: "ana".into(),
                enabled: true,
                area_id: Some("a".into()),
                area_name: Some("A".into()),
            }
        );
        let request = &records.graphql.requests()[0];
        assert_eq!(
            request["variables"]["body"],
            json!({"tenant_id": "tenant", "election_event_id": "event",
                "username": {"IsEqual": "ana"}, "limit": 100})
        );
    }

    #[test]
    fn a_missing_voter_is_not_an_error() {
        let records = records(
            QueuedGraphql::default().respond(json!({"data": {"get_users": {"items": []}}})),
        );
        assert_eq!(records.voter("event", "ana").unwrap(), None);
    }

    #[test]
    fn graphql_errors_and_transport_failures_stop_the_check() {
        let records = records(
            QueuedGraphql::default()
                .respond(json!({"data": null, "errors": [{"message": "not allowed"}]}))
                .fail("connection refused")
                .respond(json!({"data": {}})),
        );
        let error = |result: Result<Option<Voter>>| format!("{:#}", result.unwrap_err());
        assert!(error(records.voter("event", "ana")).contains("not allowed"));
        assert!(error(records.voter("event", "ana")).contains("connection refused"));
        assert!(error(records.voter("event", "ana")).contains("no get_users"));
    }

    #[test]
    fn user_events_are_read_page_by_page() {
        let first: Vec<Value> = (0..LOG_PAGE as i64)
            .map(|second| user_event(second, "CODE_TO_TOKEN", "INFO"))
            .collect();
        let records = records(
            QueuedGraphql::default()
                .respond(log_page(first))
                .respond(log_page(vec![
                    user_event(1000, "LOGIN", "INFO"),
                    user_event(1001, "LOGIN_ERROR", "ERROR"),
                ])),
        );
        let events = records.user_events("event", "voter").unwrap();
        assert_eq!(events.len(), LOG_PAGE + 2);
        assert_eq!(
            events[LOG_PAGE],
            UserEvent {
                at: DateTime::from_timestamp(1000, 0).unwrap(),
                kind: "LOGIN".into(),
                error: false,
            }
        );
        assert!(events[LOG_PAGE + 1].error);
        let requests = records.graphql.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1]["variables"]["offset"], LOG_PAGE);
        assert_eq!(
            requests[0]["variables"]["filter"],
            json!({"statement_kind": "KeycloakUserEvent", "user_id": "voter"})
        );
    }

    #[test]
    fn cast_votes_in_the_log_give_the_ballot_id_and_hash_of_both_statement_forms() {
        let hash: Vec<u8> = (0..64).collect();
        let message = |variant: &str, ballot_id: Value| {
            let message = json!({
                "statement": {"body": {variant: ["election", [1, 2], hash, "ip", "country", "ONLINE"]}},
                "ballot_id": ballot_id
            });
            json!({"statement_timestamp": 1, "message": message.to_string()})
        };
        let records = records(QueuedGraphql::default().respond(log_page(vec![
            message("CastVote", json!("b1")),
            message("CastVoteWithChannel", json!("b2")),
            message("CastVote", Value::Null),
        ])));
        let casts = records.logged_casts("event", "voter").unwrap();
        assert_eq!(
            casts,
            [
                LoggedCast {
                    ballot_id: "b1".into(),
                    vote_hash: hex::encode(&hash)
                },
                LoggedCast {
                    ballot_id: "b2".into(),
                    vote_hash: hex::encode(&hash)
                }
            ]
        );
        assert_eq!(
            records.graphql.requests()[0]["variables"]["filter"]["statement_kind"],
            "CastVote"
        );
    }

    #[test]
    fn pseudonymous_casts_in_the_log_give_their_ballot_ids_only() {
        let message = |body: Value| {
            let message = json!({"statement": {"body": body}, "ballot_id": "ABCD-0000"});
            json!({"statement_timestamp": 1, "message": message.to_string()})
        };
        let records = records(QueuedGraphql::default().respond(log_page(vec![
            message(json!({"PrivacyCastVote": {"ballot_id": "ABCD-0001", "pseudonym_hash": "00"}})),
            message(json!({"CastVote": ["election", [1, 2], [3], "ip", "country"]})),
        ])));
        let since = DateTime::from_timestamp(60, 0).unwrap();
        let ids = records.private_casts("event", since).unwrap();
        assert_eq!(ids, BTreeSet::from(["ABCD-0001".to_string()]));
        assert_eq!(
            records.graphql.requests()[0]["variables"]["filter"],
            json!({"statement_kind": "CastVote", "statement_timestamp_from": "1970-01-01T00:01:00+00:00"})
        );
    }

    #[test]
    fn published_areas_are_those_with_a_style_in_a_published_publication() {
        let records = records(
            QueuedGraphql::default()
                .respond(json!({"data": {"sequent_backend_ballot_style": [{"area_id": "a"}]}})),
        );
        let areas = records
            .published_areas("event", &["a".into(), "b".into()])
            .unwrap();
        assert_eq!(areas, BTreeSet::from(["a".to_string()]));
        let request = &records.graphql.requests()[0];
        assert!(request["query"]
            .as_str()
            .unwrap()
            .contains("published_at: {_is_null: false}"));
    }

    #[test]
    fn privacy_settings_are_read_from_the_event_presentation() {
        let records = records(
            QueuedGraphql::default()
                .respond(
                    json!({"data": {"sequent_backend_election_event": [{"presentation": {
                        "ballot_identity": "pseudonym",
                        "network_data_policy": "forensic-store",
                        "css": "body {}"
                    }}]}}),
                )
                .respond(
                    json!({"data": {"sequent_backend_election_event": [{"presentation": null}]}}),
                )
                .respond(json!({"data": {"sequent_backend_election_event": []}}))
                .respond(
                    json!({"data": {"sequent_backend_election_event": [{"presentation": {
                        "network_data_policy": "keep-everything"
                    }}]}}),
                ),
        );
        assert_eq!(
            records.privacy_policy("event").unwrap(),
            PrivacyPolicy {
                ballot_identity: Some(vote_privacy::BallotIdentity::Pseudonym),
                network_data_policy: Some(vote_privacy::NetworkDataPolicy::ForensicStore),
            }
        );
        assert_eq!(
            records.privacy_policy("event").unwrap(),
            PrivacyPolicy::default()
        );
        let error = |result: Result<PrivacyPolicy>| format!("{:#}", result.unwrap_err());
        assert!(error(records.privacy_policy("event")).contains("not found"));
        assert!(error(records.privacy_policy("event")).contains("Invalid privacy settings"));
        assert_eq!(
            records.graphql.requests()[0]["variables"],
            json!({"tenant": "tenant", "event": "event"})
        );
    }

    #[test]
    fn roles_are_read_with_their_permissions() {
        let records = records(QueuedGraphql::default().respond(
            json!({"data": {"get_roles": {"items": [
                {"id": "1", "name": "admin", "permissions": ["voter-pseudonym-resolve", "forensic-read"]},
                {"id": "2", "name": "empty", "permissions": null}
            ]}}}),
        ));
        let roles = records.roles().unwrap();
        assert_eq!(roles.len(), 2);
        assert_eq!(roles[0].name.as_deref(), Some("admin"));
        assert_eq!(
            roles[0].permissions.as_deref(),
            Some(
                &[
                    "voter-pseudonym-resolve".to_string(),
                    "forensic-read".to_string()
                ][..]
            )
        );
        assert_eq!(roles[1].permissions, None);
        assert_eq!(
            records.graphql.requests()[0]["variables"],
            json!({"body": {"tenant_id": "tenant"}})
        );
    }
}
