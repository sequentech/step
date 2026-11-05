// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Timezones in reports and notifications (VOTE-LIFECYCLE design §4, §7).
//!
//! Templates receive three reserved variables:
//! - `electionEventTimezone`: the event's primary zone (IANA);
//! - `electionTimezone`: the zone of the report's election (its own zone or
//!   the primary);
//! - `timezoneTexts`: the `timezones.*` texts in the event's default
//!   language: the defaults from `i18n_defaults.json` with the event's
//!   overrides for the `templates` scope applied (`templates:` > unprefixed >
//!   `global:`).
//!
//! The helpers `datetime_zone`, `timezone_label` and `timezone_name` print
//! through those texts, so no zone text is put together in code.

use crate::ballot::{ElectionEventPresentation, ElectionPresentation};
use crate::services::translation_scopes::template_translations;
use crate::time_zones::{
    effective_time_zone, primary_time_zone, DEFAULT_TIME_ZONE,
};
use chrono::{DateTime, Datelike, Offset, TimeZone, Utc};
use chrono_tz::{OffsetName, Tz};
use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output,
    RenderContext, RenderErrorReason,
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

/// Template variable: the event's primary zone (IANA).
pub const ELECTION_EVENT_TIMEZONE_VAR: &str = "electionEventTimezone";
/// Template variable: the election's zone (IANA), the primary for event-wide
/// reports.
pub const ELECTION_TIMEZONE_VAR: &str = "electionTimezone";
/// Template variable: the `timezones.*` texts the helpers print with.
pub const TIMEZONE_TEXTS_VAR: &str = "timezoneTexts";

pub const DATE_TIME_ZONE_KEY: &str = "timezones.dateTimeZone";
pub const VOTER_DATE_TIME_ZONE_KEY: &str = "timezones.voterDateTimeZone";
const DEFAULT_DATE_TIME_ZONE: &str = "{{dateTime}} {{zone}}";
const DEFAULT_VOTER_DATE_TIME_ZONE: &str = "{{dateTime}} {{zoneName}}";
/// `{{dateTime}}` when a template gives no `output_format`.
pub const DEFAULT_TEMPLATE_DATE_TIME_FORMAT: &str = "%B %d, %Y %H:%M";
/// The language whose defaults apply when the event's default language has
/// none.
const FALLBACK_LANGUAGE: &str = "en";

const ABBR_PREFIX: &str = "timezones.abbr.";
const ABBR_DAYLIGHT_PREFIX: &str = "timezones.abbrDaylight.";
const NAME_PREFIX: &str = "timezones.name.";
const NAME_DAYLIGHT_PREFIX: &str = "timezones.nameDaylight.";

/// The combined timezone strings and the placeholders each must keep
/// (`{{dateTime}}` and its zone placeholder). The Localization tabs refuse an
/// override that drops one (ui-core `translationScopes.ts` has the same
/// table); a stored one that does is ignored here.
pub const TIMEZONE_COMBINED_TEXTS: [(&str, &[&str]); 7] = [
    ("timezones.dateTimeZone", &["{{dateTime}}", "{{zone}}"]),
    ("timezones.myTime", &["{{dateTime}}", "{{zone}}"]),
    ("timezones.placeTime", &["{{dateTime}}", "{{zone}}"]),
    (
        "timezones.voterDateTimeZone",
        &["{{dateTime}}", "{{zoneName}}"],
    ),
    ("timezones.onThisDevice", &["{{dateTime}}"]),
    ("timezones.gap", &["{{dateTime}}", "{{city}}"]),
    ("timezones.overlap", &["{{dateTime}}", "{{city}}"]),
];

/// Whether `value` is unusable for the combined string `key`: it drops
/// `{{dateTime}}` or its zone placeholder. Other keys are always usable.
pub fn is_invalid_timezone_text(key: &str, value: &str) -> bool {
    let value = normalize_placeholders(value);
    TIMEZONE_COMBINED_TEXTS
        .iter()
        .find(|(combined_key, _)| *combined_key == key)
        .map(|(_, placeholders)| {
            placeholders
                .iter()
                .any(|placeholder| !value.contains(placeholder))
        })
        .unwrap_or(false)
}

/// The placeholders the timezone texts use.
const PLACEHOLDER_NAMES: [&str; 5] =
    ["dateTime", "zone", "zoneName", "city", "place"];

