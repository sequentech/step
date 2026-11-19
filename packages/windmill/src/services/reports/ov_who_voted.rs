// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The List of Voters who Actually Voted (`OV_WHO_VOTED`): for every Post
//! (election) of the event, or the one Post its report names, the voters who
//! cast an accepted ballot there, by area, as a PDF and as a CSV of the same
//! rows.
//!
//! Restored from the release/9.0 implementation (CM-39) and adapted:
//!
//! - A voter is listed when the ballot box holds a valid ballot of theirs in
//!   the Post (voter-ID events), or when the event's participation flags say
//!   they voted there (pseudonymous events). A voter who revoted is listed
//!   once. Pending ballots (awaiting an external confirmation) and rejected
//!   ones are not accepted votes; the tally does not count them either.
//! - The list shows who voted, never when or how: no ballot time, channel,
//!   content or ballot ID is read. 2025's "Date voted" column is gone
//!   (VOTE-PRIVACY: a voted list never shows ballot times).
//! - The voters, their areas and the counts are read once per generation, so
//!   the PDF and the CSV list the same rows.
//! - A large list renders its PDF in batches, one PDF each, in a ZIP.

use super::generation::GeneratedFile;
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    ExecutionAnnotations,
};
use super::template_renderer::*;
use super::utils::defuse_formula;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::{get_election_by_id, get_elections};
use crate::postgres::reports::{ReportFormat, ReportType};
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::ballot_box_reads::get_event_ballot_box;
use crate::services::election_dates::get_election_dates;
use crate::services::reports::activity_log::csv_export_writer;
use crate::services::users::count_keycloak_enabled_voters_of_areas;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use electoral_log::adapters::privacy::BallotIdentity;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::election_config::manifest::ConfigurationStamp;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::translations::{Alias, Name};
use sequent_core::types::hasura::core::{Area, Election};
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::OnceCell;
use tracing::instrument;

/// Keycloak user attributes the list reads besides the account's own names.
const MIDDLE_NAME_ATTR_NAME: &str = "middleName";
const SUFFIX_ATTR_NAME: &str = "suffix";
/// Voters looked up in Keycloak per query.
const VOTER_LOOKUP_BATCH: usize = 1000;
/// The name of an area without one.
const NO_AREA: &str = "-";

/// The columns of the CSV, in order. Recovered from the 2025 PDF's columns
/// (No., Name, ID) and the Post and area each list was printed for; the date
/// voted is left out on purpose. COMELEC's database schema for this file is
/// an open decision (VOTE-REPORTS D3): these are not that schema.
pub const CSV_COLUMNS: [&str; 13] = [
    "election_id",
    "election_external_id",
    "election",
    "post",
    "area_id",
    "area",
    "number",
    "last_name",
    "first_name",
    "middle_name",
    "suffix",
    "username",
    "record",
];

/// A voter who voted, as the Keycloak account names them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoterRecord {
    pub voter_id: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: String,
    pub suffix: String,
    /// Every `area-id` the account carries.
    pub area_ids: Vec<String>,
    pub enabled: bool,
    /// The account was found in the realm; a voter the ballot box names
    /// whose account is gone has an empty record.
    pub found: bool,
}

/// What the list knows about one Post before it is ordered.
#[derive(Debug, Clone, Default)]
pub struct PostInput {
    pub election_id: String,
    pub election_external_id: String,
    pub election_title: String,
    pub post: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    /// The Post's areas, `(id, name)`.
    pub areas: Vec<(String, String)>,
    /// Enabled voters of each area: the registered voters the 2025 summary
    /// counted.
    pub registered: HashMap<String, i64>,
    /// The voters with an accepted ballot in the Post, each once.
    pub voter_ids: Vec<String>,
    /// Their accounts; a voter whose account is gone has none.
    pub records: HashMap<String, VoterRecord>,
}

/// One listed voter.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ListedVoter {
    /// Position in the area's list, from 1.
    pub number: usize,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: String,
    pub suffix: String,
    /// The voter ID shown to administrators: the account's username.
    pub username: String,
    /// The ballot box names a voter whose account no longer exists.
    pub record_missing: bool,
}

/// An area's part of a Post's list. The field names are 2025's, so the
/// templates written for it still draw.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UserDataArea {
    pub election_id: String,
    pub election_external_id: String,
    pub election_title: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub post: String,
    pub area_id: String,
    pub area_name: String,
    pub voters: Vec<ListedVoter>,
    /// Voters of the area who voted: all of them, also on a batch that
    /// prints only some.
    pub voted: i64,
    /// Registered (enabled) voters of the area who did not vote.
    pub not_voted: i64,
    /// Registered (enabled) voters of the area, and those who voted with an
    /// account disabled since.
    pub total: i64,
    /// The rows of the area this document prints, `first`..=`last` of `voted`.
    pub first: usize,
    pub last: usize,
}

