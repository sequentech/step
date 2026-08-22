// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Writing the files an election event import is made of.
//!
//! The zip the Admin Portal accepts holds one JSON document and up to three
//! CSVs, and two of those CSVs are read **positionally** by the importer, with a
//! byte shape that is easy to get subtly wrong. Before this module that shape was
//! implemented three times — in windmill's exporter, in janitor's Python, and in
//! the Election Architect's TypeScript — and the copies had drifted.
//!
//! Pure, like the rest of this module: these functions return strings and byte
//! vectors. Nothing here touches the filesystem, so the same code writes a file
//! from `step-cli` and offers a download from a browser.

use serde::Serialize;
use serde_json::Value;

/// Separator for multi-valued Keycloak user attributes.
///
/// A single pipe, matching
/// `crate::services::keycloak::user::MULTIVALUE_USER_ATTRIBUTE_SEPARATOR`. The
/// import workbooks use `||`, so whatever reads one has to convert; a `||`
/// reaching a CSV is read as one value containing an empty one.
pub const MULTI_VALUE_SEPARATOR: &str = "|";

/// Columns of `export_scheduled_events-<uuid>.csv`, in order.
///
/// `import_scheduled_events.rs` reads this file by index — `record.get(10)` for
/// the payload — so the order is part of the format, not a presentation choice.
pub const SCHEDULED_EVENT_COLUMNS: &[&str] = &[
    "id",
    "tenant_id",
    "election_event_id",
    "created_at",
    "stopped_at",
    "archived_at",
    "labels",
    "annotations",
    "event_processor",
    "cron_config",
    "event_payload",
    "task_id",
];

/// Columns of `export_reports-<uuid>.csv`, in order.
///
/// Likewise positional: `process_reports_file` reads `election_id` at index 1 and
/// `permission_label` at index 7.
pub const REPORT_COLUMNS: &[&str] = &[
    "id",
    "election_id",
    "report_type",
    "template_alias",
    "cron_config",
    "encryption_policy",
    "password",
    "permission_label",
];

/// One field of a JSON-in-CSV file.
///
/// The distinction between the two variants is the whole reason this type exists:
/// a column with no value at all is written bare, while a column holding the JSON
/// value `null` is written quoted. Collapsing them would make an empty column
/// indistinguishable from one containing null.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonField {
    /// A SQL NULL: the column has no value. Written as a bare, unquoted `null`.
    Null,
    /// A JSON value, written through JSON encoding and then CSV-quoted.
    Value(Value),
}

impl JsonField {
    pub fn string(value: impl Into<String>) -> Self {
        JsonField::Value(Value::String(value.into()))
    }

    pub fn json<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        Ok(JsonField::Value(serde_json::to_value(value)?))
    }
}

