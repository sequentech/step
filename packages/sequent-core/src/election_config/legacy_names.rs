// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Names in exports from before they moved into `presentation`.
//!
//! Events, elections, contests and candidates lost their `name` column when
//! names moved to `presentation.i18n.<lang>.name`; the migration copied each
//! stored name to `en`. An export written before then still carries the plain
//! `name`, which the bundle no longer has a field for, so importing it as is
//! loses the name without a word.

use super::problem::{Code, Problem, Report};
use serde_json::{json, Map, Value};

const LANGUAGE: &str = "en";

/// Copies each plain `name` to `presentation.i18n.en.name`, as the migration
/// did, when that localized name is missing or blank. A localized name that is
/// already there is kept, and a different plain name is reported as a warning.
///
/// Returns whether the document changed.
pub fn adopt_legacy_names(document: &mut Value, report: &mut Report) -> bool {
    let mut changed = false;
    if let Some(event) = document.get_mut("election_event") {
        changed |= adopt(event, "election_event", report);
    }
    for collection in ["elections", "contests", "candidates"] {
        let Some(entities) =
            document.get_mut(collection).and_then(Value::as_array_mut)
        else {
            continue;
        };
        for (index, entity) in entities.iter_mut().enumerate() {
            changed |= adopt(entity, &format!("{collection}[{index}]"), report);
        }
    }
    changed
}

fn adopt(entity: &mut Value, path: &str, report: &mut Report) -> bool {
    let Some(entity) = entity.as_object_mut() else {
        return false;
    };
    let Some(legacy) = entity
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
    else {
        return false;
    };
    let localized = entity
        .get("presentation")
        .and_then(|presentation| presentation.get("i18n"))
        .and_then(|i18n| i18n.get(LANGUAGE))
        .and_then(|language| language.get("name"))
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty());
    if let Some(localized) = localized {
        if localized != legacy {
            report.push(Problem::warning(
                Code::ConflictingColumns,
                format!("{path}.name"),
                format!(
                    "the old name field says '{legacy}' and \
                     presentation.i18n.{LANGUAGE}.name says '{localized}'; \
                     the localized name is kept"
                ),
            ));
        }
        return false;
    }
    let Some(language) = object_at(entity, &["presentation", "i18n", LANGUAGE])
    else {
        return false;
    };
    language.insert("name".to_string(), json!(legacy));
    true
}

/// The object at `keys`, creating missing or null levels. `None` when a level
/// is some other value, which is not this function's to overwrite.
fn object_at<'a>(
    mut object: &'a mut Map<String, Value>,
    keys: &[&str],
) -> Option<&'a mut Map<String, Value>> {
    for key in keys {
        let value = object.entry(key.to_string()).or_insert(Value::Null);
        if value.is_null() {
            *value = json!({});
        }
        object = value.as_object_mut()?;
    }
    Some(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(entity: &Value) -> &Value {
        &entity["presentation"]["i18n"]["en"]["name"]
    }

    #[test]
    fn an_old_export_keeps_every_name() {
        let mut document = json!({
            "election_event": {"name": "Event", "presentation": null},
            "elections": [{"name": "Election", "presentation": {"i18n": {"en": {}}}}],
            "contests": [{"name": "Contest"}],
            "candidates": [{"name": "Candidate", "presentation": {"i18n": {"en": {"name": ""}}}}],
        });
        let mut report = Report::default();

        assert!(adopt_legacy_names(&mut document, &mut report));

        assert_eq!(name(&document["election_event"]), "Event");
        assert_eq!(name(&document["elections"][0]), "Election");
        assert_eq!(name(&document["contests"][0]), "Contest");
        assert_eq!(name(&document["candidates"][0]), "Candidate");
        assert!(report.is_empty(), "{report:?}");
    }

    #[test]
    fn a_localized_name_is_never_replaced() {
        let mut document = json!({
            "election_event": {
                "name": "Old",
                "presentation": {"i18n": {"en": {"name": "Localized"}, "es": {"name": "Localizado"}}},
            },
            "elections": [{"name": "Same", "presentation": {"i18n": {"en": {"name": "Same"}}}}],
        });
        let mut report = Report::default();

        assert!(!adopt_legacy_names(&mut document, &mut report));

        assert_eq!(name(&document["election_event"]), "Localized");
        assert_eq!(
            document["election_event"]["presentation"]["i18n"]["es"]["name"],
            "Localizado"
        );
        let warnings: Vec<_> = report.warnings().collect();
        assert_eq!(warnings.len(), 1, "{report:?}");
        assert_eq!(warnings[0].path, "election_event.name");
        assert!(!report.has_errors());
    }

    #[test]
    fn a_current_export_is_left_alone() {
        let current = json!({
            "election_event": {"presentation": {"i18n": {"en": {"name": "Event"}}}},
            "elections": [{"presentation": {"i18n": {"en": {"name": "Election"}}}}],
            "contests": [],
            "candidates": [{"name": "", "presentation": null}],
        });
        let mut document = current.clone();
        let mut report = Report::default();

        assert!(!adopt_legacy_names(&mut document, &mut report));
        assert_eq!(document, current);
        assert!(report.is_empty());
    }

    #[test]
    fn a_presentation_of_another_shape_is_not_overwritten() {
        let mut document =
            json!({"contests": [{"name": "Contest", "presentation": "text"}]});
        let mut report = Report::default();

        assert!(!adopt_legacy_names(&mut document, &mut report));
        assert_eq!(document["contests"][0]["presentation"], "text");
    }
}