/// Writes i18next's spellings of a placeholder (`{{ dateTime }}`,
/// `{{- zone}}`, `{{dateTime, format}}`) as `{{name}}`, as ui-core's
/// `missingTimeZonePlaceholders` accepts them.
pub fn normalize_placeholders(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        normalized.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            normalized.push_str(&rest[start..]);
            return normalized;
        };
        // i18next's unescaped form puts the dash right after the braces
        // (`{{- zone}}`), as ui-core's check reads it.
        let inner = &after[..end];
        let inner = inner.strip_prefix('-').unwrap_or(inner);
        let name = inner.split(',').next().unwrap_or("").trim();
        if PLACEHOLDER_NAMES.contains(&name) {
            normalized.push_str("{{");
            normalized.push_str(name);
            normalized.push_str("}}");
        } else {
            normalized.push_str(&rest[start..start + 2 + end + 2]);
        }
        rest = &after[end + 2..];
    }
    normalized.push_str(rest);
    normalized
}

/// The `timezones.*` texts one render uses, flat (`timezones.abbr.Asia/Manila`):
/// the generated defaults and, apart, the event's overrides, because an
/// override of a label or name applies at every instant while the defaults
/// depend on daylight time.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimeZoneTexts {
    defaults: HashMap<String, String>,
    overrides: HashMap<String, String>,
}

/// Which of a zone's two yearly offsets is in force at an instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZoneVariant {
    /// The lower of the year's January and July offsets (or the only one).
    Standard,
    /// The higher one, in a zone whose January and July offsets differ.
    Daylight,
    /// Neither (Ramadan in Casablanca, a rule change): no default text fits.
    Other,
}

impl TimeZoneTexts {
    /// Builds the texts from one language of `i18n_defaults.json`
    /// (`{"timezones": {"abbr": {..}, ..}}`) and that language's stored
    /// event overrides (`presentation.i18n[lang]`, scoped keys).
    pub fn build(
        defaults: Option<&Value>,
        overrides: Option<&HashMap<String, Option<String>>>,
    ) -> Self {
        let mut texts = HashMap::new();
        if let Some(Value::Object(timezones)) =
            defaults.and_then(|defaults| defaults.get("timezones"))
        {
            flatten_into(&mut texts, "timezones", timezones);
        }
        texts
            .entry(DATE_TIME_ZONE_KEY.to_string())
            .or_insert_with(|| DEFAULT_DATE_TIME_ZONE.to_string());
        texts
            .entry(VOTER_DATE_TIME_ZONE_KEY.to_string())
            .or_insert_with(|| DEFAULT_VOTER_DATE_TIME_ZONE.to_string());

        let mut selected = HashMap::new();
        for (key, value) in
            overrides.map(template_translations).unwrap_or_default()
        {
            if !key.starts_with("timezones.") {
                continue;
            }
            if is_invalid_timezone_text(&key, &value) {
                warn!(
                    key,
                    value,
                    "Ignoring a timezone text override that drops {{{{dateTime}}}} or its zone placeholder; using the default"
                );
                continue;
            }
            selected.insert(key, normalize_placeholders(&value));
        }
        TimeZoneTexts {
            defaults: texts,
            overrides: selected,
        }
    }

    /// Reads the texts a caller put in the template variables.
    pub fn from_variables(variables: &Map<String, Value>) -> Self {
        let section = |name: &str| -> HashMap<String, String> {
            match variables
                .get(TIMEZONE_TEXTS_VAR)
                .and_then(|texts| texts.get(name))
            {
                Some(Value::Object(map)) => map
                    .iter()
                    .filter_map(|(key, value)| {
                        value
                            .as_str()
                            .map(|text| (key.clone(), text.to_string()))
                    })
                    .collect(),
                _ => HashMap::new(),
            }
        };
        TimeZoneTexts {
            defaults: section("defaults"),
            overrides: section("overrides"),
        }
    }

    pub fn to_value(&self) -> Value {
        let section = |texts: &HashMap<String, String>| {
            Value::Object(
                texts
                    .iter()
                    .map(|(key, value)| {
                        (key.clone(), Value::String(value.clone()))
                    })
                    .collect(),
            )
        };
        serde_json::json!({
            "defaults": section(&self.defaults),
            "overrides": section(&self.overrides),
        })
    }

