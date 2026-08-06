// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The CSV members of a bundle, and the files that travel beside it.
//!
//! Four of a bundle's parts are not in the JSON document at all. Voters and
//! scheduled events are always CSVs, reports are a CSV or nothing, and admin
//! users, role permissions and communication templates are tenant- or
//! portal-scoped rather than part of an event import.
//!
//! A child module of [`super`] so it can reach the builder's resolved ids while
//! keeping [`super`] readable; the two are one unit of code split across two
//! files.

use super::{value_as_text, Builder};
use crate::election_config::emit::{
    scheduled_event_task_id, JsonField, MULTI_VALUE_SEPARATOR, REPORT_COLUMNS,
    SCHEDULED_EVENT_COLUMNS,
};
use crate::election_config::problem::Code;
use crate::election_config::sheet::{
    Origin, Row, SHEET_ADMIN_USERS, SHEET_PERMISSIONS, SHEET_REPORTS,
    SHEET_SCHEDULED_EVENTS, SHEET_TEMPLATES, SHEET_VOTERS,
};
use serde_json::{json, Value};

/// Voter columns the builder derives or reorders.
///
/// Everything else on the sheet is passed through as a Keycloak user attribute,
/// which is how a client adds a reporting breakout column without a code change.
pub const VOTER_LEADING_COLUMNS: &[&str] = &[
    "id",
    "email",
    "email_verified",
    "enabled",
    "first_name",
    "last_name",
    "username",
    "area_name",
    "authorized-election-ids",
];

/// `EventProcessors` in `crate::types::scheduled_event`.
pub const EVENT_PROCESSORS: &[&str] = &[
    "ALLOW_INIT_REPORT",
    "CREATE_REPORT",
    "SEND_TEMPLATE",
    "START_VOTING_PERIOD",
    "END_VOTING_PERIOD",
    "ALLOW_VOTING_PERIOD_END",
    "START_ENROLLMENT_PERIOD",
    "END_ENROLLMENT_PERIOD",
    "START_LOCKDOWN_PERIOD",
    "END_LOCKDOWN_PERIOD",
    "ALLOW_TALLY",
];

/// A CSV to be written: header plus already-stringified rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlainTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl PlainTable {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// The index of a column, for a caller that needs to read one back.
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|column| column == name)
    }
}

/// A CSV whose fields hold JSON literals — the scheduled-events shape.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsonTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<JsonField>>,
}

impl JsonTable {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

/// A voter-communication or report template from the Templates sheet.
///
/// Emitted as a file beside the bundle rather than imported: the event zip has no
/// member for communication templates, so these are handed to whoever loads them
/// through the Admin Portal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommunicationTemplate {
    pub name: String,
    pub alias: String,
    pub document: String,
    pub communication_method: Option<String>,
    pub template_type: Option<String>,
    pub selected_methods: Option<Value>,
}

impl CommunicationTemplate {
    /// A filesystem-safe name for the template's own file.
    pub fn file_name(&self) -> String {
        let source = if self.alias.trim().is_empty() {
            &self.name
        } else {
            &self.alias
        };
        let mut safe = String::with_capacity(source.len());
        for character in source.trim().chars() {
            if character.is_alphanumeric()
                || character == '-'
                || character == '_'
            {
                safe.extend(character.to_lowercase());
            } else {
                safe.push('-');
            }
        }
        let trimmed = safe.trim_matches('-');
        if trimmed.is_empty() {
            "template.hbs".to_string()
        } else {
            format!("{trimmed}.hbs")
        }
    }
}

