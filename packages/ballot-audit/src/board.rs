// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Resolve the producer's ceremony board without conflating it with the event log.
use crate::{
    incomplete, invalid_input,
    source::{board_identifier, one, rows},
};
use anyhow::{ensure, Result};
use serde_json::Value;
use tokio_postgres::Transaction;
use uuid::Uuid;

pub struct CeremonySources {
    pub ceremony: Value,
    pub elections: Vec<Value>,
}

#[derive(Debug)]
pub struct ResolvedBoards {
    pub bulletin_board: String,
    pub electoral_log: String,
    pub election_id: Option<String>,
}

fn id<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    let text = value[field]
        .as_str()
        .ok_or_else(|| incomplete(format!("saved ceremony mapping lacks {field}")))?;
    ensure!(
        Uuid::parse_str(text)
            .ok()
            .is_some_and(|id| id.to_string() == text),
        invalid_input("noncanonical ceremony mapping UUID")
    );
    Ok(text)
}

pub async fn read_ceremony_sources(
    tx: &Transaction<'_>,
    session: &Value,
) -> Result<CeremonySources> {
    let tenant = id(session, "tenant_id")?;
    let event = id(session, "election_event_id")?;
    let ceremony = id(session, "keys_ceremony_id")?;
    let ceremony = one(
        tx,
        "keys ceremony",
        "SELECT jsonb_build_object('id', id, 'tenant_id', tenant_id,
        'election_event_id', election_event_id, 'is_default', is_default) AS data
        FROM sequent_backend.keys_ceremony WHERE tenant_id = $1::text::uuid
        AND election_event_id = $2::text::uuid AND id = $3::text::uuid",
        &[&tenant, &event, &ceremony],
    )
    .await?;
    // Read every event election so ambiguous ownership cannot be hidden by a scope filter.
    let elections = rows(
        tx,
        "SELECT to_jsonb(e) AS data FROM sequent_backend.election e
        WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid ORDER BY id",
        &[&tenant, &event],
    )
    .await?;
    Ok(CeremonySources {
        ceremony,
        elections,
    })
}

pub fn resolve_boards(
    session: &Value,
    event: &Value,
    sources: &CeremonySources,
    environment_slug: Option<&str>,
) -> Result<ResolvedBoards> {
    let tenant = id(session, "tenant_id")?;
    let event_id = id(session, "election_event_id")?;
    let ceremony_id = id(session, "keys_ceremony_id")?;
    ensure!(
        id(event, "id")? == event_id && id(event, "tenant_id")? == tenant,
        "saved event ownership differs from ceremony mapping"
    );
    let ceremony = &sources.ceremony;
    ensure!(
        id(ceremony, "id")? == ceremony_id
            && id(ceremony, "tenant_id")? == tenant
            && id(ceremony, "election_event_id")? == event_id,
        "saved keys ceremony differs from tally session"
    );
    let event_board = event["bulletin_board_reference"]["database_name"]
        .as_str()
        .ok_or_else(|| incomplete("saved election event lacks its board name"))?;
    board_identifier(event_board).map_err(|e| invalid_input(e.to_string()))?;
    let default = match ceremony.get("is_default") {
        Some(Value::Null) | Some(Value::Bool(true)) => true,
        Some(Value::Bool(false)) => false,
        _ => {
            return Err(invalid_input(
                "saved keys ceremony lacks its default-board predicate",
            ))
        }
    };
    if default {
        return Ok(ResolvedBoards {
            bulletin_board: event_board.into(),
            electoral_log: event_board.into(),
            election_id: None,
        });
    }
    let slug = environment_slug.ok_or_else(|| {
        invalid_input(
            "non-default keys ceremony requires an independently authenticated --environment-slug",
        )
    })?;
    ensure!(
        !slug.is_empty()
            && slug
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
        invalid_input("invalid environment slug")
    );
    let tenant_part = tenant.replace('-', "")[..17].to_owned();
    // Match Windmill get_event_board/get_election_board exactly, including hyphen removal.
    let expected_event = format!("{slug}tenant{tenant_part}event{event_id}").replace('-', "");
    ensure!(
        expected_event == event_board,
        "environment slug differs from authenticated event board mapping"
    );
    let owners: Vec<_> = sources
        .elections
        .iter()
        .filter(|e| e["keys_ceremony_id"].as_str() == Some(ceremony_id))
        .collect();
    ensure!(
        owners.len() == 1,
        invalid_input("non-default keys ceremony must belong to exactly one event election")
    );
    let election = owners[0];
    ensure!(
        id(election, "tenant_id")? == tenant && id(election, "election_event_id")? == event_id,
        "ceremony election has foreign ownership"
    );
    let election_id = id(election, "id")?;
    if let Some(ids) = session.get("election_ids") {
        ensure!(
            ids.as_array().is_some_and(
                |ids| !ids.is_empty() && ids.iter().all(|v| v.as_str() == Some(election_id))
            ),
            "non-default tally names another election"
        );
    }
    let board = format!("{slug}tenant{tenant_part}election{election_id}").replace('-', "");
    board_identifier(&board).map_err(|e| invalid_input(e.to_string()))?;
    Ok(ResolvedBoards {
        bulletin_board: board,
        electoral_log: event_board.into(),
        election_id: Some(election_id.into()),
    })
}