    /// A text: the event's override, else the default.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.overrides
            .get(key)
            .or_else(|| self.defaults.get(key))
            .map(String::as_str)
    }

    /// The short label of `zone` at `at` (`timezones.abbr.<zone>`), the one
    /// label function every report and export shares.
    pub fn label(&self, zone: &str, at: DateTime<Utc>) -> String {
        self.zone_text(zone, at, ABBR_PREFIX, ABBR_DAYLIGHT_PREFIX)
            .unwrap_or_else(|| fallback_label(zone, at))
    }

    /// The long name of `zone` at `at` (`timezones.name.<zone>`).
    pub fn name(&self, zone: &str, at: DateTime<Utc>) -> String {
        self.zone_text(zone, at, NAME_PREFIX, NAME_DAYLIGHT_PREFIX)
            .or_else(|| {
                // Neither yearly offset: the zone's name still names it.
                self.defaults.get(&format!("{NAME_PREFIX}{zone}")).cloned()
            })
            .unwrap_or_else(|| {
                warn!(zone, "No timezone name text; using the zone id");
                zone.to_string()
            })
    }

    /// `{{dateTime}} {{zone}}` (`timezones.dateTimeZone`) or, for voters,
    /// `{{dateTime}} {{zoneName}}` (`timezones.voterDateTimeZone`).
    pub fn date_time_zone(
        &self,
        at: DateTime<Utc>,
        zone: &str,
        output_format: &str,
        style: DateTimeZoneStyle,
    ) -> String {
        let date_time = format_in_zone(at, zone, output_format);
        let (key, default) = match style {
            DateTimeZoneStyle::Plain => return date_time,
            DateTimeZoneStyle::Label => {
                (DATE_TIME_ZONE_KEY, DEFAULT_DATE_TIME_ZONE)
            }
            DateTimeZoneStyle::Voter => {
                (VOTER_DATE_TIME_ZONE_KEY, DEFAULT_VOTER_DATE_TIME_ZONE)
            }
        };
        let text = self
            .get(key)
            .filter(|text| !is_invalid_timezone_text(key, text))
            .unwrap_or(default);
        let mut rendered = text.replace("{{dateTime}}", &date_time);
        if rendered.contains("{{zoneName}}") {
            rendered = rendered.replace("{{zoneName}}", &self.name(zone, at));
        }
        if rendered.contains("{{zone}}") {
            rendered = rendered.replace("{{zone}}", &self.label(zone, at));
        }
        rendered
    }

    /// An override of the base text applies at every instant; an override of
    /// the daylight text only in daylight time. Defaults follow the offset in
    /// force; when it is neither of the year's offsets, no default fits.
    fn zone_text(
        &self,
        zone: &str,
        at: DateTime<Utc>,
        prefix: &str,
        daylight_prefix: &str,
    ) -> Option<String> {
        let base = format!("{prefix}{zone}");
        let daylight = format!("{daylight_prefix}{zone}");
        let variant = zone_variant(zone, at);
        let candidates: &[(&HashMap<String, String>, &str)] = match variant {
            ZoneVariant::Daylight => &[
                (&self.overrides, &daylight),
                (&self.overrides, &base),
                (&self.defaults, &daylight),
                (&self.defaults, &base),
            ],
            ZoneVariant::Standard => {
                &[(&self.overrides, &base), (&self.defaults, &base)]
            }
            ZoneVariant::Other => &[(&self.overrides, &base)],
        };
        candidates
            .iter()
            .find_map(|(texts, key)| texts.get(*key).cloned())
    }
}

/// Which combined string `datetime_zone` prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTimeZoneStyle {
    /// `timezones.dateTimeZone`: the short label (admin times, reports).
    Label,
    /// `timezones.voterDateTimeZone`: the long name (emails, voter texts).
    Voter,
    /// The date (and time) in the zone, without zone text: dates such as a
    /// voting period, next to a time that names the zone.
    Plain,
}

fn flatten_into(
    texts: &mut HashMap<String, String>,
    prefix: &str,
    object: &Map<String, Value>,
) {
    for (key, value) in object {
        let path = format!("{prefix}.{key}");
        match value {
            Value::String(text) => {
                texts.insert(path, text.clone());
            }
            Value::Object(child) => flatten_into(texts, &path, child),
            _ => {}
        }
    }
}

/// Parses an IANA zone. An unknown zone is logged and read as UTC, so a bad
/// configuration shows a UTC time with the UTC label instead of a wrong one.
fn parse_zone(zone: &str) -> Tz {
    zone.parse::<Tz>().unwrap_or_else(|_| {
        warn!(zone, "Unknown timezone in a template; using UTC");
        Tz::UTC
    })
}

