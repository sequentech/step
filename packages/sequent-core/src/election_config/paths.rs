// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Dotted column headers, cell coercion, and deep merge.
//!
//! An authoring spreadsheet's column headers are paths into the target JSON:
//! `presentation.i18n.en.name` means
//! `{"presentation": {"i18n": {"en": {"name": …}}}}`. Keeping that mapping
//! generic is what makes a workbook client-agnostic — a new column lands in the
//! output without a code change.
//!
//! This layer knows nothing about spreadsheets. It takes [`Cell`]s, which a
//! reader produces from `.xlsx`, a CSV, or a browser's form state, and turns them
//! into `serde_json` values. Pure and dependency-light on purpose: it is the part
//! most worth testing and the part that runs in a browser.

use crate::election_config::problem::{Code, Problem};
use chrono::{NaiveDateTime, TimeZone, Utc};
use serde_json::{Map, Number, Value};

/// Separator for multi-valued cells.
///
/// Two pipes, as the authoring workbooks already write them —
/// `statewide-officers || dlc-officers-dburs || cburs`. Note this is *not*
/// [`super::emit::MULTI_VALUE_SEPARATOR`], which is the single pipe the importer
/// splits Keycloak attributes on; whatever reads a workbook has to convert.
pub const MULTI_VALUE_SEPARATOR: &str = "||";

/// A cell holding exactly this, case-insensitively, means JSON `null`.
///
/// A blank cell means something different: leave the template's default alone.
pub const NULL_LITERAL: &str = "null";

/// One cell as the reader handed it over, before it becomes JSON.
///
/// A neutral vocabulary so this module has no opinion about where a row came
/// from. `Blank` and a cell holding the text `null` are deliberately different
/// things and stay different all the way through [`coerce_cell`].
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Empty, or nothing but whitespace: the author said nothing about this
    /// field.
    Blank,
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    /// A date or time. Spreadsheet cells carry no timezone, so the reader hands
    /// over a naive instant and [`coerce_scalar_cell`] reads it as UTC.
    DateTime(NaiveDateTime),
}

impl Cell {
    /// Build a cell from text, treating whitespace-only as blank.
    ///
    /// A cell someone typed a space into is as empty in intent as one they never
    /// touched, and readers should not each have to remember that.
    pub fn text(value: impl Into<String>) -> Self {
        let value: String = value.into();
        if value.trim().is_empty() {
            Cell::Blank
        } else {
            Cell::Text(value)
        }
    }

    pub fn is_blank(&self) -> bool {
        matches!(self, Cell::Blank)
    }
}

/// Split a dotted header into its parts.
///
/// `"presentation.i18n.en.name"` becomes
/// `["presentation", "i18n", "en", "name"]`. Keys containing a literal dot are
/// not supported; no entity schema has one.
pub fn split_path(header: &str) -> Vec<String> {
    header
        .trim()
        .split('.')
        .map(|part| part.trim().to_string())
        .collect()
}

/// Turn a piece of text into the JSON value the platform expects.
///
/// Spreadsheets are typed loosely and the platform is not: `max_votes` typed as
/// `3` must not arrive as `3.0`, which fails deserialization into `i64`, and text
/// that happens to be JSON is meant as JSON — it is how a whole
/// `voting_channels` array fits in one cell.
pub fn coerce_scalar(text: &str) -> Value {
    let text = text.trim();

    if text.eq_ignore_ascii_case(NULL_LITERAL) {
        return Value::Null;
    }

    if text.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if text.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }

    // A trailing ".0" on an id or a code is a spreadsheet artifact, never intent.
    if let Some(integral) = parse_integral_float(text) {
        return Value::from(integral);
    }

    // Only bracketed text is tried as JSON. Without that guard a plain "1" would
    // be reinterpreted as a number by a different route, and a candidate whose
    // name is "NaN" would turn into something that is not even valid JSON.
    let bracketed = (text.starts_with('{') || text.starts_with('['))
        && (text.ends_with('}') || text.ends_with(']'));
    if bracketed {
        if let Ok(parsed) = serde_json::from_str::<Value>(text) {
            return parsed;
        }
        // Not JSON after all — a description that opens with a bracket is still
        // a description.
    }

    Value::String(text.to_string())
}

/// `"-12.000"` -> `-12`; anything else -> `None`.
fn parse_integral_float(text: &str) -> Option<i64> {
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1i64, text),
    };
    let (whole, fraction) = digits.split_once('.')?;
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.is_empty()
        || !fraction.bytes().all(|byte| byte == b'0')
    {
        return None;
    }
    whole.parse::<i64>().ok().map(|value| sign * value)
}