/// Render a JSON-in-CSV file, the shape the platform's own exporter writes.
///
/// Every field holds a JSON literal which is then CSV-quoted on top, so a string
/// ends up wrapped in three double quotes — one from JSON, doubled by CSV
/// escaping, inside the CSV's own pair — while a SQL NULL is written bare and
/// unquoted:
///
/// ```text
/// id,labels,cron_config
/// """2c978b94-…""",null,"{""cron"":null,""scheduled_date"":""2026-10-24T16:15:00.000Z""}"
/// ```
///
/// It looks wrong and is not. The importer parses each field with
/// `deserialize_str` after CSV decoding, so the JSON layer is load-bearing.
///
/// Line endings are `\n`: `\r\n` is what the `csv` crate defaults to and the Rust
/// reader accepts either, but a file with Windows endings diffs badly in review
/// for no benefit.
pub fn json_csv(columns: &[&str], rows: &[Vec<JsonField>]) -> String {
    let mut out = String::new();
    out.push_str(&columns.join(","));
    out.push('\n');

    for row in rows {
        let fields: Vec<String> = row.iter().map(json_csv_field).collect();
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

fn json_csv_field(field: &JsonField) -> String {
    match field {
        JsonField::Null => "null".to_string(),
        JsonField::Value(value) => {
            let encoded = serde_json::to_string(value)
                .unwrap_or_else(|_| "null".to_string());
            format!("\"{}\"", encoded.replace('"', "\"\""))
        }
    }
}

/// Render an ordinary CSV: comma-separated, minimally quoted, `\n` endings.
///
/// Used for `export_voters` and `export_reports`, which hold plain values rather
/// than JSON literals.
pub fn plain_csv(columns: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str(&join_csv(columns.iter().map(|column| column.to_string())));
    out.push('\n');
    for row in rows {
        out.push_str(&join_csv(row.iter().cloned()));
        out.push('\n');
    }
    out
}

fn join_csv(values: impl Iterator<Item = String>) -> String {
    let raw: Vec<String> = values.collect();

    // A row consisting of one empty field is the only case where emptiness needs
    // quoting: unquoted it is a blank line, which a reader cannot tell from no row
    // at all. An empty field beside others is unambiguous and stays bare — which
    // is also what Python's csv writer does, and these files have to stay
    // byte-identical to what janitor already produces.
    if raw.len() == 1 && raw[0].is_empty() {
        return "\"\"".to_string();
    }

    raw.into_iter()
        .map(escape_csv)
        .collect::<Vec<_>>()
        .join(",")
}

/// Quote a CSV field only when its content requires it.
fn escape_csv(value: String) -> String {
    let needs_quoting = value.contains(',')
        || value.contains('"')
        || value.contains('\n')
        || value.contains('\r');

    if needs_quoting {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

/// The member name the importer looks for, for each part of a bundle.
///
/// `import_election_event.rs` dispatches on these prefixes, so a file named
/// anything else is silently ignored rather than rejected.
pub mod member {
    pub const ELECTION_EVENT: &str = "export_election_event";
    pub const VOTERS: &str = "export_voters";
    pub const SCHEDULED_EVENTS: &str = "export_scheduled_events";
    pub const REPORTS: &str = "export_reports";

    /// `export_voters-<event id>.csv` and friends.
    pub fn file_name(prefix: &str, event_id: &str, extension: &str) -> String {
        format!("{prefix}-{event_id}.{extension}")
    }
}

/// Join multi-valued attribute values the way the importer splits them.
pub fn join_multi_value<I, S>(values: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    values
        .into_iter()
        .map(|value| value.as_ref().to_string())
        .collect::<Vec<_>>()
        .join(MULTI_VALUE_SEPARATOR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::scheduled_event::{
        generate_manage_date_task_name, EventProcessors,
    };
    use serde_json::json;

    // -- the JSON-in-CSV byte shape ----------------------------------------

    #[test]
    fn it_matches_a_real_export_byte_for_byte() {
        // Taken from a platform export. This is the assertion that makes the
        // three previous implementations redundant.
        let rows = vec![vec![
            JsonField::string("2c978b94-f167-59de-aee7-dbb6d8a1b913"),
            JsonField::Null,
            JsonField::Value(json!({
                "cron": null,
                "scheduled_date": "2026-10-24T16:15:00.000Z"
            })),
            JsonField::Value(json!({"election_id": null})),
        ]];
        let rendered =
            json_csv(&["id", "labels", "cron_config", "event_payload"], &rows);

        assert_eq!(
            rendered,
            concat!(
                "id,labels,cron_config,event_payload\n",
                "\"\"\"2c978b94-f167-59de-aee7-dbb6d8a1b913\"\"\",",
                "null,",
                "\"{\"\"cron\"\":null,\"\"scheduled_date\"\":\"\"2026-10-24T16:15:00.000Z\"\"}\",",
                "\"{\"\"election_id\"\":null}\"\n"
            )
        );
    }

    #[test]
    fn it_matches_what_janitor_already_writes() {
        // The property that justifies replacing three implementations with one:
        // this row is copied verbatim from the file janitor's Python emitted for
        // the SEIU1000 event. If Rust and Python disagree by a byte, the tools do
        // not actually share a format and the unification is cosmetic.
        const FROM_JANITOR: &str = concat!(
            "\"\"\"543e5b91-b725-5c8d-97f8-7c8188c89a7c\"\"\",",
            "\"\"\"9384db41-1b21-4b93-a6aa-edfc007136d8\"\"\",",
            "\"\"\"e8e06504-e7f2-5f35-acc1-abaf576bb300\"\"\",",
            "\"\"\"2026-01-01T00:00:00.000000Z\"\"\",",
            "null,null,null,",
            "\"{\"\"janitor.event_name\"\":\"\"Voting Period Opens\"\"}\",",
            "\"\"\"START_VOTING_PERIOD\"\"\",",
            "\"{\"\"cron\"\":null,\"\"scheduled_date\"\":\"\"2027-04-20T00:00:00+00:00\"\"}\",",
            "\"{\"\"election_id\"\":\"\"cf433085-801f-56b4-ac9e-24245d4d516a\"\"}\",",
            "\"\"\"tenant_9384db41-1b21-4b93-a6aa-edfc007136d8",
            "_event_e8e06504-e7f2-5f35-acc1-abaf576bb300",
            "_election_cf433085-801f-56b4-ac9e-24245d4d516a",
            "_START_VOTING_PERIOD\"\"\""
        );

        let task_id = generate_manage_date_task_name(
            "9384db41-1b21-4b93-a6aa-edfc007136d8",
            "e8e06504-e7f2-5f35-acc1-abaf576bb300",
            Some("cf433085-801f-56b4-ac9e-24245d4d516a"),
            &EventProcessors::START_VOTING_PERIOD,
        );

        let row = vec![
            JsonField::string("543e5b91-b725-5c8d-97f8-7c8188c89a7c"),
            JsonField::string("9384db41-1b21-4b93-a6aa-edfc007136d8"),
            JsonField::string("e8e06504-e7f2-5f35-acc1-abaf576bb300"),
            JsonField::string("2026-01-01T00:00:00.000000Z"),
            JsonField::Null,
            JsonField::Null,
            JsonField::Null,
            JsonField::Value(
                json!({"janitor.event_name": "Voting Period Opens"}),
            ),
            JsonField::string("START_VOTING_PERIOD"),
            JsonField::Value(
                json!({"cron": null, "scheduled_date": "2027-04-20T00:00:00+00:00"}),
            ),
            JsonField::Value(
                json!({"election_id": "cf433085-801f-56b4-ac9e-24245d4d516a"}),
            ),
            JsonField::string(task_id),
        ];

        let rendered = json_csv(SCHEDULED_EVENT_COLUMNS, &[row]);
        let data_line = rendered.lines().nth(1).unwrap();
        assert_eq!(data_line, FROM_JANITOR);
    }

    #[test]
    fn a_string_is_wrapped_in_three_quotes() {
        let rows = vec![vec![JsonField::string("START_VOTING_PERIOD")]];
        assert_eq!(
            json_csv(&["a"], &rows),
            "a\n\"\"\"START_VOTING_PERIOD\"\"\"\n"
        );
    }

    #[test]
    fn a_sql_null_is_bare_but_a_json_null_is_quoted() {
        // Collapsing these would make an empty column indistinguishable from one
        // holding the JSON value null.
        assert_eq!(json_csv(&["a"], &[vec![JsonField::Null]]), "a\nnull\n");
        assert_eq!(
            json_csv(&["a"], &[vec![JsonField::Value(Value::Null)]]),
            "a\n\"null\"\n"
        );
    }

    #[test]
    fn a_csv_reader_recovers_the_json() {
        // The importer CSV-decodes and then runs deserialize_str, so a round trip
        // has to give back the original structure.
        let payload = json!({"cron": null, "scheduled_date": "2027-04-20T00:00:00+00:00"});
        let rendered = json_csv(
            &["cron_config"],
            &[vec![JsonField::Value(payload.clone())]],
        );

        let line = rendered.lines().nth(1).unwrap();
        let decoded = decode_one_csv_field(line);
        assert_eq!(serde_json::from_str::<Value>(&decoded).unwrap(), payload);
    }

    #[test]
    fn embedded_quotes_survive_the_round_trip() {
        let payload = json!({"name": "He said \"hi\""});
        let rendered =
            json_csv(&["a"], &[vec![JsonField::Value(payload.clone())]]);
        let decoded = decode_one_csv_field(rendered.lines().nth(1).unwrap());
        assert_eq!(serde_json::from_str::<Value>(&decoded).unwrap(), payload);
    }

    /// Undo one CSV-quoted field, the way a reader would.
    fn decode_one_csv_field(line: &str) -> String {
        let trimmed = line
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(line);
        trimmed.replace("\"\"", "\"")
    }

    // -- plain CSV ---------------------------------------------------------

    #[test]
    fn a_plain_csv_is_minimally_quoted() {
        let rows = vec![vec!["1".to_string(), "2".to_string()]];
        assert_eq!(plain_csv(&["a", "b"], &rows), "a,b\n1,2\n");
    }

    #[test]
    fn a_value_with_a_comma_is_quoted() {
        let rows = vec![vec!["DLC 703 Members, No CBUR".to_string()]];
        assert_eq!(
            plain_csv(&["a"], &rows),
            "a\n\"DLC 703 Members, No CBUR\"\n"
        );
    }

    #[test]
    fn a_value_with_a_quote_is_escaped() {
        let rows = vec![vec!["say \"hi\"".to_string()]];
        assert_eq!(plain_csv(&["a"], &rows), "a\n\"say \"\"hi\"\"\"\n");
    }

    #[test]
    fn an_empty_field_beside_others_stays_empty() {
        // A voter with no email must get an empty field, never the text "None".
        let rows = vec![vec!["x".to_string(), String::new()]];
        assert_eq!(plain_csv(&["a", "b"], &rows), "a,b\nx,\n");
    }

    #[test]
    fn a_lone_empty_field_is_quoted() {
        // Otherwise the row is a blank line, which a reader cannot tell from no
        // row at all.
        let rows = vec![vec![String::new()]];
        assert_eq!(plain_csv(&["a"], &rows), "a\n\"\"\n");
    }

    #[test]
    fn line_endings_are_unix() {
        let rows = vec![vec!["1".to_string()]];
        assert!(!plain_csv(&["a"], &rows).contains('\r'));
    }

    // -- task ids ----------------------------------------------------------

    #[test]
    fn a_task_id_names_its_election_when_it_has_one() {
        // The platform looks the task up by this name. `emit` used to build it here
        // and this test asserted the copy matched; it calls the platform's own
        // function now, and the assertion pins the shape the scheduler expects.
        assert_eq!(
            generate_manage_date_task_name(
                "t",
                "e",
                Some("el"),
                &EventProcessors::START_VOTING_PERIOD
            ),
            "tenant_t_event_e_election_el_START_VOTING_PERIOD"
        );
    }

    #[test]
    fn an_event_wide_task_id_omits_the_election() {
        assert_eq!(
            generate_manage_date_task_name(
                "t",
                "e",
                None,
                &EventProcessors::END_VOTING_PERIOD
            ),
            "tenant_t_event_e_END_VOTING_PERIOD"
        );
    }

    // -- multi-value -------------------------------------------------------

    #[test]
    fn multi_values_join_with_a_single_pipe() {
        // The workbooks use "||"; MULTIVALUE_USER_ATTRIBUTE_SEPARATOR is "|".
        let joined = join_multi_value(["a", "b", "c"]);
        assert_eq!(joined, "a|b|c");
        assert!(!joined.contains("||"));
    }

    #[test]
    fn one_value_joins_to_itself() {
        assert_eq!(join_multi_value(["only"]), "only");
    }

    #[test]
    fn no_values_join_to_nothing() {
        assert_eq!(join_multi_value(Vec::<String>::new()), "");
    }

    // -- member names ------------------------------------------------------

    #[test]
    fn member_names_carry_the_prefixes_the_importer_dispatches_on() {
        assert_eq!(
            member::file_name(member::VOTERS, "abc", "csv"),
            "export_voters-abc.csv"
        );
        assert_eq!(
            member::file_name(member::ELECTION_EVENT, "abc", "json"),
            "export_election_event-abc.json"
        );
    }

    // -- the positional column orders -------------------------------------

    #[test]
    fn scheduled_event_columns_are_in_the_order_the_importer_reads() {
        // import_scheduled_events.rs reads event_payload at index 10.
        assert_eq!(SCHEDULED_EVENT_COLUMNS[10], "event_payload");
        assert_eq!(SCHEDULED_EVENT_COLUMNS[9], "cron_config");
        assert_eq!(SCHEDULED_EVENT_COLUMNS[8], "event_processor");
        assert_eq!(SCHEDULED_EVENT_COLUMNS[11], "task_id");
        assert_eq!(SCHEDULED_EVENT_COLUMNS.len(), 12);
    }

    #[test]
    fn report_columns_are_in_the_order_the_importer_reads() {
        // process_reports_file reads election_id at 1 and permission_label at 7.
        assert_eq!(REPORT_COLUMNS[1], "election_id");
        assert_eq!(REPORT_COLUMNS[7], "permission_label");
        assert_eq!(REPORT_COLUMNS.len(), 8);
    }
}