/// A Post's totals, printed on the list's summary.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PostSummary {
    pub election_id: String,
    pub election_title: String,
    pub post: String,
    pub voted: i64,
    pub areas: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListScope {
    AllPosts,
    Post,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub scope: ListScope,
    pub posts: Vec<PostSummary>,
    pub areas: Vec<UserDataArea>,
    /// Voters listed, across every Post.
    pub voted_total: i64,
    /// This document's part when the PDF is printed in batches.
    pub batch: Option<BatchInfo>,
    pub execution_annotations: ExecutionAnnotations,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BatchInfo {
    pub number: i64,
    pub total: i64,
    pub first_row: i64,
    pub last_row: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// The system template's data. It loads no script: the list names voters,
/// so it is rendered with scripts disabled.
pub struct SystemData {
    pub rendered_user_template: String,
}

/// Every Post's list, ordered: Posts by title, areas by name, voters by
/// name. Voters without an area of the Post close it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VotedList {
    pub scope: Option<ListScope>,
    pub posts: Vec<PostSummary>,
    pub areas: Vec<UserDataArea>,
}

fn name_key(voter: &VoterRecord) -> [String; 4] {
    [
        voter.last_name.to_lowercase(),
        voter.first_name.to_lowercase(),
        voter.middle_name.to_lowercase(),
        voter.suffix.to_lowercase(),
    ]
}

/// The order voters are listed in: by name, ignoring case, then by the
/// exact names, the username and the voter's ID, so that equal names still
/// list the same way every time.
fn compare_voters(a: &VoterRecord, b: &VoterRecord) -> Ordering {
    name_key(a)
        .cmp(&name_key(b))
        .then_with(|| a.last_name.cmp(&b.last_name))
        .then_with(|| a.first_name.cmp(&b.first_name))
        .then_with(|| a.username.cmp(&b.username))
        .then_with(|| a.voter_id.cmp(&b.voter_id))
}

/// The Post's name: its `post` annotation, or its title when the election
/// has none (the annotation reads `-` then).
pub fn post_name(annotation: &str, title: &str) -> String {
    match annotation.trim() {
        "" | NO_AREA => title.to_string(),
        post => post.to_string(),
    }
}

/// One Post's areas with their voters. A voter is listed in the first of
/// the Post's areas (by name) their account carries; one in none of them in
/// a last, unnamed area.
pub fn post_areas(input: &PostInput) -> Vec<UserDataArea> {
    let mut areas: Vec<(String, String)> = input.areas.clone();
    areas.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let mut voters_by_area: HashMap<String, Vec<VoterRecord>> = HashMap::new();
    let mut unplaced: Vec<VoterRecord> = Vec::new();
    for voter_id in &input.voter_ids {
        let record = input
            .records
            .get(voter_id)
            .cloned()
            .unwrap_or_else(|| VoterRecord {
                voter_id: voter_id.clone(),
                ..Default::default()
            });
        match areas
            .iter()
            .find(|(area_id, _)| record.area_ids.contains(area_id))
        {
            Some((area_id, _)) => voters_by_area
                .entry(area_id.clone())
                .or_default()
                .push(record),
            None => unplaced.push(record),
        }
    }

    let area = |area_id: &str, area_name: &str, mut voters: Vec<VoterRecord>| {
        voters.sort_by(compare_voters);
        let voted = voters.len() as i64;
        let disabled_voted = voters.iter().filter(|voter| !voter.enabled).count() as i64;
        let registered = input.registered.get(area_id).copied().unwrap_or(0);
        let enabled_voted = voted - disabled_voted;
        UserDataArea {
            election_id: input.election_id.clone(),
            election_external_id: input.election_external_id.clone(),
            election_title: input.election_title.clone(),
            election_dates: input.election_dates.clone(),
            post: input.post.clone(),
            area_id: area_id.to_string(),
            area_name: area_name.to_string(),
            voted,
            not_voted: (registered - enabled_voted).max(0),
            total: registered.max(enabled_voted) + disabled_voted,
            first: usize::from(!voters.is_empty()),
            last: voters.len(),
            voters: voters
                .into_iter()
                .enumerate()
                .map(|(index, voter)| ListedVoter {
                    number: index + 1,
                    record_missing: !voter.found,
                    last_name: voter.last_name,
                    first_name: voter.first_name,
                    middle_name: voter.middle_name,
                    suffix: voter.suffix,
                    username: voter.username,
                })
                .collect(),
        }
    };

    let mut listed: Vec<UserDataArea> = areas
        .iter()
        .map(|(area_id, area_name)| {
            area(
                area_id,
                area_name,
                voters_by_area.remove(area_id).unwrap_or_default(),
            )
        })
        .collect();
    if !unplaced.is_empty() {
        // Voters whose account is in no area of the Post: no area ID or name.
        listed.push(area("", "", unplaced));
    }
    listed
}

impl VotedList {
    /// Every Post's list, the Posts ordered by title then ID.
    pub fn build(scope: ListScope, mut posts: Vec<PostInput>) -> Self {
        posts.sort_by(|a, b| {
            a.election_title
                .cmp(&b.election_title)
                .then_with(|| a.election_id.cmp(&b.election_id))
        });
        let mut list = VotedList {
            scope: Some(scope),
            ..Default::default()
        };
        for post in &posts {
            let areas = post_areas(post);
            list.posts.push(PostSummary {
                election_id: post.election_id.clone(),
                election_title: post.election_title.clone(),
                post: post.post.clone(),
                voted: areas.iter().map(|area| area.voted).sum(),
                areas: areas.len(),
            });
            list.areas.extend(areas);
        }
        list
    }

    /// Voters listed across every Post: the rows of the CSV.
    pub fn rows(&self) -> i64 {
        self.areas.iter().map(|area| area.voted).sum()
    }

    /// The areas, cut to rows `offset`..`offset + limit` of the whole list.
    /// An area keeps its totals; `first` and `last` say which of its rows
    /// the part holds. Areas without voters are printed on the first part.
    pub fn page(&self, offset: i64, limit: i64) -> Vec<UserDataArea> {
        let start = offset.max(0) as usize;
        let end = start.saturating_add(limit.max(0) as usize);
        let mut seen = 0usize;
        let mut page = Vec::new();
        for area in &self.areas {
            let (from, to) = (seen, seen + area.voters.len());
            seen = to;
            if area.voters.is_empty() {
                if start == 0 {
                    page.push(area.clone());
                }
                continue;
            }
            let (cut_from, cut_to) = (from.max(start), to.min(end));
            if cut_from >= cut_to {
                continue;
            }
            let mut part = area.clone();
            part.voters = area.voters[cut_from - from..cut_to - from].to_vec();
            part.first = cut_from - from + 1;
            part.last = cut_to - from;
            page.push(part);
        }
        page
    }

    /// The CSV's rows: one per listed voter, in the PDF's order.
    pub fn csv_records(&self) -> impl Iterator<Item = [String; 13]> + '_ {
        self.areas.iter().flat_map(|area| {
            area.voters.iter().map(move |voter| {
                [
                    area.election_id.clone(),
                    area.election_external_id.clone(),
                    area.election_title.clone(),
                    area.post.clone(),
                    area.area_id.clone(),
                    area.area_name.clone(),
                    voter.number.to_string(),
                    voter.last_name.clone(),
                    voter.first_name.clone(),
                    voter.middle_name.clone(),
                    voter.suffix.clone(),
                    voter.username.clone(),
                    if voter.record_missing {
                        "missing"
                    } else {
                        "found"
                    }
                    .to_string(),
                ]
                .map(|cell| defuse_formula(&cell))
            })
        })
    }

    /// Writes the CSV: the configuration's stamp line when there is one, the
    /// header, then a row per voter. RFC 4180 quoting; a cell a spreadsheet
    /// would run as a formula is written after a `'`.
    pub fn write_csv<W: std::io::Write>(
        &self,
        writer: W,
        stamp: Option<&ConfigurationStamp>,
    ) -> Result<W> {
        let mut csv = csv_export_writer(writer, stamp)?;
        csv.write_record(CSV_COLUMNS)
            .context("Error writing the CSV header")?;
        for record in self.csv_records() {
            csv.write_record(&record)
                .context("Error writing a CSV row")?;
        }
        csv.into_inner()
            .map_err(|error| anyhow!("Error finishing the CSV: {error}"))
    }
}