/// The generator's rule (`scripts/gen-tz-defaults.mjs`) at render time:
/// daylight is the higher of the zone's offsets on 15 January and 15 July
/// of the instant's year.
fn zone_variant(zone: &str, at: DateTime<Utc>) -> ZoneVariant {
    let tz = parse_zone(zone);
    let offset_at = |instant: DateTime<Utc>| {
        tz.offset_from_utc_datetime(&instant.naive_utc())
            .fix()
            .local_minus_utc()
    };
    let reference = |month: u32| {
        Utc.with_ymd_and_hms(at.year(), month, 15, 12, 0, 0)
            .single()
            .map(offset_at)
    };
    let (Some(january), Some(july)) = (reference(1), reference(7)) else {
        return ZoneVariant::Standard;
    };
    let now = offset_at(at);
    if now == january.min(july) {
        ZoneVariant::Standard
    } else if now == january.max(july) {
        ZoneVariant::Daylight
    } else {
        ZoneVariant::Other
    }
}

/// `at` as wall time in `zone`, with a chrono `strftime` pattern.
pub fn format_in_zone(
    at: DateTime<Utc>,
    zone: &str,
    output_format: &str,
) -> String {
    at.with_timezone(&parse_zone(zone))
        .format(output_format)
        .to_string()
}

/// The tz database's abbreviation when it is a word (EDT, CET), else
/// `GMT+8` / `GMT+5:30`, like Intl's short names.
fn fallback_label(zone: &str, at: DateTime<Utc>) -> String {
    let tz = parse_zone(zone);
    let offset = tz.offset_from_utc_datetime(&at.naive_utc());
    if let Some(abbreviation) = offset.abbreviation() {
        if abbreviation.chars().all(|c| c.is_ascii_alphabetic()) {
            return abbreviation.to_string();
        }
    }
    let seconds = offset.fix().local_minus_utc();
    if seconds == 0 {
        return "GMT".to_string();
    }
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.abs() / 60;
    match minutes % 60 {
        0 => format!("GMT{sign}{}", minutes / 60),
        rest => format!("GMT{sign}{}:{rest:02}", minutes / 60),
    }
}

/// The reserved variables for a template rendered for `event` (and
/// `election`), in the event's default language. `i18n_defaults` is the whole
/// `i18n_defaults.json`.
pub fn template_time_variables(
    event: Option<&ElectionEventPresentation>,
    election: Option<&ElectionPresentation>,
    i18n_defaults: &Value,
) -> Map<String, Value> {
    let language = event
        .and_then(|event| event.language_conf.as_ref())
        .and_then(|conf| conf.default_language_code.clone())
        .unwrap_or_else(|| FALLBACK_LANGUAGE.to_string());
    let defaults = i18n_defaults
        .get(&language)
        .filter(|section| section.get("timezones").is_some())
        .or_else(|| i18n_defaults.get(FALLBACK_LANGUAGE));
    let overrides = event
        .and_then(|event| event.i18n.as_ref())
        .and_then(|i18n| i18n.get(&language));
    let texts = TimeZoneTexts::build(defaults, overrides);

    let mut variables = Map::new();
    variables.insert(
        ELECTION_EVENT_TIMEZONE_VAR.to_string(),
        Value::String(primary_time_zone(event)),
    );
    variables.insert(
        ELECTION_TIMEZONE_VAR.to_string(),
        Value::String(effective_time_zone(event, election)),
    );
    variables.insert(TIMEZONE_TEXTS_VAR.to_string(), texts.to_value());
    variables
}

/// Copies the timezone variables from `source` (an object such as a
/// pipeline's `extra_data`) into a template's variables, so a renderer that
/// receives them nested still gives the helpers their zone and texts.
pub fn copy_template_time_variables(
    source: &Value,
    target: &mut Map<String, Value>,
) {
    for key in [
        ELECTION_EVENT_TIMEZONE_VAR,
        ELECTION_TIMEZONE_VAR,
        TIMEZONE_TEXTS_VAR,
    ] {
        if let Some(value) = source.get(key) {
            target.insert(key.to_string(), value.clone());
        }
    }
}

/// What the helpers of one render know: the texts and the zone they use
/// when a template names none.
#[derive(Debug, Clone)]
pub(super) struct TemplateTimeContext {
    texts: TimeZoneTexts,
    default_zone: String,
}

impl TemplateTimeContext {
    pub(super) fn from_variables(variables: &Map<String, Value>) -> Self {
        let zone_var = |name: &str| {
            variables
                .get(name)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|zone| !zone.is_empty())
                .map(str::to_string)
        };
        TemplateTimeContext {
            texts: TimeZoneTexts::from_variables(variables),
            default_zone: zone_var(ELECTION_TIMEZONE_VAR)
                .or_else(|| zone_var(ELECTION_EVENT_TIMEZONE_VAR))
                .unwrap_or_else(|| DEFAULT_TIME_ZONE.to_string()),
        }
    }
}

