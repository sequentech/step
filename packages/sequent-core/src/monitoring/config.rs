// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Configuration as authored.
//!
//! Four kinds, each stored and revisioned on its own: a widget, a dashboard, a
//! theme, and the event's settings. Every struct refuses unknown fields, so a
//! misspelt key is a validation error instead of a setting that silently does
//! nothing — and so there is nowhere to put a counting rule or an access check,
//! which configuration is not allowed to change.
//!
//! Types here only describe shape. What makes a shaped document acceptable is
//! [`super::policy`].

use super::sources::{
    BuiltinDimension, DataSourceId, Measure, QueryTemplate, TimeGrain,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// Which of the four kinds a document is.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ConfigKind {
    Dashboard,
    Widget,
    Theme,
    Settings,
}

/// Who may change a kind of document.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum Editability {
    /// Administrators edit it in the Admin Portal.
    Editor,
    /// Only resetting the event to a preset writes it. Settings decide who
    /// counts as pre-enrolled, which is a denominator, so an editor must not
    /// be able to change them.
    PresetOnly,
}

impl ConfigKind {
    pub fn editability(self) -> Editability {
        match self {
            ConfigKind::Dashboard | ConfigKind::Widget | ConfigKind::Theme => {
                Editability::Editor
            }
            ConfigKind::Settings => Editability::PresetOnly,
        }
    }
}

/// A dashboard selector: one that applies to every widget that follows it.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ScopeSelector {
    Region,
    /// An election. On an election's own page it is pinned to that election.
    Post,
    Country,
}

impl ScopeSelector {
    /// The dimension a source must have to be narrowed by this selector.
    pub fn dimension(self) -> BuiltinDimension {
        match self {
            ScopeSelector::Region => BuiltinDimension::Region,
            ScopeSelector::Post => BuiltinDimension::Post,
            ScopeSelector::Country => BuiltinDimension::Country,
        }
    }
}

/// One widget: a data source, its queries, its selectors and its chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Widget {
    pub id: String,
    pub title: String,
    pub source: DataSourceId,

    /// The monitoring requirements this widget answers, for the catalog
    /// search. Free text: a requirement ID is the customer's, not ours.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<String>,

    /// Which dashboard selectors narrow this widget. Absent means every one
    /// its source can be narrowed by; an empty list means the widget always
    /// shows the whole event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follows: Option<Vec<ScopeSelector>>,

    /// Selectors shown in the widget's header, in the order declared.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub selectors: IndexMap<String, Selector>,

    /// The widget's one query, which charts read as `data`. Exactly one of
    /// `query` and `queries` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<Query>,

    /// Several named queries, for a chart that reads more than one.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub queries: IndexMap<String, Query>,

    /// dbt Charts YAML: everything about how the widget looks. Kept as a
    /// value because dbt Charts owns its schema; the policy walks it for what
    /// it must not contain, and the renderer checks the rest.
    pub chart: serde_yaml::Value,

    /// Frame height in pixels. The chart frame runs no script, so it cannot
    /// measure its own content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// The name a chart reads a widget's single `query` by.
pub const DEFAULT_QUERY_NAME: &str = "data";

impl Widget {
    /// The widget's governed queries by name, whichever way they were written.
    pub fn named_queries(&self) -> IndexMap<String, &Query> {
        match &self.query {
            Some(query) => {
                IndexMap::from([(DEFAULT_QUERY_NAME.to_string(), query)])
            }
            None => self
                .queries
                .iter()
                .map(|(name, query)| (name.clone(), query))
                .collect(),
        }
    }

    pub fn follows(&self, selector: ScopeSelector) -> bool {
        self.follows
            .as_ref()
            .map_or(true, |follows| follows.contains(&selector))
    }
}

/// How a selector is drawn.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SelectorControl {
    #[default]
    Dropdown,
    Toggle,
}

/// Options that are not listed in the YAML because they depend on the data.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DynamicOptions {
    /// The days of the event with activity, in its time zone, as
    /// `YYYY-MM-DD`. The default is the latest.
    EventDays,
}