pub fn validate_scope_election(
    election_id: &str,
    sources: &CeremonySources,
    boards: &ResolvedBoards,
) -> Result<()> {
    let matches: Vec<_> = sources
        .elections
        .iter()
        .filter(|e| e["id"].as_str() == Some(election_id))
        .collect();
    ensure!(
        matches.len() == 1,
        "scope election is absent or ambiguous in the ceremony source"
    );
    let election = matches[0];
    ensure!(
        election["tenant_id"] == sources.ceremony["tenant_id"]
            && election["election_event_id"] == sources.ceremony["election_event_id"],
        "scope election has foreign ownership"
    );
    if let Some(expected) = &boards.election_id {
        ensure!(
            election_id == expected,
            "scope election differs from non-default ceremony owner"
        );
    } else {
        ensure!(
            election["keys_ceremony_id"].is_null()
                || election["keys_ceremony_id"] == sources.ceremony["id"],
            "scope election uses another keys ceremony"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const T: &str = "00000000-0000-4000-8000-000000000010";
    const E: &str = "00000000-0000-4000-8000-000000000020";
    const EL: &str = "00000000-0000-4000-8000-000000000030";
    const C: &str = "00000000-0000-4000-8000-000000000070";
    fn fixture(default: bool) -> (Value, Value, CeremonySources) {
        (
            json!({"tenant_id":T,"election_event_id":E,"keys_ceremony_id":C,"election_ids":[EL]}),
            json!({"id":E,"tenant_id":T,"bulletin_board_reference":{"database_name":format!("testtenant{}event{}",&T.replace('-', "")[..17], E.replace('-', ""))}}),
            CeremonySources {
                ceremony: json!({"id":C,"tenant_id":T,"election_event_id":E,"is_default":default}),
                elections: vec![
                    json!({"id":EL,"tenant_id":T,"election_event_id":E,"keys_ceremony_id":C}),
                ],
            },
        )
    }
    #[test]
    fn default_and_election_boards_keep_event_log_separate() {
        for default in [true, false] {
            let (s, e, c) = fixture(default);
            let b = resolve_boards(&s, &e, &c, Some("test")).unwrap();
            assert_eq!(
                b.electoral_log,
                e["bulletin_board_reference"]["database_name"]
                    .as_str()
                    .unwrap()
            );
            assert_eq!(b.bulletin_board == b.electoral_log, default);
            validate_scope_election(EL, &c, &b).unwrap();
        }
    }
    #[test]
    fn nondefault_requires_matching_environment_and_unique_owned_election() {
        let (s, e, mut c) = fixture(false);
        for slug in [None, Some("wrong"), Some("bad;sql")] {
            assert!(resolve_boards(&s, &e, &c, slug).is_err());
        }
        c.elections.push(c.elections[0].clone());
        assert!(resolve_boards(&s, &e, &c, Some("test")).is_err());
        c.elections.pop();
        c.elections[0]["tenant_id"] = json!(E);
        assert!(resolve_boards(&s, &e, &c, Some("test")).is_err());
    }
    #[test]
    fn foreign_ceremony_tally_or_scope_is_refused() {
        let (mut s, e, c) = fixture(false);
        s["election_ids"] = json!([E]);
        assert!(resolve_boards(&s, &e, &c, Some("test")).is_err());
        let (s, e, mut c) = fixture(false);
        let b = resolve_boards(&s, &e, &c, Some("test")).unwrap();
        assert!(validate_scope_election(E, &c, &b).is_err());
        c.ceremony["election_event_id"] = json!(EL);
        assert!(resolve_boards(&s, &e, &c, Some("test")).is_err());
    }
}