/// Coerce one cell, ignoring the multi-value question.
///
/// A blank cell has no JSON value at all, which is why this returns an `Option`:
/// `None` means "the author said nothing", and `Some(Value::Null)` means "the
/// author wrote null". Collapsing the two would make it impossible to clear a
/// template default.
pub fn coerce_scalar_cell(cell: &Cell) -> Option<Value> {
    match cell {
        Cell::Blank => None,
        Cell::Text(text) => Some(coerce_scalar(text)),
        Cell::Bool(value) => Some(Value::Bool(*value)),
        Cell::Int(value) => Some(Value::from(*value)),
        Cell::Float(value) => Some(coerce_float(*value)),
        Cell::DateTime(naive) => {
            // Read as UTC. A spreadsheet cell has no timezone, and guessing the
            // author's local one would silently shift a voting window.
            Some(Value::String(Utc.from_utc_datetime(naive).to_rfc3339()))
        }
    }
}

/// `3.0` is `3`. A genuine fraction stays a fraction.
///
/// No schema field wants a fraction, but truncating one silently would be worse
/// than passing it through for validation to object to. A value JSON cannot
/// represent — an infinity, a NaN — becomes null rather than an unparseable file.
fn coerce_float(value: f64) -> Value {
    if value.fract() == 0.0 && value.is_finite() && value.abs() < 9e15 {
        return Value::from(value as i64);
    }
    Number::from_f64(value).map_or(Value::Null, Value::Number)
}

/// Coerce a cell, optionally splitting it into a list.
///
/// Whether a column is multi-valued is decided by the column, not by the
/// content: a cell that happens to hold no separator still becomes a
/// one-element list, or the JSON type emitted would depend on the data.
pub fn coerce_cell(cell: &Cell, multi_value: bool) -> Option<Value> {
    if cell.is_blank() {
        return None;
    }

    if multi_value {
        if let Cell::Text(text) = cell {
            let parts: Vec<Value> = text
                .split(MULTI_VALUE_SEPARATOR)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(coerce_scalar)
                .collect();
            return Some(Value::Array(parts));
        }
        return coerce_scalar_cell(cell).map(|value| Value::Array(vec![value]));
    }

    coerce_scalar_cell(cell)
}

/// Set `value` at `path`, creating the objects along the way.
///
/// Fails when the path runs through something that is not an object, which means
/// two columns disagree about the shape — `presentation` and
/// `presentation.i18n` both being set, for instance. Reported rather than
/// panicked because it is an authoring mistake, and the author is the one who
/// has to see it.
pub fn set_path(
    target: &mut Map<String, Value>,
    path: &[String],
    value: Value,
) -> Result<(), Problem> {
    let dotted = path.join(".");

    let (last, parents) = match path.split_last() {
        Some(split) if !split.0.is_empty() => split,
        // An empty header, or one ending in a dot: there is no field named "".
        _ => {
            return Err(Problem::error(
                Code::ConflictingColumns,
                dotted.clone(),
                format!("'{dotted}' is not a usable column header: it names an empty field"),
            ))
        }
    };

    let mut cursor = target;
    for key in parents {
        if key.is_empty() {
            return Err(Problem::error(
                Code::ConflictingColumns,
                dotted.clone(),
                format!("'{dotted}' is not a usable column header: it names an empty field"),
            ));
        }
        let entry = cursor
            .entry(key.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        if entry.is_null() {
            *entry = Value::Object(Map::new());
        }
        // Named before descending: once `entry` is borrowed mutably it can no
        // longer be inspected for the message.
        let held = type_name(entry);
        cursor = match entry.as_object_mut() {
            Some(object) => object,
            None => {
                return Err(Problem::error(
                    Code::ConflictingColumns,
                    dotted.clone(),
                    format!(
                        "cannot set '{dotted}': '{key}' already holds a \
                         {held}, not an object. Two columns disagree about \
                         the shape."
                    ),
                ))
            }
        };
    }

    cursor.insert(last.clone(), value);
    Ok(())
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "list",
        Value::Object(_) => "object",
    }
}

/// Turn `[("a.b", 1), ("a.c", 2)]` into `{"a": {"b": 1, "c": 2}}`.
///
/// Ordered pairs rather than a map, because a row's fields arrive in column
/// order and two columns writing the same path should resolve left to right,
/// the way a reader of the spreadsheet would expect.
pub fn expand(
    fields: &[(String, Value)],
) -> Result<Map<String, Value>, Problem> {
    let mut nested = Map::new();
    for (header, value) in fields {
        set_path(&mut nested, &split_path(header), value.clone())?;
    }
    Ok(nested)
}