/// A widget selector. Its value feeds a query parameter, and only the options
/// it lists are accepted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub label: String,

    /// Value to label, in display order. Exactly one of `options` and
    /// `options_from` is given.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub options: IndexMap<String, String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options_from: Option<DynamicOptions>,

    /// Required with listed options; with dynamic ones, absent means the
    /// latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    #[serde(default)]
    pub control: SelectorControl,

    /// Show the selector only while another one has one of these values. The
    /// other selector must be declared earlier, so a chain settles in one pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<Condition>,

    /// What each option stands for in a query, when that is not the option's
    /// own value: `voted_reg: [voted, registered]` for a ratio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maps: Option<IndexMap<String, serde_yaml::Value>>,
}

/// `when: {selector: grain, in: [hour]}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub selector: String,
    #[serde(rename = "in")]
    pub one_of: Vec<String>,
}

/// A query parameter: written literally, or taken from a selector.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Param<T> {
    Selector(SelectorRef),
    Literal(T),
}

/// Only a `{selector: name}` mapping is a reference. serde's derived untagged
/// form would also read a struct from a sequence, turning `[voted]` into a
/// reference to a selector named `voted`.
///
/// The value is read on its own, so an error inside it names where: the
/// outer path stops at the parameter.
impl<'de, T: serde::de::DeserializeOwned> Deserialize<'de> for Param<T> {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        use serde::de::Error;
        let raw = serde_yaml::Value::deserialize(deserializer)?;
        let within = |why: serde_path_to_error::Error<serde_yaml::Error>| {
            let at = why.path().to_string();
            if at == "." {
                D::Error::custom(why.into_inner())
            } else {
                D::Error::custom(format!("at {at}: {}", why.into_inner()))
            }
        };
        if raw.is_mapping() && raw.get("selector").is_some() {
            serde_path_to_error::deserialize(raw)
                .map(Param::Selector)
                .map_err(within)
        } else {
            serde_path_to_error::deserialize(raw)
                .map(Param::Literal)
                .map_err(within)
        }
    }
}

/// A parameter that can be left out, including by a selector option that
/// maps to `null`: `{top: 10, all: null}` offers "All" for a limit.
pub type OptionalParam<T> = Param<Option<T>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectorRef {
    pub selector: String,
}

/// `[numerator, denominator]`. A ratio is always computed from the two counts,
/// never averaged from other ratios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ratio(pub Measure, pub Measure);

impl Ratio {
    pub fn numerator(&self) -> Measure {
        self.0
    }

    pub fn denominator(&self) -> Measure {
        self.1
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

/// What to sort a result by. Named columns of the template's output rather
/// than free text, so a sort cannot name something the result lacks.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SortKey {
    /// The group or Post label.
    Label,
    /// The first measure.
    Value,
    /// The ratio.
    Ratio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sort {
    pub by: SortKey,
    #[serde(default)]
    pub order: SortOrder,
}

/// A template of the widget's data source plus its parameters. Any parameter
/// can take a selector's value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub template: QueryTemplate,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measures: Option<Param<Vec<Measure>>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratio: Option<Param<Ratio>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_by: Option<Param<String>>,

    /// Keep only these values of a dimension. A selector option mapping to
    /// `null` keeps them all.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub filters: IndexMap<String, OptionalParam<Vec<String>>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grain: Option<Param<TimeGrain>>,

    /// The day an hourly series covers, as `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<Param<String>>,

    /// Order of a group or Post list. Absent: the template's own order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<OptionalParam<Sort>>,

    /// Rows of a group or Post list to keep, after sorting. Unknown is
    /// always kept, after them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<OptionalParam<u32>>,

    /// How to show a measure or a Post state in the result:
    /// `{login_failures: Failed}`. Unlabelled ones use the platform's words.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub labels: IndexMap<String, String>,
}

/// Widgets on a 12-column grid, with the dashboard selectors they share.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dashboard {
    pub id: String,
    pub title: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<String>,

    /// Position in the dashboard switcher, lowest first.
    #[serde(default)]
    pub order: u32,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selectors: Vec<ScopeSelector>,

    /// The theme merged into every widget. Absent means [`DEFAULT_THEME`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    pub layout: Vec<LayoutItem>,
}

