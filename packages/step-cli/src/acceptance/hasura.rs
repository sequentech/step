// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Reads the voting records through Hasura with the tenant administrator's
//! session, the same data the Admin Portal shows.
use super::voting::{LoggedCast, StoredBallot, UserEvent, Voter, VotingRecords};
use crate::ports::graphql::GraphqlClient;
use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use electoral_log::messages::statement::{StatementLogType, StatementType};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use windmill::services::{cast_votes::CastVoteStatus, users::FilterOption};

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

const CAST_VOTES: &str = "query AcceptanceCastVotes($tenant: uuid!, $event: uuid!, $voters: [String!], $status: String) {
  sequent_backend_cast_vote(order_by: {created_at: asc}, where: {
    tenant_id: {_eq: $tenant}, election_event_id: {_eq: $event},
    voter_id_string: {_in: $voters}, status: {_eq: $status}
  }) { voter_id_string ballot_id content created_at }
}";

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

#[derive(Deserialize)]
struct CastVoteRow {
    voter_id_string: Option<String>,
    ballot_id: Option<String>,
    content: Option<String>,
    created_at: DateTime<Utc>,
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
        let mut rows = Vec::new();
        loop {
            let page: Vec<LogRow> = self.query(
                LIST_LOG,
                json!({
                    "event": event,
                    "filter": {"statement_kind": kind.to_string(), "user_id": user_id},
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

    fn stored_ballots(&self, event: &str, voter_ids: &[String]) -> Result<Vec<StoredBallot>> {
        let rows: Vec<CastVoteRow> = self.query(
            CAST_VOTES,
            json!({
                "tenant": self.tenant_id,
                "event": event,
                "voters": voter_ids,
                "status": CastVoteStatus::Valid.to_string(),
            }),
            &["sequent_backend_cast_vote"],
        )?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                Some(StoredBallot {
                    voter_id: row.voter_id_string?,
                    ballot_id: row.ballot_id.unwrap_or_default(),
                    content: row.content.unwrap_or_default(),
                    cast_at: row.created_at,
                })
            })
            .collect())
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
    fn only_valid_cast_votes_of_the_tenant_and_event_are_read() {
        let records = records(QueuedGraphql::default().respond(json!({"data": {"sequent_backend_cast_vote": [
            {"voter_id_string": "v", "ballot_id": "b", "content": "c", "created_at": "2028-01-10T09:00:00.5+00:00"},
            {"voter_id_string": null, "ballot_id": "x", "content": "c", "created_at": "2028-01-10T09:00:00+00:00"}
        ]}})));
        let ballots = records.stored_ballots("event", &["v".into()]).unwrap();
        assert_eq!(ballots.len(), 1);
        assert_eq!(ballots[0].ballot_id, "b");
        assert_eq!(
            records.graphql.requests()[0]["variables"],
            json!({"tenant": "tenant", "event": "event", "voters": ["v"], "status": "valid"})
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
}
