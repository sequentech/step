// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{bail, Context, Result};
use sequent_core::ballot::ElectionEventPresentation;
use serde_json::{Map, Value};

/// The presentation stored for an election event, `stored`, with the settings
/// that `changes` sets, and every other setting as stored.
///
/// Writing back a whole `ElectionEventPresentation` would instead store every
/// setting as this version reads it, losing what it doesn't know: a newer
/// version's setting, or a number format it reads as none.
pub fn change_presentation(stored: Value, changes: ElectionEventPresentation) -> Result<Value> {
    let mut presentation = match stored {
        Value::Null => Map::new(),
        Value::Object(presentation) => presentation,
        _ => bail!("The stored presentation is not a JSON object"),
    };
    let Value::Object(changes) =
        serde_json::to_value(changes).context("Failed to serialize the presentation changes")?
    else {
        bail!("The presentation changes are not a JSON object");
    };
    presentation.extend(changes.into_iter().filter(|(_, value)| !value.is_null()));
    Ok(Value::Object(presentation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::{Enrollment, LockedDown};
    use serde_json::json;

    #[test]
    fn a_change_keeps_the_settings_this_version_does_not_know() -> Result<()> {
        let stored = json!({
            "locked_down": "not-locked-down",
            "enrollment": "enabled",
            "number_format_policy": "a-newer-format",
            "a_newer_setting": {"enabled": true},
        });
        let changes = ElectionEventPresentation {
            locked_down: Some(LockedDown::LOCKED_DOWN),
            ..Default::default()
        };
        assert_eq!(
            change_presentation(stored, changes)?,
            json!({
                "locked_down": "locked-down",
                "enrollment": "enabled",
                "number_format_policy": "a-newer-format",
                "a_newer_setting": {"enabled": true},
            })
        );
        Ok(())
    }

    #[test]
    fn a_change_sets_each_setting_it_names() -> Result<()> {
        let changes = ElectionEventPresentation {
            enrollment: Some(Enrollment::DISABLED),
            results_website: Some("{\"status\":\"enabled\"}".to_string()),
            ..Default::default()
        };
        assert_eq!(
            change_presentation(json!({"enrollment": "enabled"}), changes)?,
            json!({
                "enrollment": "disabled",
                "results_website": "{\"status\":\"enabled\"}",
            })
        );
        Ok(())
    }

    #[test]
    fn an_event_without_a_presentation_gets_only_the_change() -> Result<()> {
        let changes = ElectionEventPresentation {
            locked_down: Some(LockedDown::LOCKED_DOWN),
            ..Default::default()
        };
        assert_eq!(
            change_presentation(Value::Null, changes)?,
            json!({"locked_down": "locked-down"})
        );
        Ok(())
    }

    #[test]
    fn a_presentation_that_is_not_an_object_is_refused() {
        let changes = ElectionEventPresentation {
            locked_down: Some(LockedDown::LOCKED_DOWN),
            ..Default::default()
        };
        assert!(change_presentation(json!("locked-down"), changes).is_err());
    }
}