/// The voters a Post's list names must be exactly those with an accepted
/// ballot there. `accepted` counts the distinct identities with a valid
/// ballot in the ballot box; `flagged` the participation flags the names come
/// from (the same number in a voter-ID event); `listed` the voters named.
///
/// A flag is set by a valid or a pending ballot and never removed, and every
/// valid ballot sets one, so equal counts mean the same voters. More flags
/// mean a pending or rejected ballot, which the flags cannot tell from an
/// accepted one without resolving pseudonyms; fewer named voters mean the
/// identity mapping is incomplete. Either way no list is printed: a wrong
/// list would be an official statement about who voted.
pub fn check_population(post: &str, accepted: i64, flagged: i64, listed: usize) -> Result<()> {
    if flagged != accepted {
        return Err(anyhow!(
            "Post {post}: {flagged} voters are marked as having voted but {accepted} have an \
             accepted ballot. The difference is pending or rejected ballots, which cannot be \
             told apart without resolving pseudonyms, so the list is not generated"
        ));
    }
    if i64::try_from(listed)? != accepted {
        return Err(anyhow!(
            "Post {post}: {accepted} voters have an accepted ballot but {listed} can be named: \
             the voter identity mapping is incomplete, so the list is not generated"
        ));
    }
    Ok(())
}

/// Looks the voters up in the event's realm: names, username, whether the
/// account is enabled and its areas. No other attribute is read.
#[instrument(skip_all, err)]
pub async fn fetch_voter_records(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    voter_ids: &[String],
) -> Result<HashMap<String, VoterRecord>> {
    let statement = keycloak_transaction
        .prepare(&format!(
            r#"
            SELECT
                u.id,
                u.username,
                COALESCE(u.first_name, '') AS first_name,
                COALESCE(u.last_name, '') AS last_name,
                COALESCE(u.enabled, false) AS enabled,
                COALESCE(
                    array_agg(ua.value ORDER BY ua.value)
                        FILTER (WHERE ua.name = '{AREA_ID_ATTR_NAME}' AND ua.value IS NOT NULL),
                    '{{}}'
                ) AS area_ids,
                COALESCE(min(ua.value) FILTER (WHERE ua.name = '{MIDDLE_NAME_ATTR_NAME}'), '')
                    AS middle_name,
                COALESCE(min(ua.value) FILTER (WHERE ua.name = '{SUFFIX_ATTR_NAME}'), '')
                    AS suffix
            FROM user_entity AS u
            JOIN realm AS ra ON ra.id = u.realm_id
            LEFT JOIN user_attribute AS ua
                ON ua.user_id = u.id
                AND ua.name IN ('{AREA_ID_ATTR_NAME}', '{MIDDLE_NAME_ATTR_NAME}', '{SUFFIX_ATTR_NAME}')
            WHERE ra.name = $1
                AND u.id = ANY($2)
                AND u.service_account_client_link IS NULL
            GROUP BY u.id, u.username, u.first_name, u.last_name, u.enabled
            "#
        ))
        .await
        .context("Error preparing the voters' lookup")?;
    let mut records = HashMap::with_capacity(voter_ids.len());
    for chunk in voter_ids.chunks(VOTER_LOOKUP_BATCH) {
        let rows = keycloak_transaction
            .query(&statement, &[&realm, &chunk])
            .await
            .context("Error looking the voters up")?;
        for row in rows {
            let record = VoterRecord {
                voter_id: row.try_get("id")?,
                username: row
                    .try_get::<_, Option<String>>("username")?
                    .unwrap_or_default(),
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name")?,
                middle_name: row.try_get("middle_name")?,
                suffix: row.try_get("suffix")?,
                area_ids: row.try_get("area_ids")?,
                enabled: row.try_get("enabled")?,
                found: true,
            };
            records.insert(record.voter_id.clone(), record);
        }
    }
    Ok(records)
}

#[derive(Debug)]
pub struct OVUsersWhoVotedTemplate {
    ids: ReportOrigins,
    list: OnceCell<VotedList>,
}