/// Merge `override_value` onto `base`; objects recurse, everything else replaces.
///
/// Lists replace rather than concatenate. A cell listing three voting channels
/// means exactly those three, not those three appended to whatever the template
/// had — there would otherwise be no way to remove one.
pub fn deep_merge(base: Value, override_value: Value) -> Value {
    match (base, override_value) {
        (Value::Object(base), Value::Object(override_object)) => {
            let mut merged = base;
            for (key, value) in override_object {
                let combined = match merged.remove(&key) {
                    Some(existing) => deep_merge(existing, value),
                    None => value,
                };
                merged.insert(key, combined);
            }
            Value::Object(merged)
        }
        (_, override_value) => override_value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Cell {
        Cell::DateTime(
            NaiveDate::from_ymd_opt(year, month, day)
                .unwrap()
                .and_hms_opt(hour, minute, 0)
                .unwrap(),
        )
    }

    #[test]
    fn a_dotted_header_becomes_a_path() {
        assert_eq!(
            split_path("presentation.i18n.en.name"),
            vec!["presentation", "i18n", "en", "name"]
        );
    }

    #[test]
    fn spaces_around_a_header_and_its_parts_are_noise() {
        // Spreadsheets collect trailing spaces nobody can see.
        assert_eq!(split_path("  a . b  "), vec!["a", "b"]);
    }

    #[test]
    fn a_header_with_no_dots_is_a_single_part() {
        assert_eq!(split_path("max_votes"), vec!["max_votes"]);
    }

    #[test]
    fn whitespace_is_as_empty_as_empty() {
        assert!(Cell::text("   ").is_blank());
        assert!(Cell::text("").is_blank());
        assert!(!Cell::text(" x ").is_blank());
    }

    #[test]
    fn an_integral_float_loses_its_pointless_fraction() {
        // Excel hands back 3.0 for a 3 someone typed, and the platform wants an
        // i64.
        assert_eq!(coerce_scalar_cell(&Cell::Float(3.0)), Some(json!(3)));
        assert_eq!(coerce_scalar_cell(&Cell::Float(-7.0)), Some(json!(-7)));
        assert_eq!(coerce_scalar("3.0"), json!(3));
        assert_eq!(coerce_scalar("-12.000"), json!(-12));
    }

    #[test]
    fn a_genuine_fraction_survives() {
        assert_eq!(coerce_scalar_cell(&Cell::Float(1.5)), Some(json!(1.5)));
    }

    #[test]
    fn a_number_json_cannot_hold_becomes_null_rather_than_a_broken_file() {
        assert_eq!(
            coerce_scalar_cell(&Cell::Float(f64::INFINITY)),
            Some(Value::Null)
        );
        assert_eq!(
            coerce_scalar_cell(&Cell::Float(f64::NAN)),
            Some(Value::Null)
        );
    }

    #[test]
    fn a_version_string_is_not_a_number() {
        // "1.0.0" has a dot and digits and is still text.
        assert_eq!(coerce_scalar("1.0.0"), json!("1.0.0"));
        assert_eq!(coerce_scalar("3.5"), json!("3.5"));
        assert_eq!(coerce_scalar(".0"), json!(".0"));
    }

    #[test]
    fn the_null_literal_is_json_null_and_a_blank_cell_is_not() {
        // The difference decides whether a template default can be cleared.
        assert_eq!(coerce_scalar_cell(&Cell::text("null")), Some(Value::Null));
        assert_eq!(coerce_scalar_cell(&Cell::text("NULL")), Some(Value::Null));
        assert_eq!(coerce_scalar_cell(&Cell::Blank), None);
    }

    #[test]
    fn booleans_are_recognised_whatever_their_case() {
        assert_eq!(coerce_scalar("TRUE"), json!(true));
        assert_eq!(coerce_scalar("false"), json!(false));
        assert_eq!(coerce_scalar_cell(&Cell::Bool(true)), Some(json!(true)));
    }

    #[test]
    fn bracketed_text_is_read_as_json() {
        // This is how a whole array fits in one cell.
        assert_eq!(coerce_scalar(r#"["web", "ivr"]"#), json!(["web", "ivr"]));
        assert_eq!(coerce_scalar(r#"{"a": 1}"#), json!({"a": 1}));
    }

    #[test]
    fn text_that_only_looks_like_json_stays_text() {
        // A description that opens with a bracket is still a description.
        assert_eq!(coerce_scalar("[not json"), json!("[not json"));
        assert_eq!(coerce_scalar("[1, 2,]"), json!("[1, 2,]"));
    }

    #[test]
    fn a_candidate_named_like_a_keyword_is_not_reinterpreted() {
        // Unbracketed text is never parsed as JSON, so these survive.
        assert_eq!(coerce_scalar("NaN"), json!("NaN"));
        assert_eq!(coerce_scalar("Infinity"), json!("Infinity"));
    }

    #[test]
    fn a_naive_timestamp_is_read_as_utc() {
        // Guessing the author's local zone would shift a voting window.
        assert_eq!(
            coerce_scalar_cell(&at(2026, 10, 24, 16, 15)),
            Some(json!("2026-10-24T16:15:00+00:00"))
        );
    }

    #[test]
    fn a_multi_value_column_always_yields_a_list() {
        // Even with one value, or the emitted JSON type would follow the data.
        assert_eq!(
            coerce_cell(&Cell::text("a || b || c"), true),
            Some(json!(["a", "b", "c"]))
        );
        assert_eq!(
            coerce_cell(&Cell::text("only"), true),
            Some(json!(["only"]))
        );
        assert_eq!(coerce_cell(&Cell::Int(4), true), Some(json!([4])));
    }

    #[test]
    fn empty_pieces_of_a_multi_value_cell_are_dropped() {
        assert_eq!(
            coerce_cell(&Cell::text("a ||  || b"), true),
            Some(json!(["a", "b"]))
        );
        assert_eq!(coerce_cell(&Cell::text("||"), true), Some(json!([])));
    }

    #[test]
    fn a_blank_multi_value_cell_is_still_absent_not_an_empty_list() {
        // Absent must stay absent: an empty list would overwrite the template.
        assert_eq!(coerce_cell(&Cell::Blank, true), None);
    }

    #[test]
    fn setting_a_path_builds_the_objects_on_the_way() {
        let mut target = Map::new();
        set_path(
            &mut target,
            &split_path("presentation.i18n.en.name"),
            json!("President"),
        )
        .unwrap();
        assert_eq!(
            Value::Object(target),
            json!({"presentation": {"i18n": {"en": {"name": "President"}}}})
        );
    }

    #[test]
    fn two_columns_under_one_parent_share_it() {
        let expanded = expand(&[
            ("a.b".to_string(), json!(1)),
            ("a.c".to_string(), json!(2)),
        ])
        .unwrap();
        assert_eq!(Value::Object(expanded), json!({"a": {"b": 1, "c": 2}}));
    }

    #[test]
    fn a_later_column_wins_over_an_earlier_one() {
        // Left to right, the way someone reading the sheet would expect.
        let expanded =
            expand(&[("a".to_string(), json!(1)), ("a".to_string(), json!(2))])
                .unwrap();
        assert_eq!(Value::Object(expanded), json!({"a": 2}));
    }

    #[test]
    fn columns_that_disagree_about_the_shape_are_reported() {
        // `presentation` as a scalar and `presentation.i18n` cannot both hold.
        let problem = expand(&[
            ("presentation".to_string(), json!("plain")),
            ("presentation.i18n".to_string(), json!({})),
        ])
        .unwrap_err();
        assert_eq!(problem.code, Code::ConflictingColumns);
        assert!(problem.message.contains("already holds a string"));
    }

    #[test]
    fn a_null_on_the_way_through_is_replaced_rather_than_refused() {
        // "null" then a child column is a cleared cell followed by a set one,
        // not a contradiction.
        let expanded = expand(&[
            ("a".to_string(), Value::Null),
            ("a.b".to_string(), json!(1)),
        ])
        .unwrap();
        assert_eq!(Value::Object(expanded), json!({"a": {"b": 1}}));
    }

    #[test]
    fn a_header_naming_an_empty_field_is_refused() {
        assert_eq!(
            expand(&[("a.".to_string(), json!(1))]).unwrap_err().code,
            Code::ConflictingColumns
        );
        assert_eq!(
            expand(&[(".a".to_string(), json!(1))]).unwrap_err().code,
            Code::ConflictingColumns
        );
        assert_eq!(
            expand(&[("".to_string(), json!(1))]).unwrap_err().code,
            Code::ConflictingColumns
        );
    }

    #[test]
    fn merging_recurses_through_objects() {
        let merged = deep_merge(
            json!({"presentation": {"i18n": {"en": {"name": "A"}}}, "keep": 1}),
            json!({"presentation": {"i18n": {"es": {"name": "B"}}}}),
        );
        assert_eq!(
            merged,
            json!({
                "presentation": {"i18n": {"en": {"name": "A"}, "es": {"name": "B"}}},
                "keep": 1,
            })
        );
    }

    #[test]
    fn a_list_replaces_rather_than_appends() {
        // Three channels means exactly those three; appending would leave no way
        // to remove one the template had.
        let merged = deep_merge(
            json!({"voting_channels": ["web", "ivr", "paper"]}),
            json!({"voting_channels": ["web"]}),
        );
        assert_eq!(merged, json!({"voting_channels": ["web"]}));
    }

    #[test]
    fn an_explicit_null_clears_a_template_default() {
        let merged = deep_merge(
            json!({"description": "old"}),
            json!({"description": null}),
        );
        assert_eq!(merged, json!({"description": null}));
    }

    #[test]
    fn a_scalar_override_replaces_an_object() {
        assert_eq!(deep_merge(json!({"a": 1}), json!(3)), json!(3));
    }
}