impl Builder<'_> {
    // -- voters -----------------------------------------------------------

    pub(super) fn build_voters(&mut self) -> PlainTable {
        let Some(sheet) = self.workbook.sheet(SHEET_VOTERS).cloned() else {
            return PlainTable::default();
        };

        // Anything the builder does not derive is carried through as a Keycloak
        // user attribute. `area.external_id` is a reference, not an attribute.
        let passthrough: Vec<String> = sheet
            .headers
            .iter()
            .filter(|header| {
                !header.is_empty()
                    && !VOTER_LEADING_COLUMNS.contains(&header.as_str())
                    && *header != "area.external_id"
            })
            .cloned()
            .collect();

        let mut columns: Vec<String> = VOTER_LEADING_COLUMNS
            .iter()
            .map(|column| (*column).to_string())
            .collect();
        columns.extend(passthrough.iter().cloned());
        self.check_csv_headers(&columns, "Voters");

        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut seen: Vec<(String, usize)> = Vec::new();

        for row in &sheet.rows {
            let Some(username) = self.require_unique(
                row,
                "username",
                "a voter needs a username",
                &mut seen,
            ) else {
                continue;
            };

            let Some(area_name) = self.voter_area_name(row) else {
                continue;
            };

            let email = row.get("email").map(value_as_text).unwrap_or_default();

            // An unverified address blocks delivery of the one-time code, and a
            // census address is one the client asserts is correct. So the default
            // follows whether there is an address at all.
            let email_verified = match row.get("email_verified") {
                Some(value) => csv_bool(value),
                None => bool_text(!email.is_empty()),
            };
            let enabled = match row.get("enabled") {
                Some(value) => csv_bool(value),
                None => bool_text(true),
            };

            let mut values: Vec<(&str, String)> = vec![
                ("id", self.ids.uid("voter", &[&username])),
                ("email", email),
                ("email_verified", email_verified),
                ("enabled", enabled),
                (
                    "first_name",
                    row.get("first_name")
                        .map(value_as_text)
                        .unwrap_or_default(),
                ),
                (
                    "last_name",
                    row.get("last_name").map(value_as_text).unwrap_or_default(),
                ),
                ("username", username),
                ("area_name", area_name),
                ("authorized-election-ids", self.voter_elections(row)),
            ];

            let carried: Vec<(&str, String)> = passthrough
                .iter()
                .map(|header| (header.as_str(), csv_scalar(row.get(header))))
                .collect();
            values.extend(carried);

            rows.push(
                columns
                    .iter()
                    .map(|column| {
                        values
                            .iter()
                            .find(|(name, _)| name == column)
                            .map(|(_, value)| value.clone())
                            .unwrap_or_default()
                    })
                    .collect(),
            );
        }

        let table = self.drop_empty_voter_columns(PlainTable { columns, rows });
        self.check_voter_reachability(&table);
        table
    }

    /// The area's *name*, which is what the voters CSV identifies an area by.
    fn voter_area_name(&mut self, row: &Row) -> Option<String> {
        let Some(external_id) = row.get("area.external_id").map(value_as_text)
        else {
            self.problem(
                row.origin(Some("area.external_id")),
                Code::MissingField,
                "a voter needs an area",
            );
            return None;
        };
        let key = external_id.trim().to_string();

        if let Some((_, name)) =
            self.area_names.iter().find(|(id, _)| id == &key)
        {
            return Some(name.clone());
        }

        // A known area with no name has already been reported by the area
        // builder; reporting it again per voter would bury the real message
        // under one line per row.
        if !self.area_ids.iter().any(|(id, _)| id == &key) {
            self.problem(
                row.origin(Some("area.external_id")),
                Code::DanglingReference,
                format!("no area has external_id '{key}'"),
            );
        }
        None
    }

    /// Resolve `authorized-election-ids` from external ids to UUIDs.
    ///
    /// Blank means every election in the event: a voter with no restriction is
    /// eligible for all of them, and writing an empty attribute would deny access
    /// to all of them instead.
    fn voter_elections(&mut self, row: &Row) -> String {
        let Some(raw) = row.get("authorized-election-ids").cloned() else {
            let all: Vec<&str> = self
                .election_ids
                .iter()
                .map(|(_, id)| id.as_str())
                .collect();
            return all.join(MULTI_VALUE_SEPARATOR);
        };

        let requested: Vec<Value> = match raw {
            Value::Array(items) => items,
            single => vec![single],
        };

        let mut resolved: Vec<String> = Vec::new();
        for item in requested {
            let key = value_as_text(&item).trim().to_string();
            if key.is_empty() {
                continue;
            }
            match self.election_ids.iter().find(|(id, _)| id == &key) {
                Some((_, election_id)) => {
                    let election_id = election_id.clone();
                    if !resolved.contains(&election_id) {
                        resolved.push(election_id);
                    }
                }
                None => self.problem(
                    row.origin(Some("authorized-election-ids")),
                    Code::DanglingReference,
                    format!("no election has external_id '{key}'"),
                ),
            }
        }
        resolved.join(MULTI_VALUE_SEPARATOR)
    }

    /// Drop passthrough columns that are empty for every voter.
    ///
    /// An all-blank column carries nothing, and one of them is actively harmful:
    /// `get_copy_from_query` treats the mere presence of a `password` header as
    /// "hash a password for each of these voters", so a blank password column
    /// would give every voter an empty credential.
    fn drop_empty_voter_columns(&mut self, table: PlainTable) -> PlainTable {
        let keep: Vec<usize> = (0..table.columns.len())
            .filter(|index| {
                VOTER_LEADING_COLUMNS.contains(&table.columns[*index].as_str())
                    || table.rows.iter().any(|row| {
                        row.get(*index).is_some_and(|value| !value.is_empty())
                    })
            })
            .collect();

        if keep.len() == table.columns.len() {
            return table;
        }

        let dropped: Vec<&str> = (0..table.columns.len())
            .filter(|index| !keep.contains(index))
            .map(|index| table.columns[index].as_str())
            .collect();
        let message = format!(
            "dropped voter columns that are blank for every voter: {}",
            dropped.join(", ")
        );
        self.warn("voters", message);

        PlainTable {
            columns: keep
                .iter()
                .map(|index| table.columns[*index].clone())
                .collect(),
            rows: table
                .rows
                .iter()
                .map(|row| {
                    keep.iter().map(|index| row[*index].clone()).collect()
                })
                .collect(),
        }
    }

    /// Warn when voters have no channel to receive a one-time code on.
    ///
    /// Not an error: credentials are sometimes distributed on paper. And under an
    /// identity provider that authenticates the voter itself there is nothing to
    /// send, which is why the presets level gates this — with no preset named,
    /// checking is the safer default.
    fn check_voter_reachability(&mut self, table: &PlainTable) {
        if table.rows.is_empty() {
            return;
        }

        // Under an identity provider that authenticates the voter itself there is
        // no code to send, so the whole question is noise. With no preset named,
        // checking is the safer default.
        if self.auth_preset.is_some_and(|preset| !preset.uses_otp) {
            return;
        }

        // There is always at least one contact column: `email` is derived, so it
        // is present whether or not the source has it. The Python this was ported
        // from also carried a "no contact column at all" warning, which for the
        // same reason could never fire — a census with no email column at all
        // reaches the per-voter count below with every voter unreachable, which
        // is the more useful message anyway.
        let contact: Vec<usize> = table
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| {
                *column == "email"
                    || column.contains("mobile")
                    || column.contains("phone")
            })
            .map(|(index, _)| index)
            .collect();

        let unreachable = table
            .rows
            .iter()
            .filter(|row| {
                !contact.iter().any(|index| {
                    row.get(*index).is_some_and(|value| !value.is_empty())
                })
            })
            .count();

        if unreachable > 0 {
            let message = format!(
                "{unreachable} of {} voters have neither an email address nor \
                 a mobile number and cannot be sent a one-time code",
                table.rows.len()
            );
            self.warn("voters", message);
        }
    }

    // -- scheduled events -------------------------------------------------

    pub(super) fn build_scheduled_events(&mut self) -> JsonTable {
        let rows_in: Vec<Row> =
            self.workbook.rows(SHEET_SCHEDULED_EVENTS).to_vec();
        let mut rows: Vec<Vec<JsonField>> = Vec::new();

        // Kept alongside so the window check does not have to read the payload
        // back out of the JSON it just wrote.
        let mut scheduled: Vec<(String, Option<String>)> = Vec::new();

        for row in &rows_in {
            let Some(processor) = self.event_processor(row) else {
                continue;
            };

            let Some(when) = row.get("scheduled_datetime").map(value_as_text)
            else {
                self.problem(
                    row.origin(Some("scheduled_datetime")),
                    Code::MissingField,
                    format!("{processor} needs a scheduled_datetime"),
                );
                continue;
            };

            let mut election_id = None;
            if row.get("election.external_id").is_some() {
                let elections = self.election_ids.clone();
                let Some(resolved) = self.resolve(
                    row,
                    "election.external_id",
                    &elections,
                    "election",
                ) else {
                    continue;
                };
                election_id = Some(resolved);
            }

            // Not a schema field, but it is how the author labels the row, and
            // keeping it makes the emitted CSV readable next to the source.
            let annotations = match row.get("event_name").map(value_as_text) {
                Some(name) if !name.is_empty() => {
                    JsonField::Value(json!({"janitor.event_name": name}))
                }
                _ => JsonField::Null,
            };

            let task_id = scheduled_event_task_id(
                &self.tenant_id,
                &self.event_id,
                election_id.as_deref(),
                &processor,
            );

            rows.push(vec![
                JsonField::string(self.ids.uid(
                    "scheduled_event",
                    &[&processor, election_id.as_deref().unwrap_or("")],
                )),
                JsonField::string(self.tenant_id.clone()),
                JsonField::string(self.event_id.clone()),
                JsonField::string(self.created_at.clone()),
                JsonField::Null, // stopped_at
                JsonField::Null, // archived_at
                JsonField::Null, // labels
                annotations,
                JsonField::string(processor.clone()),
                JsonField::Value(json!({
                    "cron": Value::Null,
                    "scheduled_date": when,
                })),
                JsonField::Value(json!({"election_id": election_id})),
                JsonField::string(task_id),
            ]);
            scheduled.push((processor, election_id));
        }

        if rows.is_empty() {
            self.warn(
                "scheduled_events",
                "no scheduled events: the voting period will have to be opened \
                 and closed by hand in the Admin Portal",
            );
        } else {
            self.check_voting_windows(&scheduled);
        }

        JsonTable {
            columns: SCHEDULED_EVENT_COLUMNS
                .iter()
                .map(|column| (*column).to_string())
                .collect(),
            rows,
        }
    }

    /// The row's `event_type` as a processor name the platform knows.
    fn event_processor(&mut self, row: &Row) -> Option<String> {
        let Some(raw) = row.get("event_type").map(value_as_text) else {
            self.problem(
                row.origin(Some("event_type")),
                Code::MissingField,
                "a scheduled event needs an event_type",
            );
            return None;
        };

        // Authors write "start voting period" and "start-voting-period" as often
        // as the constant.
        let processor = raw.trim().to_uppercase().replace(['-', ' '], "_");

        if !EVENT_PROCESSORS.contains(&processor.as_str()) {
            let mut expected: Vec<&str> = EVENT_PROCESSORS.to_vec();
            expected.sort_unstable();
            let message = format!(
                "'{}' is not an event processor; expected one of {}",
                raw.trim(),
                expected.join(", ")
            );
            self.problem(
                row.origin(Some("event_type")),
                Code::InvalidValue,
                message,
            );
            return None;
        }
        Some(processor)
    }

    /// Warn about an election whose voting period never opens or never closes.
    ///
    /// A scheduled event with no election applies to the whole event, so an
    /// election is covered either by its own row or by an event-wide one. An
    /// uncovered election imports fine and then quietly never opens.
    fn check_voting_windows(&mut self, scheduled: &[(String, Option<String>)]) {
        let elections = self.election_ids.clone();
        let mut warnings: Vec<String> = Vec::new();

        for (external_id, election_id) in &elections {
            let covered: Vec<&str> = scheduled
                .iter()
                .filter(|(_, scoped)| {
                    scoped.is_none() || scoped.as_deref() == Some(election_id)
                })
                .map(|(processor, _)| processor.as_str())
                .collect();

            let missing: Vec<&str> =
                ["START_VOTING_PERIOD", "END_VOTING_PERIOD"]
                    .into_iter()
                    .filter(|processor| !covered.contains(processor))
                    .collect();

            if missing.is_empty() {
                continue;
            }

            let effects: Vec<&str> = missing
                .iter()
                .map(|processor| {
                    if *processor == "START_VOTING_PERIOD" {
                        "open"
                    } else {
                        "close"
                    }
                })
                .collect();
            warnings.push(format!(
                "election '{external_id}' has no {} scheduled event; its \
                 voting period will not {} on its own",
                missing.join(" and no "),
                effects.join(" or ")
            ));
        }

        for warning in warnings {
            self.warn("scheduled_events", warning);
        }
    }

    // -- reports ----------------------------------------------------------

    pub(super) fn build_reports(&mut self) -> Option<PlainTable> {
        let rows_in: Vec<Row> = self.workbook.rows(SHEET_REPORTS).to_vec();
        if rows_in.is_empty() {
            return None;
        }

        let aliases: Vec<String> = self
            .workbook
            .rows(SHEET_TEMPLATES)
            .iter()
            .filter_map(|row| row.get("alias"))
            .map(|alias| value_as_text(alias).trim().to_string())
            .collect();

        let mut rows: Vec<Vec<String>> = Vec::new();
        for (index, row) in rows_in.iter().enumerate() {
            let Some(report_type) = row.get("report_type").map(value_as_text)
            else {
                self.problem(
                    row.origin(Some("report_type")),
                    Code::MissingField,
                    "a report needs a report_type",
                );
                continue;
            };

            let mut election_id = String::new();
            if row.get("election.external_id").is_some() {
                let elections = self.election_ids.clone();
                let Some(resolved) = self.resolve(
                    row,
                    "election.external_id",
                    &elections,
                    "election",
                ) else {
                    continue;
                };
                election_id = resolved;
            }

            let alias = row.get("template.alias").map(value_as_text);
            if let Some(alias) = &alias {
                if !aliases.contains(&alias.trim().to_string()) {
                    let message =
                        format!("no Templates row has alias '{alias}'");
                    self.problem(
                        row.origin(Some("template.alias")),
                        Code::DanglingReference,
                        message,
                    );
                    continue;
                }
            }

            let context = json!({
                "id": self.ids.uid(
                    "report",
                    &[&report_type, &election_id, &(index + 1).to_string()],
                ),
                "tenant_id": self.tenant_id,
                "election_event_id": self.event_id,
                "created_at": self.created_at,
                "report_type": report_type,
            });
            let rendered = self.render("report", Some(row), context);

            if row.get("password").is_some() {
                let path = row.origin(Some("password")).to_string();
                self.warn(
                    path,
                    "the report password is written to the reports CSV in clear \
                     text; treat the output as a secret",
                );
            }

            // These come from the sheet's own columns rather than from
            // dotted-path overrides: they are control columns, so the row was
            // excluded from the merge and the rendered value is only the
            // template's default. Reading the template instead is how
            // `configured_password` silently became `unencrypted` once.
            let cron_config = row
                .get("cron_config")
                .cloned()
                .or_else(|| rendered.get("cron_config").cloned());
            let policy = row
                .get("encryption_policy")
                .map(value_as_text)
                .filter(|policy| !policy.is_empty())
                .or_else(|| {
                    rendered
                        .get("encryption_policy")
                        .map(value_as_text)
                        .filter(|policy| !policy.is_empty())
                })
                .unwrap_or_else(|| "unencrypted".to_string());

            rows.push(vec![
                rendered.get("id").map(value_as_text).unwrap_or_default(),
                election_id,
                report_type,
                alias.unwrap_or_default(),
                csv_json(cron_config.as_ref()),
                policy,
                csv_scalar(row.get("password")),
                // Option<Vec<String>>, split on "|" by process_reports_file.
                join_multi(row.get("permission_label")),
            ]);
        }

        if rows.is_empty() {
            return None;
        }
        Some(PlainTable {
            columns: REPORT_COLUMNS
                .iter()
                .map(|column| (*column).to_string())
                .collect(),
            rows,
        })
    }

    // -- admin users, permissions, templates -------------------------------

    pub(super) fn build_admin_users(&mut self) -> Option<PlainTable> {
        let sheet = self.workbook.sheet(SHEET_ADMIN_USERS).cloned()?;
        if sheet.rows.is_empty() {
            return None;
        }

        let columns: Vec<String> = sheet
            .headers
            .iter()
            .filter(|header| !header.is_empty())
            .cloned()
            .collect();
        self.check_csv_headers(&columns, "Admin Users");

        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut warned_password = false;

        for row in &sheet.rows {
            if row.get("username").is_none() {
                self.problem(
                    row.origin(Some("username")),
                    Code::MissingField,
                    "an admin user needs a username",
                );
                continue;
            }
            if row.get("password").is_some() && !warned_password {
                warned_password = true;
                self.warn(
                    "admin_users",
                    "Admin Users carries clear-text passwords; the emitted \
                     admin_users CSV is a secret, not a deliverable",
                );
            }

            rows.push(
                columns
                    .iter()
                    .map(|header| match row.get(header) {
                        // permission_labels arrives "||"-separated and leaves
                        // "|"-separated, like every multi-valued attribute.
                        Some(Value::Array(_)) => join_multi(row.get(header)),
                        value => csv_scalar(value),
                    })
                    .collect(),
            );
        }

        Some(PlainTable { columns, rows })
    }

    /// Transpose the permission matrix into the platform's own shape.
    ///
    /// `export_tenant_config.rs` writes `role,permissions` with permissions
    /// joined by `|`; the source holds the transpose, one row per permission and
    /// one column per role, marked with any non-empty cell. A matrix is what a
    /// human can check at a glance, which is why the conversion lives here.
    pub(super) fn build_role_permissions(&mut self) -> Option<PlainTable> {
        let sheet = self.workbook.sheet(SHEET_PERMISSIONS).cloned()?;
        if sheet.rows.is_empty() {
            return None;
        }

        let roles: Vec<String> = sheet
            .headers
            .iter()
            .skip(1)
            .filter(|header| !header.is_empty())
            .cloned()
            .collect();

        if roles.is_empty() {
            self.problem(
                Origin::sheet(sheet.name.clone()),
                Code::MissingField,
                "the Permissions matrix has no role columns; expected the first \
                 column to hold permissions and one further column per role",
            );
            return None;
        }

        let permission_column = sheet.headers[0].clone();
        let mut granted: Vec<(String, Vec<String>)> = roles
            .iter()
            .map(|role| (role.clone(), Vec::new()))
            .collect();

        for row in &sheet.rows {
            let Some(permission) =
                row.get(&permission_column).map(value_as_text)
            else {
                continue;
            };
            for (role, permissions) in granted.iter_mut() {
                if row.get(role).is_some() {
                    permissions.push(permission.clone());
                }
            }
        }

        Some(PlainTable {
            columns: vec!["role".to_string(), "permissions".to_string()],
            rows: granted
                .into_iter()
                .map(|(role, permissions)| {
                    vec![role, permissions.join(MULTI_VALUE_SEPARATOR)]
                })
                .collect(),
        })
    }

    pub(super) fn build_templates(&mut self) -> Vec<CommunicationTemplate> {
        let rows: Vec<Row> = self.workbook.rows(SHEET_TEMPLATES).to_vec();
        let mut templates = Vec::new();
        let mut seen: Vec<(String, usize)> = Vec::new();

        for row in &rows {
            let alias = row
                .get("alias")
                .or_else(|| row.get("name"))
                .map(value_as_text)
                .filter(|alias| !alias.is_empty());

            let Some(alias) = alias else {
                self.problem(
                    row.origin(Some("alias")),
                    Code::MissingField,
                    "a template needs a name or an alias",
                );
                continue;
            };

            if let Some((_, earlier)) = seen.iter().find(|(id, _)| id == &alias)
            {
                let message =
                    format!("alias '{alias}' is already used by row {earlier}");
                self.problem(
                    row.origin(Some("alias")),
                    Code::DuplicateId,
                    message,
                );
                continue;
            }
            seen.push((alias.clone(), row.number));

            let Some(document) =
                row.get("template.document").map(value_as_text)
            else {
                self.problem(
                    row.origin(Some("template.document")),
                    Code::MissingField,
                    "a template needs a document",
                );
                continue;
            };

            templates.push(CommunicationTemplate {
                name: row
                    .get("name")
                    .map(value_as_text)
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| alias.clone()),
                alias,
                document: unescape_document(&document),
                communication_method: row
                    .get("communication_method")
                    .map(value_as_text),
                template_type: row.get("type").map(value_as_text),
                selected_methods: row.get("template.selected_methods").cloned(),
            });
        }
        templates
    }

    // -- shared -----------------------------------------------------------

    /// A required, unique value from one column.
    fn require_unique(
        &mut self,
        row: &Row,
        column: &str,
        missing: &str,
        seen: &mut Vec<(String, usize)>,
    ) -> Option<String> {
        let value = match row.get(column).map(value_as_text) {
            Some(value) if !value.is_empty() => value,
            _ => {
                self.problem(
                    row.origin(Some(column)),
                    Code::MissingField,
                    missing,
                );
                return None;
            }
        };

        if let Some((_, earlier)) = seen.iter().find(|(id, _)| id == &value) {
            let message =
                format!("{column} '{value}' is already used by row {earlier}");
            self.problem(row.origin(Some(column)), Code::DuplicateId, message);
            return None;
        }
        seen.push((value.clone(), row.number));
        Some(value)
    }

    /// Both CSV importers reject headers outside their `HEADER_RE`.
    ///
    /// Catching it here turns an opaque mid-import failure into a message naming
    /// the column.
    fn check_csv_headers(&mut self, columns: &[String], sheet: &str) {
        let offenders: Vec<String> = columns
            .iter()
            .filter(|column| !is_importable_header(column))
            .cloned()
            .collect();

        for column in offenders {
            self.problem(
                Origin::column(sheet, column),
                Code::InvalidValue,
                "the importer rejects this column name; only letters, digits, \
                 '.', '_' and '-' are allowed",
            );
        }
    }
}

