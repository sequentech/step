// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Handlebars rendering of the base entity templates.
//!
//! The templates supply the platform boilerplate; the source document overrides
//! it. That split is what keeps client configuration out of the code: a delivery
//! engineer who needs a different default supplies their own template, without
//! touching the tool.
//!
//! Only ids, timestamps and enum values are interpolated. Client text — names,
//! descriptions, CSS — is deep-merged into the *parsed* result instead, so a
//! spreadsheet cell containing a quote or a backslash cannot produce invalid
//! JSON.
//!
//! The builtin templates are compiled into the binary rather than read from
//! disk, because a browser has no directory to read and downloading eight files
//! before rendering anything would be absurd. Overrides are handed in as text by
//! whoever has a way to obtain them: `step-cli` reads a directory, a SPA can take
//! an upload.
//!
//! Behind the `election_config_templates` feature, so a front end that only
//! validates an existing bundle does not carry a template engine.

use crate::election_config::problem::{Code, Problem};
use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext,
    RenderErrorReason,
};
use serde_json::{Map, Value};

/// Entity templates that get rendered. An override may replace any of them.
pub const ENTITY_TEMPLATES: &[&str] = &[
    "election_event",
    "election",
    "contest",
    "candidate",
    "area",
    "area_contest",
    "scheduled_event",
    "report",
];

/// The builtin source for each template, compiled in.
pub const BUILTIN_TEMPLATES: &[(&str, &str)] = &[
    (
        "election_event",
        include_str!("templates/election_event.hbs"),
    ),
    ("election", include_str!("templates/election.hbs")),
    ("contest", include_str!("templates/contest.hbs")),
    ("candidate", include_str!("templates/candidate.hbs")),
    ("area", include_str!("templates/area.hbs")),
    ("area_contest", include_str!("templates/area_contest.hbs")),
    (
        "scheduled_event",
        include_str!("templates/scheduled_event.hbs"),
    ),
    ("report", include_str!("templates/report.hbs")),
];

/// Escape an interpolated value for the inside of a JSON string.
///
/// Handlebars' default escape function is for HTML, which is the wrong language
/// here: it would turn a quote into `&quot;`, which is valid JSON holding the
/// wrong text. Turning escaping off instead would let a stray quote break the
/// document.
///
/// The builtin templates only interpolate ids, timestamps and enum values, none
/// of which can contain anything needing this. It exists so that a custom
/// template which does interpolate something less disciplined still renders
/// parseable JSON.
fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            // The control characters JSON forbids raw.
            character if (character as u32) < 0x20 => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

/// `{{json value}}` — emit `value` as JSON.
///
/// For custom templates that do need to interpolate arbitrary data: a client
/// template building an annotations block from spreadsheet columns needs a safe
/// way to do it. Writes straight to the output, which is how it escapes the
/// escaping — the whole point is to emit a JSON literal, and `\"` around an
/// object would not be one.
fn helper_json(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let value = helper.param(0).map_or(&Value::Null, |param| param.value());
    let encoded = serde_json::to_string(value).map_err(|error| {
        RenderErrorReason::Other(format!("could not encode as JSON: {error}"))
    })?;
    out.write(&encoded)?;
    Ok(())
}

/// `{{default value "fallback"}}` — `fallback` when the value is absent or empty.
///
/// Escapes for a JSON string itself, because a helper's output bypasses the
/// engine's escape function and this one is meant to land inside quotes.
fn helper_default(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let fallback = helper.param(1).map_or(&Value::Null, |param| param.value());
    let value = helper.param(0).map_or(&Value::Null, |param| param.value());

    let chosen = match value {
        Value::Null => fallback,
        Value::String(text) if text.is_empty() => fallback,
        value => value,
    };

    let text = match chosen {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    out.write(&escape_json_string(&text))?;
    Ok(())
}

/// Compiled entity templates, with optional overrides.
pub struct TemplateSet {
    handlebars: Handlebars<'static>,
    overridden: Vec<String>,
}

impl std::fmt::Debug for TemplateSet {
    /// The compiled templates are noise; which ones were replaced is the only
    /// thing worth reading in a failure message.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TemplateSet")
            .field("overridden", &self.overridden)
            .finish_non_exhaustive()
    }
}