/// The theme a dashboard uses when it names none.
pub const DEFAULT_THEME: &str = "default";

/// The number of columns in a dashboard's grid.
pub const GRID_COLUMNS: u8 = 12;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutItem {
    pub widget: String,
    pub width: u8,

    /// This dashboard's defaults for the widget's selectors.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub values: IndexMap<String, String>,
}

/// dbt Charts style merged into every widget on a dashboard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// A dbt Charts built-in theme to start from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<BuiltinTheme>,

    /// dbt Charts board style: fonts, palette, category colours.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<serde_yaml::Value>,
}

/// The themes dbt Charts ships. A theme may only start from one of these:
/// dbt Charts would read any other name as a board file to inherit.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BuiltinTheme {
    Clarity,
    Neon,
    Paper,
    Stark,
    Vivid,
}

/// How this event's voters map onto dimensions, and the event's time zone.
///
/// Configuration rather than code because deployments store these facts in
/// different places: one keeps a voter's country as `Country/Embassy` in a
/// voter attribute, another in an area annotation. Counting stays in the data
/// source; this only says where to read a value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// IANA name. Buckets and export ranges are in this zone.
    pub time_zone: String,

    /// Where the region and country of a voter or Post are read. The Post is
    /// the election itself.
    pub scope: ScopeSettings,

    /// Which voters count as pre-enrolled.
    pub pre_enrolled: AttributeMatch,

    /// The voter dimensions widgets may group by, keyed by the name they use.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub dimensions: IndexMap<String, DimensionMapping>,

    /// How a missing value is shown, in the preset's language. Absent:
    /// "Unknown". Its key stays `__unknown__` whatever the label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unknown_label: Option<String>,
}

impl Settings {
    pub fn unknown_label(&self) -> &str {
        self.unknown_label
            .as_deref()
            .unwrap_or(super::payload::UNKNOWN_LABEL)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeSettings {
    pub region: DimensionMapping,
    pub country: DimensionMapping,
}

/// `{voter_attribute: <name>, equals: <value>}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeMatch {
    pub voter_attribute: String,
    pub equals: String,
}

/// Where one dimension's value is read, and how it is cleaned.
///
/// Exactly one of `voter_attribute`, `election_annotation` and
/// `area_annotation` is given; [`DimensionMapping::origin`] says which.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionMapping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voter_attribute: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub election_annotation: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_annotation: Option<String>,

    /// Keep one part of a compound value: `Spain/Madrid` → `Spain`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<Split>,

    /// Treat the value as a date of birth and count the voter in the band
    /// their age falls in on the event's first day.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub age_bands: Vec<AgeBand>,

    /// Display labels for raw values: `{M: Male, F: Female}`. Values with no
    /// label are shown as they are; missing values are shown as Unknown.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub labels: IndexMap<String, String>,
}

/// Where a [`DimensionMapping`] reads from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionOrigin<'a> {
    VoterAttribute(&'a str),
    ElectionAnnotation(&'a str),
    AreaAnnotation(&'a str),
}

impl DimensionMapping {
    /// `None` unless exactly one origin is given, which the policy reports.
    pub fn origin(&self) -> Option<DimensionOrigin<'_>> {
        match (
            &self.voter_attribute,
            &self.election_annotation,
            &self.area_annotation,
        ) {
            (Some(name), None, None) => {
                Some(DimensionOrigin::VoterAttribute(name))
            }
            (None, Some(key), None) => {
                Some(DimensionOrigin::ElectionAnnotation(key))
            }
            (None, None, Some(key)) => {
                Some(DimensionOrigin::AreaAnnotation(key))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Split {
    pub separator: String,
    pub part: usize,
}

/// A voter is in the first band whose `to` their age does not exceed; the
/// last band may leave `to` out to take everyone older.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgeBand {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<u32>,
}

/// Every live document of an event, for the checks that span documents: a
/// layout naming a widget that exists, a group-by naming a configured
/// dimension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfigSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
    #[serde(default)]
    pub themes: IndexMap<String, Theme>,
    #[serde(default)]
    pub widgets: IndexMap<String, Widget>,
    #[serde(default)]
    pub dashboards: IndexMap<String, Dashboard>,
}