/// `^[a-zA-Z0-9._-]+$`, the pattern both CSV importers enforce.
fn is_importable_header(column: &str) -> bool {
    !column.is_empty()
        && column.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || character == '.'
                || character == '_'
                || character == '-'
        })
}

fn bool_text(value: bool) -> String {
    if value {
        "true".to_string()
    } else {
        "false".to_string()
    }
}

/// A cell meant as a flag.
///
/// Spreadsheets carry every spelling of yes: an author ticking a column with an
/// `x` means the same as one typing `TRUE`.
fn csv_bool(value: &Value) -> String {
    match value {
        Value::Bool(value) => bool_text(*value),
        Value::String(text) => bool_text(matches!(
            text.trim().to_lowercase().as_str(),
            "true" | "1" | "yes" | "x"
        )),
        Value::Null => bool_text(false),
        Value::Number(number) => bool_text(number.as_f64() != Some(0.0)),
        _ => bool_text(true),
    }
}

/// One cell of a plain CSV. Absent becomes empty, never the word "null".
fn csv_scalar(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(flag)) => bool_text(*flag),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

/// A cell holding JSON: text passes through, anything else is encoded.
fn csv_json(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

/// A multi-valued cell, joined the way the importer splits it.
fn join_multi(value: Option<&Value>) -> String {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(value_as_text)
            .collect::<Vec<_>>()
            .join(MULTI_VALUE_SEPARATOR),
        other => csv_scalar(other),
    }
}