impl TemplateSet {
    /// The templates compiled into the binary.
    pub fn builtin() -> Result<Self, Problem> {
        Self::with_overrides(&[])
    }

    /// The builtins, with any of them replaced by `overrides`.
    ///
    /// An override for a name that is not an entity template is refused rather
    /// than ignored: a file called `elections.hbs` in a templates directory is
    /// someone's typo, and silently rendering the builtin instead would leave
    /// them staring at output that does not reflect their edit.
    pub fn with_overrides(overrides: &[(&str, &str)]) -> Result<Self, Problem> {
        let mut handlebars = Handlebars::new();
        handlebars.register_escape_fn(escape_json_string);
        handlebars.set_strict_mode(false);
        handlebars.register_helper("json", Box::new(helper_json));
        handlebars.register_helper("default", Box::new(helper_default));

        for (name, _) in overrides {
            if !ENTITY_TEMPLATES.contains(name) {
                return Err(Problem::error(
                    Code::InvalidValue,
                    format!("templates.{name}"),
                    format!(
                        "'{name}' is not an entity template. Expected one of: {}.",
                        ENTITY_TEMPLATES.join(", ")
                    ),
                ));
            }
        }

        let mut overridden = Vec::new();
        for (name, builtin) in BUILTIN_TEMPLATES {
            let source = match overrides
                .iter()
                .find(|(override_name, _)| override_name == name)
            {
                Some((_, source)) => {
                    overridden.push((*name).to_string());
                    *source
                }
                None => *builtin,
            };

            handlebars.register_template_string(name, source).map_err(
                |error| {
                    Problem::error(
                        Code::InvalidValue,
                        format!("templates.{name}"),
                        format!("could not be compiled: {error}"),
                    )
                },
            )?;
        }

        overridden.sort();
        Ok(TemplateSet {
            handlebars,
            overridden,
        })
    }

    /// Templates being taken from an override, for a caller to report.
    pub fn overridden(&self) -> &[String] {
        &self.overridden
    }

    /// Render `name` and return the raw text.
    pub fn render(
        &self,
        name: &str,
        context: &Value,
    ) -> Result<String, Problem> {
        self.handlebars.render(name, context).map_err(|error| {
            Problem::error(
                Code::InvalidValue,
                format!("templates.{name}"),
                format!("could not be rendered: {error}"),
            )
        })
    }

    /// Render `name` and parse the result as a JSON object.
    ///
    /// A template that renders invalid JSON is a bug in the template, and the
    /// message quotes the lines around the failure — debugging that from a bare
    /// parse error is otherwise miserable.
    pub fn render_json(
        &self,
        name: &str,
        context: &Value,
    ) -> Result<Map<String, Value>, Problem> {
        let rendered = self.render(name, context)?;

        let parsed: Value =
            serde_json::from_str(&rendered).map_err(|error| {
                Problem::error(
                    Code::InvalidValue,
                    format!("templates.{name}"),
                    format!(
                        "did not render valid JSON: {error}\n{}",
                        excerpt(&rendered, Some(error.line()))
                    ),
                )
            })?;

        match parsed {
            Value::Object(object) => Ok(object),
            other => Err(Problem::error(
                Code::InvalidValue,
                format!("templates.{name}"),
                format!(
                    "rendered a {}, expected a JSON object",
                    match other {
                        Value::Null => "null",
                        Value::Bool(_) => "boolean",
                        Value::Number(_) => "number",
                        Value::String(_) => "string",
                        Value::Array(_) => "list",
                        Value::Object(_) => unreachable!(),
                    }
                ),
            )),
        }
    }
}