impl OVUsersWhoVotedTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        OVUsersWhoVotedTemplate {
            ids,
            list: OnceCell::new(),
        }
    }

    fn scope(&self) -> ListScope {
        if self.ids.election_id.is_some() {
            ListScope::Post
        } else {
            ListScope::AllPosts
        }
    }

    /// The Posts the list covers: the report's, or every Post of the event.
    async fn posts(&self, hasura_transaction: &Transaction<'_>) -> Result<Vec<Election>> {
        match &self.ids.election_id {
            Some(election_id) => Ok(vec![get_election_by_id(
                hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                election_id,
            )
            .await
            .context("Error reading the report's election")?
            .ok_or_else(|| anyhow!("The report's election {election_id} does not exist"))?]),
            None => {
                let elections = get_elections(
                    hasura_transaction,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                )
                .await
                .context("Error reading the event's elections")?;
                if elections.is_empty() {
                    return Err(anyhow!("The election event has no elections to list"));
                }
                Ok(elections)
            }
        }
    }

    /// The list, read once: the PDF, its batches and the CSV share it.
    async fn voted_list(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<&VotedList> {
        self.list
            .get_or_try_init(|| self.read_list(hasura_transaction, keycloak_transaction))
            .await
    }

    #[instrument(skip_all, err)]
    async fn read_list(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<VotedList> {
        let tenant_id = &self.ids.tenant_id;
        let event_id = &self.ids.election_event_id;
        let realm = get_event_realm(tenant_id, event_id);
        let elections = self.posts(hasura_transaction).await?;
        let scheduled_events =
            find_scheduled_event_by_election_event_id(hasura_transaction, tenant_id, event_id)
                .await
                .context("Error reading the event's scheduled events")?;
        let store = get_event_ballot_box(hasura_transaction, tenant_id, event_id)
            .await?
            .store;
        let pseudonymous = store.ballot_identity(event_id).await? == BallotIdentity::Pseudonym;
        if pseudonymous && store.identity_mapping_purged(event_id).await? {
            return Err(anyhow!(
                "The event's voter identity mapping was purged: who voted can no longer be listed"
            ));
        }

        let mut posts = Vec::with_capacity(elections.len());
        for election in elections {
            let language = election.get_default_language();
            let election_title = election
                .get_alias(&language)
                .unwrap_or_else(|| election.get_name(&language));
            let post = post_name(
                &extract_election_data(&election).await?.post,
                &election_title,
            );
            let areas: Vec<Area> =
                get_areas_by_election_id(hasura_transaction, tenant_id, event_id, &election.id)
                    .await
                    .context("Error reading the Post's areas")?;
            // Keycloak names an election without an external ID by its ID.
            let authorization_alias = election
                .external_id
                .clone()
                .filter(|alias| !alias.is_empty())
                .unwrap_or_else(|| election.id.clone());
            let mut registered = HashMap::with_capacity(areas.len());
            for area in &areas {
                let count = count_keycloak_enabled_voters_of_areas(
                    keycloak_transaction,
                    &realm,
                    std::slice::from_ref(&area.id),
                    &[],
                    &authorization_alias,
                )
                .await?;
                registered.insert(area.id.clone(), i64::try_from(count)?);
            }
            // Who voted: never when, how or with which ballot. Pseudonymous
            // events name their voters only through the participation flags.
            let accepted = store
                .participation(event_id, Some(election.id.as_str()), None)
                .await?
                .voters;
            let (mut voter_ids, flagged) = if pseudonymous {
                (
                    store
                        .participating_voters(event_id, Some(election.id.as_str()))
                        .await?,
                    store
                        .privacy_participation_count(event_id, Some(election.id.as_str()))
                        .await?,
                )
            } else {
                (
                    store.accepted_voters(event_id, &election.id).await?,
                    accepted,
                )
            };
            voter_ids.sort();
            voter_ids.dedup();
            check_population(&election_title, accepted, flagged, voter_ids.len())?;
            let records = fetch_voter_records(keycloak_transaction, &realm, &voter_ids).await?;
            posts.push(PostInput {
                election_dates: get_election_dates(&election, scheduled_events.clone()).ok(),
                election_id: election.id.clone(),
                election_external_id: election.external_id.clone().unwrap_or_default(),
                election_title,
                post,
                areas: areas
                    .iter()
                    .map(|area| {
                        (
                            area.id.clone(),
                            area.name.clone().unwrap_or_else(|| NO_AREA.to_string()),
                        )
                    })
                    .collect(),
                registered,
                voter_ids,
                records,
            });
        }
        Ok(VotedList::build(self.scope(), posts))
    }

    async fn user_data(
        &self,
        list: &VotedList,
        areas: Vec<UserDataArea>,
        batch: Option<BatchInfo>,
    ) -> Result<UserData> {
        let app_version = get_app_version();
        Ok(UserData {
            scope: self.scope(),
            posts: list.posts.clone(),
            areas,
            voted_total: list.rows(),
            batch,
            execution_annotations: ExecutionAnnotations {
                date_printed: get_date_and_time(),
                report_hash: get_report_hash(&ReportType::OV_WHO_VOTED.to_string())
                    .await
                    .unwrap_or_else(|_| "-".to_string()),
                software_version: app_version.clone(),
                app_version,
                app_hash: get_app_hash(),
                executer_username: self.ids.executer_username.clone(),
                results_hash: None,
            },
        })
    }
}

#[async_trait]
impl TemplateRenderer for OVUsersWhoVotedTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::OV_WHO_VOTED
    }

    fn get_tenant_id(&self) -> String {
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_initial_template_alias(&self) -> Option<String> {
        self.ids.template_alias.clone()
    }

    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn get_election_id(&self) -> Option<String> {
        self.ids.election_id.clone()
    }

    /// It lists voters by name.
    fn contains_sensitive_data(&self) -> bool {
        true
    }

    /// A list of named voters stays in the document store, behind its
    /// permissions: it is never mailed, also when it is scheduled.
    fn should_send_email(&self, _is_scheduled_task: bool) -> bool {
        false
    }

    fn requested_by(&self) -> Option<String> {
        self.ids.executer_username.clone()
    }

    fn base_name(&self) -> String {
        "ov_who_voted".to_string()
    }

    fn prefix(&self) -> String {
        match &self.ids.election_id {
            Some(election_id) => format!(
                "ov_who_voted_{}_{}",
                self.ids.election_event_id, election_id
            ),
            None => format!("ov_who_voted_{}_all_posts", self.ids.election_event_id),
        }
    }

    async fn count_items(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Option<i64>> {
        Ok(Some(
            self.voted_list(hasura_transaction, keycloak_transaction)
                .await?
                .rows(),
        ))
    }

    /// One PDF while the list fits in a document; batches beyond that.
    fn renders_pdf_in_batches(&self, items_count: i64, per_report_limit: i64) -> bool {
        items_count > per_report_limit
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data_batch(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        offset: &mut i64,
        limit: i64,
    ) -> Result<Self::UserData> {
        let list = self
            .voted_list(hasura_transaction, keycloak_transaction)
            .await?;
        let rows = list.rows();
        let limit = limit.max(1);
        let batch = BatchInfo {
            number: *offset / limit + 1,
            total: ((rows + limit - 1) / limit).max(1),
            first_row: (*offset + 1).min(rows),
            last_row: (*offset + limit).min(rows),
        };
        self.user_data(list, list.page(*offset, limit), Some(batch))
            .await
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let list = self
            .voted_list(hasura_transaction, keycloak_transaction)
            .await?;
        self.user_data(list, list.areas.clone(), None).await
    }

    #[instrument(err, skip_all)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
        })
    }

    #[instrument(err, skip_all)]
    async fn write_format(
        &self,
        format: ReportFormat,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        stamp: Option<&ConfigurationStamp>,
        directory: &Path,
    ) -> Result<Option<GeneratedFile>> {
        if format != ReportFormat::Csv {
            return Ok(None);
        }
        let list = self
            .voted_list(hasura_transaction, keycloak_transaction)
            .await?;
        let name = format!("{}.{}", self.prefix(), format.extension());
        let path = directory.join(&name);
        let file =
            std::fs::File::create(&path).with_context(|| format!("Error creating {name}"))?;
        let mut file = list.write_csv(std::io::BufWriter::new(file), stamp)?;
        std::io::Write::flush(&mut file).with_context(|| format!("Error writing {name}"))?;
        Ok(Some(GeneratedFile {
            name,
            path: path.to_string_lossy().to_string(),
            media_type: format.media_type().to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voter(id: &str, last: &str, first: &str, area: &str) -> VoterRecord {
        VoterRecord {
            voter_id: id.to_string(),
            username: format!("user-{id}"),
            first_name: first.to_string(),
            last_name: last.to_string(),
            area_ids: vec![area.to_string()],
            enabled: true,
            found: true,
            ..Default::default()
        }
    }

    fn post(id: &str, title: &str, areas: &[(&str, &str)], voters: Vec<VoterRecord>) -> PostInput {
        PostInput {
            election_id: id.to_string(),
            election_external_id: format!("ext-{id}"),
            election_title: title.to_string(),
            post: format!("{title} Post"),
            election_dates: None,
            areas: areas
                .iter()
                .map(|(id, name)| (id.to_string(), name.to_string()))
                .collect(),
            registered: areas.iter().map(|(id, _)| (id.to_string(), 5)).collect(),
            voter_ids: voters.iter().map(|voter| voter.voter_id.clone()).collect(),
            records: voters
                .into_iter()
                .map(|voter| (voter.voter_id.clone(), voter))
                .collect(),
        }
    }

    fn names(area: &UserDataArea) -> Vec<(usize, &str)> {
        area.voters
            .iter()
            .map(|voter| (voter.number, voter.last_name.as_str()))
            .collect()
    }

    #[test]
    fn every_post_is_listed_with_its_areas_and_voters_by_name() {
        let list = VotedList::build(
            ListScope::AllPosts,
            vec![
                post(
                    "p2",
                    "Zeta",
                    &[("a3", "Gamma")],
                    vec![voter("v4", "Ölçer", "Ayşe", "a3")],
                ),
                post(
                    "p1",
                    "Alpha",
                    &[("a2", "Beta"), ("a1", "Alpha")],
                    vec![
                        voter("v1", "dela Cruz", "Juan", "a1"),
                        voter("v2", "Abad", "María", "a1"),
                        voter("v3", "Santos", "José", "a2"),
                    ],
                ),
                post("p3", "Mu", &[("a4", "Delta")], vec![]),
            ],
        );
        assert_eq!(
            list.posts
                .iter()
                .map(|post| (post.election_title.as_str(), post.voted, post.areas))
                .collect::<Vec<_>>(),
            [("Alpha", 3, 2), ("Mu", 0, 1), ("Zeta", 1, 1)]
        );
        assert_eq!(
            list.areas
                .iter()
                .map(|area| area.area_name.as_str())
                .collect::<Vec<_>>(),
            ["Alpha", "Beta", "Delta", "Gamma"]
        );
        // Case is ignored: "dela Cruz" comes after "Abad".
        assert_eq!(names(&list.areas[0]), [(1, "Abad"), (2, "dela Cruz")]);
        assert_eq!(list.areas[0].voted, 2);
        assert_eq!(list.areas[0].not_voted, 3);
        assert_eq!(list.areas[0].total, 5);
        // A Post nobody voted in still prints, empty.
        assert!(list.areas[2].voters.is_empty());
        assert_eq!((list.areas[2].first, list.areas[2].last), (0, 0));
        assert_eq!(list.rows(), 4);
    }

    #[test]
    fn a_voter_outside_the_posts_areas_or_without_an_account_is_still_listed() {
        let mut input = post(
            "p1",
            "Alpha",
            &[("a1", "Alpha")],
            vec![voter("v1", "Abad", "María", "elsewhere")],
        );
        // The ballot box names a voter whose account is gone.
        input.voter_ids.push("v-gone".to_string());
        let list = VotedList::build(ListScope::Post, vec![input]);
        assert_eq!(list.rows(), 2);
        let unplaced = list.areas.last().unwrap();
        assert_eq!(unplaced.area_name, "");
        assert_eq!(unplaced.area_id, "");
        assert_eq!(unplaced.voted, 2);
        assert!(unplaced.voters.iter().any(|voter| voter.record_missing));
        // The voter's internal ID is never printed for a missing account.
        assert!(unplaced
            .voters
            .iter()
            .all(|voter| !voter.username.contains("v-gone")));
    }

    #[test]
    fn a_voter_with_several_areas_is_listed_once_in_the_first_by_name() {
        let mut both = voter("v1", "Abad", "María", "a2");
        both.area_ids.push("a1".to_string());
        let list = VotedList::build(
            ListScope::Post,
            vec![post(
                "p1",
                "Alpha",
                &[("a1", "North"), ("a2", "Middle")],
                vec![both],
            )],
        );
        assert_eq!(list.rows(), 1);
        assert_eq!(list.areas[0].area_name, "Middle");
        assert_eq!(list.areas[0].voted, 1);
        assert_eq!(list.areas[1].voted, 0);
    }

    #[test]
    fn a_disabled_voter_who_voted_counts_in_the_total_but_not_as_registered() {
        let mut disabled = voter("v1", "Abad", "María", "a1");
        disabled.enabled = false;
        let mut input = post("p1", "Alpha", &[("a1", "North")], vec![disabled]);
        input.registered.insert("a1".to_string(), 2);
        let list = VotedList::build(ListScope::Post, vec![input]);
        let area = &list.areas[0];
        assert_eq!((area.voted, area.not_voted, area.total), (1, 2, 3));
    }

    #[test]
    fn equal_names_list_in_the_same_order_every_time() {
        let twins = || {
            vec![
                voter("v2", "Reyes", "Ana", "a1"),
                voter("v1", "Reyes", "Ana", "a1"),
                voter("v3", "reyes", "ana", "a1"),
            ]
        };
        let first = VotedList::build(
            ListScope::Post,
            vec![post("p1", "Alpha", &[("a1", "North")], twins())],
        );
        let mut reversed = twins();
        reversed.reverse();
        let second = VotedList::build(
            ListScope::Post,
            vec![post("p1", "Alpha", &[("a1", "North")], reversed)],
        );
        assert_eq!(first, second);
        assert_eq!(
            first.areas[0]
                .voters
                .iter()
                .map(|voter| voter.username.as_str())
                .collect::<Vec<_>>(),
            ["user-v1", "user-v2", "user-v3"]
        );
    }

    fn three_areas() -> VotedList {
        VotedList::build(
            ListScope::AllPosts,
            vec![post(
                "p1",
                "Alpha",
                &[("a1", "A"), ("a2", "B"), ("a3", "C")],
                vec![
                    voter("v1", "A1", "x", "a1"),
                    voter("v2", "A2", "x", "a1"),
                    voter("v3", "A3", "x", "a1"),
                    voter("v4", "C1", "x", "a3"),
                    voter("v5", "C2", "x", "a3"),
                ],
            )],
        )
    }

    #[test]
    fn the_batches_cover_every_row_once_and_keep_each_areas_totals() {
        let list = three_areas();
        let first = list.page(0, 2);
        assert_eq!(first.len(), 2, "area A's first rows and empty area B");
        assert_eq!(names(&first[0]), [(1, "A1"), (2, "A2")]);
        assert_eq!((first[0].first, first[0].last, first[0].voted), (1, 2, 3));
        assert!(first[1].voters.is_empty());

        let second = list.page(2, 2);
        assert_eq!(second.len(), 2);
        assert_eq!(names(&second[0]), [(3, "A3")]);
        assert_eq!((second[0].first, second[0].last), (3, 3));
        assert_eq!(names(&second[1]), [(1, "C1")]);

        let third = list.page(4, 2);
        assert_eq!(names(&third[0]), [(2, "C2")]);
        assert!(list.page(6, 2).is_empty());

        let all: usize = (0..3)
            .map(|batch| {
                list.page(batch * 2, 2)
                    .iter()
                    .map(|area| area.voters.len())
                    .sum::<usize>()
            })
            .sum();
        assert_eq!(all as i64, list.rows());
    }

    fn csv(list: &VotedList) -> String {
        String::from_utf8(list.write_csv(Vec::new(), None).unwrap()).unwrap()
    }

    #[test]
    fn the_csv_lists_the_pdfs_rows_with_unicode_quoting_and_no_formulas() {
        let mut tricky = voter("v1", "=HYPERLINK(\"x\")", "Ñiño, \"Jr\"", "a1");
        tricky.middle_name = "李".to_string();
        tricky.suffix = "@sum".to_string();
        let list = VotedList::build(
            ListScope::Post,
            vec![post("p1", "Alpha", &[("a1", "Región Ñ")], vec![tricky])],
        );
        let text = csv(&list);
        let mut reader = csv::Reader::from_reader(text.as_bytes());
        assert_eq!(
            reader.headers().unwrap().iter().collect::<Vec<_>>(),
            CSV_COLUMNS
        );
        let rows: Vec<csv::StringRecord> = reader.records().map(Result::unwrap).collect();
        assert_eq!(rows.len() as i64, list.rows());
        let row: Vec<&str> = rows[0].iter().collect();
        assert_eq!(
            row,
            [
                "p1",
                "ext-p1",
                "Alpha",
                "Alpha Post",
                "a1",
                "Región Ñ",
                "1",
                "'=HYPERLINK(\"x\")",
                "Ñiño, \"Jr\"",
                "李",
                "'@sum",
                "user-v1",
                "found"
            ]
        );
        // Quotes and commas are quoted the RFC 4180 way.
        assert!(text.contains("\"Ñiño, \"\"Jr\"\"\""));
    }

    #[test]
    fn a_missing_account_is_a_csv_row_of_its_own_with_no_dash_area() {
        let mut input = post("p1", "Alpha", &[("a1", "North")], vec![]);
        input.voter_ids = vec!["gone-1".to_string(), "gone-2".to_string()];
        let list = VotedList::build(ListScope::Post, vec![input]);
        let text = csv(&list);
        let mut reader = csv::Reader::from_reader(text.as_bytes());
        let rows: Vec<csv::StringRecord> = reader.records().map(Result::unwrap).collect();
        assert_eq!(rows.len(), 2);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(&row[4], "", "no area ID");
            assert_eq!(&row[5], "", "no area name, and no '-");
            assert_eq!(&row[6], (index + 1).to_string());
            assert_eq!(&row[11], "", "the internal ID is not printed");
            assert_eq!(&row[12], "missing");
        }
        assert!(!text.contains("gone-"));
    }

    #[test]
    fn an_account_without_a_username_is_found_not_missing() {
        let mut nameless = voter("v1", "Abad", "María", "a1");
        nameless.username = String::new();
        let list = VotedList::build(
            ListScope::Post,
            vec![post("p1", "Alpha", &[("a1", "North")], vec![nameless])],
        );
        assert!(!list.areas[0].voters[0].record_missing);
        assert!(csv(&list).trim_end().ends_with(",found"));
    }

    #[test]
    fn a_post_without_a_post_annotation_is_named_after_its_title() {
        assert_eq!(post_name("-", "Sample Post 1"), "Sample Post 1");
        assert_eq!(post_name("  ", "Sample Post 1"), "Sample Post 1");
        assert_eq!(post_name("MADRID PE", "Sample Post 1"), "MADRID PE");
    }

    #[test]
    fn the_template_names_the_scope_and_a_continued_part() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.devcontainer/minio/public-assets");
        let template = std::fs::read_to_string(root.join("ov_who_voted_user.hbs")).unwrap();
        let draw = |scope: ListScope, areas: Vec<UserDataArea>| {
            let data = serde_json::json!({"scope": scope, "posts": [], "areas": areas,
                "voted_total": 5, "execution_annotations": {}});
            sequent_core::services::reports::render_template_text(
                &template,
                data.as_object().unwrap().clone(),
            )
            .unwrap()
        };
        let list = three_areas();
        let whole = draw(ListScope::AllPosts, list.areas.clone());
        assert!(whole.contains("All Posts"));
        assert!(!whole.contains("Continued"));
        let one = draw(ListScope::Post, list.page(2, 2));
        assert!(one.contains("One Post"));
        assert!(one.contains("Continued: rows 3 to 3 of 3"));
    }

    #[test]
    fn a_list_is_printed_only_when_its_voters_are_the_accepted_ones() {
        // A voter-ID Post, or a pseudonymous one whose flags are all accepted.
        assert!(check_population("A", 3, 3, 3).is_ok());
        assert!(check_population("Empty", 0, 0, 0).is_ok());
        // A pending or later-rejected ballot left a flag.
        let pending = check_population("A", 1, 3, 3).unwrap_err().to_string();
        assert!(pending.contains("3 voters are marked as having voted but 1 have an accepted"));
        // The mapping lost voters (a partial or complete purge).
        let purged = check_population("A", 2, 2, 0).unwrap_err().to_string();
        assert!(purged.contains("2 voters have an accepted ballot but 0 can be named"));
    }

    #[test]
    fn the_list_is_never_mailed() {
        let list = OVUsersWhoVotedTemplate::new(origins(None));
        assert!(!list.should_send_email(true));
        assert!(!list.should_send_email(false));
    }

    #[test]
    fn a_2025_template_still_draws_without_the_date_voted() {
        // The 2025 default printed each voter's date voted; the data no
        // longer has it, and the template still draws, the cell empty.
        let template = r#"{{#each areas}}{{#each voters}}<td>{{last_name}}</td><td>{{username}}</td><td>{{datetime_zone date_voted output_format="%B %d, %Y"}}</td>{{/each}}<td>{{voting_privilege_voted}}</td>{{/each}}"#;
        let list = three_areas();
        let data = serde_json::json!({"areas": list.areas});
        let rendered = sequent_core::services::reports::render_template_text(
            template,
            data.as_object().unwrap().clone(),
        )
        .unwrap();
        assert!(rendered.contains("<td>A1</td><td>user-v1</td>"));
    }

    #[test]
    fn neither_format_names_a_ballot_time_or_choice() {
        let list = three_areas();
        let text = csv(&list).to_lowercase();
        let data = serde_json::to_string(&list.areas).unwrap().to_lowercase();
        for leaked in [
            "date_voted",
            "voted_at",
            "accepted_at",
            "ballot",
            "channel",
            "content",
        ] {
            assert!(!text.contains(leaked), "CSV names {leaked}");
            assert!(!data.contains(leaked), "PDF data names {leaked}");
        }
    }

    #[test]
    fn an_empty_list_writes_only_its_header() {
        let list = VotedList::build(
            ListScope::AllPosts,
            vec![post("p1", "Alpha", &[("a1", "North")], vec![])],
        );
        assert_eq!(csv(&list).lines().count(), 1);
        assert_eq!(list.page(0, 10).len(), 1);
    }

    fn origins(election_id: Option<&str>) -> ReportOrigins {
        ReportOrigins {
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            election_id: election_id.map(str::to_string),
            template_alias: None,
            voter_id: None,
            report_origin: ReportOriginatedFrom::ReportsTab,
            executer_username: Some("admin".to_string()),
            tally_session_id: None,
        }
    }

    #[test]
    fn the_report_is_named_for_all_posts_or_its_post_and_batches_when_large() {
        let all = OVUsersWhoVotedTemplate::new(origins(None));
        assert_eq!(all.get_report_type(), ReportType::OV_WHO_VOTED);
        assert_eq!(all.scope(), ListScope::AllPosts);
        assert_eq!(all.prefix(), "ov_who_voted_event_all_posts");
        assert!(all.contains_sensitive_data());
        assert!(!all.renders_pdf_in_batches(1000, 1000));
        assert!(all.renders_pdf_in_batches(1001, 1000));
        let one = OVUsersWhoVotedTemplate::new(origins(Some("post")));
        assert_eq!(one.scope(), ListScope::Post);
        assert_eq!(one.prefix(), "ov_who_voted_event_post");
        assert_eq!(
            one.output_formats(None).unwrap(),
            vec![ReportFormat::Pdf, ReportFormat::Csv],
            "a report that names no format writes its PDF and its CSV"
        );
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL configured through KEYCLOAK_DB__*"]
    async fn the_voters_are_looked_up_in_their_realm_with_their_names_and_areas() -> Result<()> {
        let pool = crate::services::database::generate_keycloak_pool().await?;
        let mut client = pool.get().await?;
        let transaction = client.transaction().await?;
        transaction
            .batch_execute(&format!(
                r#"
                CREATE TEMP TABLE realm (id TEXT PRIMARY KEY, name TEXT) ON COMMIT DROP;
                CREATE TEMP TABLE user_entity (
                    id TEXT PRIMARY KEY, realm_id TEXT, username TEXT, first_name TEXT,
                    last_name TEXT, enabled BOOLEAN, service_account_client_link TEXT
                ) ON COMMIT DROP;
                CREATE TEMP TABLE user_attribute (
                    id SERIAL PRIMARY KEY, user_id TEXT, name TEXT, value TEXT
                ) ON COMMIT DROP;
                INSERT INTO realm VALUES ('r1', 'event-a'), ('r2', 'event-b');
                INSERT INTO user_entity VALUES
                    ('v1', 'r1', 'voter-1', 'José', 'Ñúñez', true, NULL),
                    ('v2', 'r1', 'voter-2', NULL, NULL, false, NULL),
                    ('svc', 'r1', 'service', 'S', 'S', true, 'client'),
                    ('other', 'r2', 'other', 'O', 'O', true, NULL);
                INSERT INTO user_attribute (user_id, name, value) VALUES
                    ('v1', '{AREA_ID_ATTR_NAME}', 'a2'),
                    ('v1', '{AREA_ID_ATTR_NAME}', 'a1'),
                    ('v1', '{MIDDLE_NAME_ATTR_NAME}', '李'),
                    ('v1', '{SUFFIX_ATTR_NAME}', 'Jr.'),
                    ('v1', 'dateOfBirth', '2000-01-01'),
                    ('other', '{AREA_ID_ATTR_NAME}', 'a1');
                "#
            ))
            .await?;
        // More than one lookup batch, with IDs nobody holds.
        let mut ids: Vec<String> = (0..VOTER_LOOKUP_BATCH + 5)
            .map(|index| format!("missing-{index}"))
            .collect();
        ids.extend(["v1", "v2", "svc", "other"].map(String::from));
        let records = fetch_voter_records(&transaction, "event-a", &ids).await?;

        assert_eq!(
            records.len(),
            2,
            "service accounts and other realms are left out"
        );
        assert_eq!(
            records["v1"],
            VoterRecord {
                voter_id: "v1".to_string(),
                username: "voter-1".to_string(),
                first_name: "José".to_string(),
                last_name: "Ñúñez".to_string(),
                middle_name: "李".to_string(),
                suffix: "Jr.".to_string(),
                area_ids: vec!["a1".to_string(), "a2".to_string()],
                enabled: true,
                found: true,
            }
        );
        assert_eq!(
            records["v2"],
            VoterRecord {
                voter_id: "v2".to_string(),
                username: "voter-2".to_string(),
                enabled: false,
                found: true,
                ..Default::default()
            }
        );
        assert!(fetch_voter_records(&transaction, "event-a", &[])
            .await?
            .is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn the_default_template_draws_the_preview_without_a_ballot_time() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.devcontainer/minio/public-assets");
        let template = std::fs::read_to_string(root.join("ov_who_voted_user.hbs")).unwrap();
        let preview: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("ov_who_voted.json")).unwrap())
                .unwrap();
        let data: UserData = serde_json::from_value(preview.clone()).unwrap();
        assert_eq!(data.scope, ListScope::AllPosts);
        let rendered = sequent_core::services::reports::render_template_text(
            &template,
            preview.as_object().unwrap().clone(),
        )
        .unwrap();
        assert!(rendered.contains("LIST OF VOTERS WHO ACTUALLY VOTED"));
        for area in &data.areas {
            assert!(rendered.contains(&area.area_name));
        }
        // The scope and the rows of each area, drawn from the data.
        assert!(rendered.contains("All Posts"));
        assert!(!rendered.contains("One Post"));
        assert!(!rendered.contains("Continued"));
        let lower = rendered.to_lowercase();
        // It names voters, so it renders without scripts: nothing in it may
        // depend on one (2025 drew its QR code with a script).
        assert!(!lower.contains("<script"));
        assert!(!lower.contains("qrcode"));
        assert!(rendered.contains("Report hash:"));
        assert!(rendered.contains("Generated by: sample.admin"));
        let system = std::fs::read_to_string(root.join("ov_who_voted_system.hbs")).unwrap();
        assert!(!system.to_lowercase().contains("<script"));
        assert!(!lower.contains("date voted"));
        assert!(!lower.contains("voted_at"));
    }
}