/// Un-escape a document pasted into a spreadsheet cell.
///
/// The Templates sheet holds documents with literal `\n` and `\"`, because that
/// is what survives a copy-paste out of a JSON export. Writing those through
/// verbatim emits a template full of backslash-n instead of newlines.
fn unescape_document(document: &str) -> String {
    if !document.contains("\\n") && !document.contains("\\\"") {
        return document.to_string();
    }

    // One pass, so that an escaped backslash before an n is not then read as a
    // newline: "\\n" is a backslash followed by an n, not a line break.
    let mut out = String::with_capacity(document.len());
    let mut characters = document.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => out.push('\n'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_the_importer_would_reject_is_caught_here() {
        assert!(is_importable_header("permission_labels"));
        assert!(is_importable_header("area.external_id"));
        assert!(is_importable_header("mobile-number"));
        // The ones that would fail mid-import, opaquely.
        assert!(!is_importable_header("home address"));
        assert!(!is_importable_header("años"));
        assert!(!is_importable_header(""));
    }

    #[test]
    fn every_spelling_of_yes_is_a_yes() {
        // An author ticking a column with an x means what someone typing TRUE
        // means.
        for yes in ["true", "TRUE", "Yes", "x", "1"] {
            assert_eq!(csv_bool(&json!(yes)), "true", "{yes}");
        }
        for no in ["false", "no", "", "0"] {
            assert_eq!(csv_bool(&json!(no)), "false", "{no}");
        }
        assert_eq!(csv_bool(&json!(true)), "true");
        assert_eq!(csv_bool(&json!(0)), "false");
    }

    #[test]
    fn an_absent_cell_is_empty_and_never_the_word_null() {
        // "null" in a plain CSV is a voter attribute holding the text null.
        assert_eq!(csv_scalar(None), "");
        assert_eq!(csv_scalar(Some(&Value::Null)), "");
        assert_eq!(csv_scalar(Some(&json!("kept"))), "kept");
        assert_eq!(csv_scalar(Some(&json!(7))), "7");
        assert_eq!(csv_scalar(Some(&json!({"a": 1}))), r#"{"a":1}"#);
    }

    #[test]
    fn a_multi_valued_cell_leaves_single_pipe_separated() {
        // "||" going in, "|" coming out: the source's separator is not the
        // importer's.
        assert_eq!(join_multi(Some(&json!(["a", "b"]))), "a|b");
        assert_eq!(join_multi(Some(&json!(["only"]))), "only");
        assert_eq!(join_multi(Some(&json!([]))), "");
        assert_eq!(join_multi(Some(&json!("plain"))), "plain");
        assert_eq!(join_multi(None), "");
    }

    #[test]
    fn a_pasted_document_gets_its_newlines_back() {
        // What survives a copy-paste out of a JSON export.
        assert_eq!(
            unescape_document(r#"Dear {{name}},\n\nYour code is {{code}}."#),
            "Dear {{name}},\n\nYour code is {{code}}."
        );
        assert_eq!(unescape_document(r#"say \"hi\""#), r#"say "hi""#);
    }

    #[test]
    fn a_document_with_no_escapes_is_left_exactly_alone() {
        let literal = "already\nreal\nnewlines and a \\ backslash";
        assert_eq!(unescape_document(literal), literal);
    }

    #[test]
    fn an_escaped_backslash_before_an_n_is_not_a_newline() {
        // The bug a two-pass replace would have: "\\n" is a backslash and an n.
        assert_eq!(unescape_document(r"a\\nb"), r"a\nb");
    }

    #[test]
    fn a_template_file_name_is_safe_to_write() {
        let template = |alias: &str, name: &str| CommunicationTemplate {
            name: name.to_string(),
            alias: alias.to_string(),
            document: String::new(),
            communication_method: None,
            template_type: None,
            selected_methods: None,
        };
        assert_eq!(
            template("Voter Credentials", "").file_name(),
            "voter-credentials.hbs"
        );
        assert_eq!(template("otp_email", "").file_name(), "otp_email.hbs");
        // Falls back to the name, then to something writable.
        assert_eq!(
            template("", "Fallback Name").file_name(),
            "fallback-name.hbs"
        );
        assert_eq!(template("///", "").file_name(), "template.hbs");
    }

    #[test]
    fn the_scheduled_events_columns_are_the_ones_the_importer_reads() {
        // Positional: a reordering here is a payload read as a task id.
        assert_eq!(SCHEDULED_EVENT_COLUMNS[10], "event_payload");
        assert_eq!(REPORT_COLUMNS[1], "election_id");
        assert_eq!(REPORT_COLUMNS[7], "permission_label");
    }
}