/// The lines around a parse failure, numbered, with the offending one marked.
fn excerpt(text: &str, line: Option<usize>) -> String {
    const CONTEXT: usize = 2;
    let lines: Vec<&str> = text.lines().collect();

    let Some(line) = line.filter(|line| *line > 0 && !lines.is_empty()) else {
        return lines
            .iter()
            .take(20)
            .enumerate()
            .map(|(index, text)| format!("  {:4} | {text}", index + 1))
            .collect::<Vec<_>>()
            .join("\n");
    };

    let start = line.saturating_sub(1 + CONTEXT);
    let end = (line + CONTEXT).min(lines.len());
    (start..end)
        .map(|index| {
            let marker = if index == line - 1 { '>' } else { ' ' };
            format!("{marker} {:4} | {}", index + 1, lines[index])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn context() -> Value {
        json!({
            "id": "11111111-1111-5111-8111-111111111111",
            "tenant_id": "22222222-2222-5222-8222-222222222222",
            "election_event_id": "33333333-3333-5333-8333-333333333333",
            "election_id": "44444444-4444-5444-8444-444444444444",
            "area_id": "55555555-5555-5555-8555-555555555555",
            "contest_id": "66666666-6666-5666-8666-666666666666",
            "created_at": "2026-01-01T00:00:00+00:00",
            "task_id": "tenant_x_event_y_manage_election_date",
            "event_processor": "manage_election_date",
            "report_type": "tally",
        })
    }

    #[test]
    fn every_named_template_is_compiled_in() {
        // A name in the list with no source behind it fails at render time, in
        // whichever tool happens to reach it first.
        for name in ENTITY_TEMPLATES {
            assert!(
                BUILTIN_TEMPLATES.iter().any(|(builtin, _)| builtin == name),
                "{name} has no builtin source"
            );
        }
        assert_eq!(BUILTIN_TEMPLATES.len(), ENTITY_TEMPLATES.len());
    }

    #[test]
    fn every_builtin_renders_valid_json() {
        // The templates are data, so nothing else would catch a stray comma.
        let templates = TemplateSet::builtin().unwrap();
        for name in ENTITY_TEMPLATES {
            let rendered = templates.render_json(name, &context());
            assert!(rendered.is_ok(), "{name}: {:?}", rendered.unwrap_err());
        }
    }

    #[test]
    fn the_ids_arrive_where_the_template_puts_them() {
        let templates = TemplateSet::builtin().unwrap();
        let contest = templates.render_json("contest", &context()).unwrap();
        assert_eq!(
            contest["id"],
            json!("11111111-1111-5111-8111-111111111111")
        );
        assert_eq!(
            contest["election_id"],
            json!("44444444-4444-5444-8444-444444444444")
        );
        // last_updated_at reuses created_at, which is what makes a regenerated
        // bundle diff cleanly.
        assert_eq!(contest["created_at"], contest["last_updated_at"]);
    }

    #[test]
    fn a_missing_context_value_renders_empty_rather_than_failing() {
        // Strict mode off on purpose: a template referring to a field this entity
        // has no value for should leave it blank for the merge to fill, not stop
        // the build.
        let templates = TemplateSet::builtin().unwrap();
        let area = templates.render_json("area", &json!({})).unwrap();
        assert_eq!(area["id"], json!(""));
    }

    #[test]
    fn an_interpolated_quote_cannot_break_the_document() {
        // The reason the escape function is JSON and not HTML. The builtin
        // templates never interpolate anything like this; a custom one might.
        let templates =
            TemplateSet::with_overrides(&[("area", r#"{"name": "{{id}}"}"#)])
                .unwrap();
        let area = templates
            .render_json("area", &json!({"id": r#"a "quoted" \ name"#}))
            .unwrap();
        assert_eq!(area["name"], json!(r#"a "quoted" \ name"#));
    }

    #[test]
    fn an_interpolated_newline_survives_as_an_escape() {
        let templates =
            TemplateSet::with_overrides(&[("area", r#"{"name": "{{id}}"}"#)])
                .unwrap();
        let area = templates
            .render_json("area", &json!({"id": "two\nlines"}))
            .unwrap();
        assert_eq!(area["name"], json!("two\nlines"));
    }

    #[test]
    fn the_json_helper_emits_a_literal_not_a_quoted_string() {
        // What it is for: a whole object from one context value.
        let templates = TemplateSet::with_overrides(&[(
            "area",
            r#"{"annotations": {{json extra}}}"#,
        )])
        .unwrap();
        let area = templates
            .render_json("area", &json!({"extra": {"a": [1, 2]}}))
            .unwrap();
        assert_eq!(area["annotations"], json!({"a": [1, 2]}));
    }

    #[test]
    fn the_json_helper_escapes_what_it_encodes() {
        let templates = TemplateSet::with_overrides(&[(
            "area",
            r#"{"annotations": {{json extra}}}"#,
        )])
        .unwrap();
        let area = templates
            .render_json("area", &json!({"extra": {"k": "a \"quote\""}}))
            .unwrap();
        assert_eq!(area["annotations"]["k"], json!("a \"quote\""));
    }

    #[test]
    fn the_default_helper_fills_in_for_absent_and_empty() {
        let templates = TemplateSet::with_overrides(&[(
            "area",
            r#"{"a": "{{default missing "fallback"}}", "b": "{{default blank "fallback"}}", "c": "{{default present "fallback"}}"}"#,
        )])
        .unwrap();
        let area = templates
            .render_json("area", &json!({"blank": "", "present": "kept"}))
            .unwrap();
        assert_eq!(area["a"], json!("fallback"));
        assert_eq!(area["b"], json!("fallback"));
        assert_eq!(area["c"], json!("kept"));
    }

    #[test]
    fn the_default_helper_escapes_for_the_string_it_lands_in() {
        // A helper's output bypasses the engine's escape function, so it has to
        // do its own or a quote in a fallback breaks the document.
        let templates = TemplateSet::with_overrides(&[(
            "area",
            r#"{"a": "{{default missing "say \"hi\""}}"}"#,
        )])
        .unwrap();
        let area = templates.render_json("area", &json!({})).unwrap();
        assert_eq!(area["a"], json!(r#"say "hi""#));
    }

    #[test]
    fn an_override_replaces_the_builtin_and_is_reported() {
        let templates =
            TemplateSet::with_overrides(&[("area", r#"{"mine": true}"#)])
                .unwrap();
        assert_eq!(templates.overridden(), ["area".to_string()]);
        assert_eq!(
            templates.render_json("area", &context()).unwrap()["mine"],
            json!(true)
        );
        // The rest still come from the builtins.
        assert!(templates.overridden().len() == 1);
        assert!(templates.render_json("contest", &context()).is_ok());
    }

    #[test]
    fn an_override_for_a_name_nobody_renders_is_refused() {
        // Otherwise a typo means silently rendering the builtin, and the author
        // staring at output that ignores their edit.
        let problem =
            TemplateSet::with_overrides(&[("elections", "{}")]).unwrap_err();
        assert_eq!(problem.code, Code::InvalidValue);
        assert!(problem.message.contains("not an entity template"));
    }

    #[test]
    fn a_template_that_does_not_compile_names_itself() {
        let problem =
            TemplateSet::with_overrides(&[("area", "{{#if}}")]).unwrap_err();
        assert_eq!(problem.path, "templates.area");
        assert!(problem.message.contains("could not be compiled"));
    }

    #[test]
    fn a_template_that_renders_broken_json_quotes_the_offending_line() {
        // A bare parse error against 100 lines of rendered output is not
        // debuggable.
        let templates = TemplateSet::with_overrides(&[(
            "area",
            "{\n  \"a\": 1,\n  \"b\": ,\n  \"c\": 3\n}",
        )])
        .unwrap();
        let problem = templates.render_json("area", &context()).unwrap_err();
        assert!(problem.message.contains("did not render valid JSON"));
        assert!(
            problem.message.contains(r#">    3 |   "b": ,"#),
            "{}",
            problem.message
        );
    }

    #[test]
    fn a_template_that_renders_something_other_than_an_object_is_refused() {
        let templates =
            TemplateSet::with_overrides(&[("area", "[1, 2]")]).unwrap();
        let problem = templates.render_json("area", &context()).unwrap_err();
        assert!(problem.message.contains("rendered a list"));
    }

    #[test]
    fn escaping_leaves_ordinary_text_alone() {
        // Cheap to get wrong in a way that mangles every name in a bundle.
        assert_eq!(
            escape_json_string("Board of Directors"),
            "Board of Directors"
        );
        assert_eq!(escape_json_string("José-Muñoz"), "José-Muñoz");
        assert_eq!(escape_json_string("a\u{1}b"), "a\\u0001b");
    }
}