/// The helpers' output, safe in the places templates print it: HTML text
/// and attributes, and the JavaScript template literals of the default
/// templates' QR codes (no backtick, `${`, or backslash gets through).
/// Overrides are admin text, but a label must never become markup or code.
pub fn escape_template_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            '`' => escaped.push_str("&#x60;"),
            '=' => escaped.push_str("&#x3D;"),
            '$' => escaped.push_str("&#36;"),
            '\\' => escaped.push_str("&#92;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

fn param_instant(
    helper: &Helper,
    index: usize,
    name: &'static str,
) -> Result<Option<DateTime<Utc>>, RenderErrorReason> {
    let Some(param) = helper.param(index) else {
        return Ok(None);
    };
    match param.value() {
        Value::String(text) if !text.trim().is_empty() => {
            DateTime::parse_from_rfc3339(text.trim())
                .map(|at| Some(at.with_timezone(&Utc)))
                .map_err(|_| {
                    RenderErrorReason::InvalidParamType(
                        "an RFC 3339 date and time",
                    )
                })
        }
        Value::Number(number) => number
            .as_i64()
            .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
            .map(Some)
            .ok_or(RenderErrorReason::InvalidParamType("unix seconds")),
        Value::Null => {
            Err(RenderErrorReason::ParamNotFoundForIndex(name, index))
        }
        Value::String(_) => {
            Err(RenderErrorReason::ParamNotFoundForIndex(name, index))
        }
        _ => Err(RenderErrorReason::InvalidParamType(
            "an RFC 3339 date and time or unix seconds",
        )),
    }
}

fn param_zone(helper: &Helper, index: usize) -> Option<String> {
    helper
        .param(index)
        .and_then(|param| param.value().as_str())
        .map(str::trim)
        .filter(|zone| !zone.is_empty())
        .map(str::to_string)
}

fn hash_str<'a>(helper: &'a Helper, name: &str) -> Option<&'a str> {
    helper
        .hash_get(name)
        .and_then(|value| value.value().as_str())
}

/// `{{datetime_zone instant [zone] output_format="%B %d, %Y %H:%M" style="voter"}}`
///
/// Prints `instant` (RFC 3339 or unix seconds) in `zone` (default: the
/// election's, else the event's primary) through `timezones.dateTimeZone`,
/// `timezones.voterDateTimeZone` with `style="voter"`, or without zone text
/// with `style="plain"`.
pub(super) struct DateTimeZoneHelper(pub(super) Arc<TemplateTimeContext>);

impl HelperDef for DateTimeZoneHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let at = param_instant(helper, 0, "datetime_zone")?.ok_or(
            RenderErrorReason::ParamNotFoundForIndex("datetime_zone", 0),
        )?;
        let zone = param_zone(helper, 1)
            .unwrap_or_else(|| self.0.default_zone.clone());
        let output_format = hash_str(helper, "output_format")
            .unwrap_or(DEFAULT_TEMPLATE_DATE_TIME_FORMAT);
        let style = match hash_str(helper, "style") {
            Some("voter") => DateTimeZoneStyle::Voter,
            Some("plain") => DateTimeZoneStyle::Plain,
            _ => DateTimeZoneStyle::Label,
        };
        out.write(&escape_template_text(&self.0.texts.date_time_zone(
            at,
            &zone,
            output_format,
            style,
        )))?;
        Ok(())
    }
}

/// Which zone text a [`TimeZoneTextHelper`] prints.
#[derive(Debug, Clone, Copy)]
pub(super) enum ZoneText {
    Label,
    Name,
}

/// `{{timezone_label [zone] [instant]}}` and `{{timezone_name [zone] [instant]}}`:
/// the zone's short label or long name at `instant` (default: now). Zone
/// defaults to the election's, else the event's primary.
pub(super) struct TimeZoneTextHelper(
    pub(super) Arc<TemplateTimeContext>,
    pub(super) ZoneText,
);

impl HelperDef for TimeZoneTextHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let zone = param_zone(helper, 0)
            .unwrap_or_else(|| self.0.default_zone.clone());
        let name = match self.1 {
            ZoneText::Label => "timezone_label",
            ZoneText::Name => "timezone_name",
        };
        let at = param_instant(helper, 1, name)
            .ok()
            .flatten()
            .unwrap_or_else(Utc::now);
        let text = match self.1 {
            ZoneText::Label => self.0.texts.label(&zone, at),
            ZoneText::Name => self.0.texts.name(&zone, at),
        };
        out.write(&escape_template_text(&text))?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "reports_time_zones_tests.rs"]
mod reports_time_zones_tests;
