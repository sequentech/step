// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The supplemental workbook, derived from the SQL when there is none.
//!
//! The workbook adds what the dump does not hold: the event's name and identity,
//! one election context per post, the areas' names, the voting period and the
//! CCS servers. Without it, the contexts and areas come from the source itself and
//! the rest from a small reviewed [`Supplement`] in the decisions. The result is
//! the same sheets the workbook reader produces, so the conversion that follows is
//! the one every workbook goes through.
//!
//! What is derived, and from where:
//!
//! - **Elections**: one per polling center that a `pop` row places voters in,
//!   named and aliased by its `POLLING_PLACE`. A center no `pop` row uses has no
//!   voters and gets no election; it is counted as excluded.
//! - **Areas**: one per precinct cluster (`pop.CLUSTERPOLLCENTER`), named by the
//!   historical rule (the barangay's name, or the municipality's when the barangay
//!   is named after its own post) and described by the cluster's own polling
//!   place, which is the embassy a preserved voter is shown under.
//! - **Event, schedules, CCS servers**: from the supplement. Nothing is invented:
//!   an event without a name or identity is refused, and an election without a
//!   schedule is the same warning a workbook without one gives.

use super::{row, rows_sheet, Data};
use crate::election_config::{
    problem::{Code, Problem},
    sheet::{Row, Workbook},
    sql_dump::SqlDump,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

/// What the workbook would have added, reviewed with the other decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Supplement {
    pub event: SupplementEvent,
    /// Each election is named by its post (`polling_centers.POLLING_PLACE`).
    #[serde(default)]
    pub schedules: Vec<SupplementSchedule>,
    #[serde(default)]
    pub ccs_servers: Vec<SupplementCcsServer>,
    /// Comma-separated CCS server tags every area transmits to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_ccs_server_tags: Option<String>,
    /// Comma-separated trustee users every area's results are signed by.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_trustee_servers: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SupplementEvent {
    pub name: String,
    /// The election event's identity in the source system.
    pub election_event_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SupplementSchedule {
    /// The election's post, as `polling_centers.POLLING_PLACE` spells it; absent,
    /// every election. The elections are derived, so a period that is the same
    /// everywhere can be given before anyone has seen them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub election: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    /// An RFC 3339 instant, or a local date and time read with the decisions'
    /// `scheduleOffsetMinutes`.
    pub date: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SupplementCcsServer {
    pub name: String,
    pub tag: String,
    pub address: String,
    pub public_key: String,
    pub send_logs: bool,
}

/// The derived workbook, and the area row each `pop` row belongs to. Areas are
/// joined by row rather than by name, because two clusters may share a name.
pub(super) struct Derived {
    pub workbook: Workbook,
    pub areas_by_precinct: BTreeMap<usize, usize>,
    /// One per ScheduledEvents row, in order, with the index of the supplement
    /// schedule it came from.
    pub schedules: Vec<(usize, SupplementSchedule)>,
}

pub(super) fn derive(
    dump: &SqlDump,
    supplement: &Supplement,
) -> Result<Derived, Problem> {
    let populations = Data::new(
        dump,
        "pop",
        &[
            "CLUSTERPOLLCENTER",
            "POLLCENTER_CODE",
            "PROV_CODE",
            "MUN_CODE",
        ],
    )?;
    let centers = Data::new(dump, "polling_centers", &["ID", "POLLING_PLACE"])?;
    let barangays = Data::new(dump, "allbgy", &["ID_BARANGAY", "AREANAME"])?;
    let municipalities = Data::new(dump, "allmun", &["ID_CITY", "AREANAME"])?;
    let ci = centers.index("ID")?;
    let bi = barangays.index("ID_BARANGAY")?;
    let mi = municipalities.index("ID_CITY")?;

    let mut posts: Vec<String> = Vec::new();
    let mut areas: Vec<Row> = Vec::new();
    let mut area_by_cluster: BTreeMap<String, usize> = BTreeMap::new();
    let mut areas_by_precinct = BTreeMap::new();
    for pop in 0..populations.table.rows.len() {
        let center = centers.lookup(
            &ci,
            &populations.text(pop, "POLLCENTER_CODE")?,
            pop,
            "POLLCENTER_CODE",
        )?;
        let post = centers.text(center, "POLLING_PLACE")?;
        if !posts.contains(&post) {
            posts.push(post.clone());
        }
        let cluster = populations.text(pop, "CLUSTERPOLLCENTER")?;
        if let Some(&area) = area_by_cluster.get(&cluster) {
            areas_by_precinct.insert(pop, area);
            continue;
        }
        let barangay =
            barangays.lookup(&bi, &cluster, pop, "CLUSTERPOLLCENTER")?;
        // The cluster's own center, when it has one, rather than whichever center
        // (a test one, say) happens to come first.
        let own_post = match ci.get(&cluster) {
            Some(&own) => centers.text(own, "POLLING_PLACE")?,
            None => post,
        };
        let barangay_name = barangays.text(barangay, "AREANAME")?;
        let name = if barangay_name == own_post {
            let city = format!(
                "{}{}",
                populations.text(pop, "PROV_CODE")?,
                populations.text(pop, "MUN_CODE")?
            );
            let municipality =
                municipalities.lookup(&mi, &city, pop, "ID_CITY")?;
            municipalities.text(municipality, "AREANAME")?
        } else {
            barangay_name
        };
        let mut cells =
            vec![("name", json!(name)), ("description", json!(own_post))];
        if let Some(tags) = &supplement.area_ccs_server_tags {
            cells.push(("annotations.miru_ccs_server_tags", json!(tags)));
        }
        if let Some(trustees) = &supplement.area_trustee_servers {
            cells.push(("annotations.miru_trustee_servers", json!(trustees)));
        }
        let index = areas.len();
        areas.push(row("Areas", index, cells));
        area_by_cluster.insert(cluster, index);
        areas_by_precinct.insert(pop, index);
    }

    let event = &supplement.event;
    for (value, field) in [
        (&event.name, "event.name"),
        (&event.election_event_id, "event.electionEventId"),
    ] {
        if value.trim().is_empty() {
            return Err(Problem::error(
                Code::MissingField,
                format!("historical decisions supplement {field}"),
                "required without a supplemental workbook",
            ));
        }
    }
    let mut event_cells = vec![
        ("name", json!(event.name)),
        (
            "annotations.miru_election_event_id",
            json!(event.election_event_id),
        ),
        ("annotations.miru_election_event_name", json!(event.name)),
    ];
    if let Some(description) = &event.description {
        event_cells.push(("description", json!(description)));
    }
    if let Some(logo) = &event.logo_url {
        event_cells.push(("logo_url", json!(logo)));
    }
    if let Some(css) = &event.css {
        event_cells.push(("presentation.css", json!(css)));
    }

    let elections = posts
        .iter()
        .enumerate()
        .map(|(i, post)| {
            row(
                "Elections",
                i,
                vec![
                    ("election_post", json!(post)),
                    ("alias", json!(post)),
                    ("name", json!(post)),
                ],
            )
        })
        .collect();
    let expanded: Vec<(usize, SupplementSchedule)> = supplement
        .schedules
        .iter()
        .enumerate()
        .flat_map(|(source, schedule)| {
            let elections = match &schedule.election {
                Some(post) => vec![post.clone()],
                None => posts.clone(),
            };
            elections.into_iter().map(move |post| {
                (
                    source,
                    SupplementSchedule {
                        election: Some(post),
                        ..schedule.clone()
                    },
                )
            })
        })
        .collect();
    let schedules = expanded
        .iter()
        .enumerate()
        .map(|(i, (_, schedule))| {
            row(
                "ScheduledEvents",
                i,
                vec![
                    ("election_alias", json!(schedule.election)),
                    ("type", json!(schedule.event_type)),
                    ("date", json!(schedule.date)),
                ],
            )
        })
        .collect();
    let servers = supplement
        .ccs_servers
        .iter()
        .enumerate()
        .map(|(i, server)| {
            row(
                "CcsServers",
                i,
                vec![
                    ("name", json!(server.name)),
                    ("tag", json!(server.tag)),
                    ("address", json!(server.address)),
                    ("public_key", json!(server.public_key)),
                    ("send_logs", json!(server.send_logs)),
                ],
            )
        })
        .collect();
    let workbook = Workbook::new(vec![
        rows_sheet("ElectionEvent", vec![row("ElectionEvent", 0, event_cells)]),
        rows_sheet("Elections", elections),
        rows_sheet("Areas", areas),
        rows_sheet("ScheduledEvents", schedules),
        rows_sheet("CcsServers", servers),
    ])?;
    Ok(Derived {
        workbook,
        areas_by_precinct,
        schedules: expanded,
    })
}
