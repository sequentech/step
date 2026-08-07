// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a client's build of the wizard may say.
//!
//! A profile is how one customer's Election Architect differs from another's:
//! values they never choose, screens they never see, and fields they must fill
//! in. It is a build-time document, not a runtime permission — see below.
//!
//! # Paths, not top-level keys
//!
//! The TypeScript version could lock any top-level key of its config and nothing
//! deeper (`LockableConfigKey = Exclude<keyof ElectionConfig, 'elections'>`,
//! with `elections` carved out because ballot structure has to stay editable).
//!
//! That is not enough for what clients actually ask. `clients/smart-td.json`
//! locks `defaultCountingAlgorithm`, but what SMART TD wants is "every contest is
//! plurality-at-large" — and locking the event-wide default leaves every
//! per-contest override wide open, which defeats the lock entirely.
//!
//! So a profile speaks in paths, with `[]` for "every element of this list":
//!
//! ```text
//! trustee_threshold
//! schedule.voting_opens
//! elections[].contests[].overrides.tally.counting_algorithm
//! ```
//!
//! The range is deliberately tiny — literal segments and `[]`, nothing else. No
//! globs, because nobody can predict what one does; no indices, because a profile
//! that can say `elections[2]` breaks when somebody reorders their ballot.
//!
//! # A profile is not a security boundary
//!
//! `hidden` decides what the wizard draws. It is not access control, and anyone
//! who edits the saved plan can write whatever they like into a hidden field.
//! What makes a locked value stick is [`apply_profile`], which runs in Rust
//! before validation — so the locked value is the one that gets checked and the
//! one that gets built, whatever the document said.
//!
//! The TypeScript version guarded this twice, in `stripLockedUpdates` on write
//! and `reapplyLockedFields` on import, and both were bypassable by editing the
//! JSON. One enforcement point, on the path everything takes, is the whole idea.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::architect::Blueprint;
use super::problem::{Code, Problem, Report};

/// One step along a path into a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Field(String),
    /// `[]` — every element of the list this lands on.
    EveryElement,
}

/// A place in a plan a profile can speak about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanPath {
    text: String,
    segments: Vec<Segment>,
}

impl PlanPath {
    /// Read a path, refusing the shapes that would be a trap.
    pub fn parse(text: &str) -> Result<PlanPath, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("a path cannot be empty".to_string());
        }

        let mut segments = Vec::new();
        for part in trimmed.split('.') {
            let (name, brackets) = match part.find('[') {
                Some(at) => (&part[..at], &part[at..]),
                None => (part, ""),
            };

            if name.trim().is_empty() {
                return Err(format!(
                    "'{trimmed}' has an empty segment; paths look like \
                     elections[].contests[].max_votes"
                ));
            }
            segments.push(Segment::Field(name.trim().to_string()));

            match brackets {
                "" => {}
                "[]" => segments.push(Segment::EveryElement),
                other => return Err(only_every_element(trimmed, other)),
            }
        }

        Ok(PlanPath {
            text: trimmed.to_string(),
            segments,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Every place in `value` this path reaches, as a mutable reference.
    ///
    /// A path over a list that is empty reaches nothing, which is not an error —
    /// a profile locking every contest's counting algorithm is still correct for
    /// a plan with no contests yet.
    fn resolve<'a>(&self, value: &'a mut Value) -> Vec<&'a mut Value> {
        let mut here: Vec<&mut Value> = vec![value];

        for (index, segment) in self.segments.iter().enumerate() {
            let last = index == self.segments.len() - 1;
            let mut next: Vec<&mut Value> = Vec::new();

            for current in here {
                match segment {
                    Segment::Field(name) => {
                        let Some(object) = current.as_object_mut() else {
                            continue;
                        };
                        // The final field is created when absent, so a profile
                        // can supply a default the plan has not got. An
                        // intermediate one is not: inventing `schedule` to hold
                        // a `voting_opens` would build a shape the plan's own
                        // type does not have.
                        if last && !object.contains_key(name) {
                            object.insert(name.clone(), Value::Null);
                        }
                        if let Some(found) = object.get_mut(name) {
                            next.push(found);
                        }
                    }
                    Segment::EveryElement => {
                        if let Some(list) = current.as_array_mut() {
                            next.extend(list.iter_mut());
                        }
                    }
                }
            }
            here = next;
        }

        here
    }
}

/// Why a subscript other than `[]` is refused.
fn only_every_element(path: &str, subscript: &str) -> String {
    format!(
        "'{path}' uses '{subscript}'. Only '[]' — every element — is \
         understood: an index breaks as soon as somebody reorders their \
         ballot, and a pattern is something nobody can predict the effect of."
    )
}

impl fmt::Display for PlanPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// Build-time configuration of one client's wizard.
///
/// Paths are strings here because that is what JSON has; they are parsed and
/// checked by [`ClientProfile::read`], which is the only way to get one.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientProfile {
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Values a new plan starts with, and what a locked or hidden path is forced
    /// back to whatever the plan says.
    #[serde(default)]
    pub defaults: BTreeMap<String, Value>,

    /// Shown, but not editable.
    #[serde(default)]
    pub locked: Vec<String>,

    /// Not shown at all. Enforced exactly like `locked` — see the module docs on
    /// why this is not a permission.
    #[serde(default)]
    pub hidden: Vec<String>,

    /// Must be filled in. Additive: the baseline every build enforces is in
    /// [`validate_plan`](super::architect::validate_plan) and is not listed here.
    #[serde(default)]
    pub required: Vec<String>,
}

/// A profile with every path parsed and checked.
#[derive(Debug, Clone)]
pub struct Profile {
    pub id: String,
    pub display_name: Option<String>,

    /// What is odd about this profile without being wrong with it. Carried
    /// rather than discarded, so [`compile_plan`](super::architect::compile_plan)
    /// can fold it into the report somebody actually reads.
    pub warnings: Report,
    defaults: Vec<(PlanPath, Value)>,
    locked: Vec<PlanPath>,
    hidden: Vec<PlanPath>,
    required: Vec<PlanPath>,
}

impl Profile {
    /// Read a profile document, refusing one that cannot mean what it says.
    ///
    /// A path naming a field no plan has is refused here rather than ignored. A
    /// profile with a typo in it silently configures nothing, which is the
    /// failure nobody notices until a client asks why their build looks like
    /// everybody else's.
    pub fn read(document: &ClientProfile) -> Result<Profile, Report> {
        let mut report = Report::default();
        let shape = shape_of_a_plan();

        let mut parse_all = |paths: &[String], field: &str| -> Vec<PlanPath> {
            let mut parsed = Vec::new();
            for (index, text) in paths.iter().enumerate() {
                match PlanPath::parse(text) {
                    Ok(path) => {
                        if !reaches_anything(&path, &shape) {
                            report.push(Problem::error(
                                Code::DanglingReference,
                                format!("{field}[{index}]"),
                                format!(
                                    "'{path}' names nothing a plan has. A path \
                                     that reaches nothing configures nothing, \
                                     silently."
                                ),
                            ));
                        }
                        parsed.push(path);
                    }
                    Err(why) => report.push(Problem::error(
                        Code::InvalidValue,
                        format!("{field}[{index}]"),
                        why,
                    )),
                }
            }
            parsed
        };

        let locked = parse_all(&document.locked, "locked");
        let hidden = parse_all(&document.hidden, "hidden");
        let required = parse_all(&document.required, "required");

        let mut defaults = Vec::new();
        for (index, (text, value)) in document.defaults.iter().enumerate() {
            match PlanPath::parse(text) {
                Ok(path) => {
                    if !reaches_anything(&path, &shape) {
                        report.push(Problem::error(
                            Code::DanglingReference,
                            format!("defaults[{index}]"),
                            format!("'{path}' names nothing a plan has"),
                        ));
                    }
                    defaults.push((path, value.clone()));
                }
                Err(why) => report.push(Problem::error(
                    Code::InvalidValue,
                    format!("defaults[{index}]"),
                    why,
                )),
            }
        }

        if document.id.trim().is_empty() {
            report.push(Problem::error(
                Code::MissingField,
                "id",
                "a profile needs an id; it names the file and the build",
            ));
        }

        for path in locked.iter().chain(hidden.iter()) {
            if !defaults.iter().any(|(each, _)| each == path) {
                report.push(Problem::warning(
                    Code::MissingField,
                    "defaults",
                    format!(
                        "'{path}' is locked or hidden but has no default, so it \
                         is fixed at whatever the plan happens to say — which is \
                         nothing, for a new one"
                    ),
                ));
            }
        }

        if report.has_errors() {
            return Err(report);
        }

        Ok(Profile {
            id: document.id.clone(),
            display_name: document.display_name.clone(),
            warnings: report,
            defaults,
            locked,
            hidden,
            required,
        })
    }

    /// The paths the wizard should not draw, for the front end to act on.
    ///
    /// Rust decides *which paths*, because that is a statement about the plan.
    /// Which screens that empties is a question about screens, and belongs to
    /// whoever draws them.
    pub fn hidden_paths(&self) -> Vec<&str> {
        self.hidden.iter().map(PlanPath::as_str).collect()
    }

    pub fn locked_paths(&self) -> Vec<&str> {
        self.locked.iter().map(PlanPath::as_str).collect()
    }

    /// Whether a path is fixed by this profile, hidden or merely shown as read
    /// only. Both are enforced the same way; only the drawing differs.
    fn is_fixed(&self, path: &PlanPath) -> bool {
        self.locked.contains(path) || self.hidden.contains(path)
    }
}

/// Fill a plan's defaults and force what the profile fixes.
///
/// Two different things, and the difference is the whole of it:
///
/// * a **fixed** path — locked or hidden — is written unconditionally. That is
///   what makes the lock hold against a hand-edited plan.
/// * any other default is written only where the plan says nothing, so it seeds
///   a new plan without overwriting an answer somebody gave.
///
/// Works over the plan as JSON and deserializes back, so it needs no knowledge
/// of the plan's shape and cannot fall out of step with it.
pub fn apply_profile(
    plan: &Blueprint,
    profile: &Profile,
) -> Result<Blueprint, Report> {
    let mut document = serde_json::to_value(plan).map_err(|error| {
        one(Problem::error(
            Code::InvalidValue,
            "plan",
            format!(
                "this plan cannot be read as JSON, which is a bug: {error}"
            ),
        ))
    })?;

    for (path, value) in &profile.defaults {
        let fixed = profile.is_fixed(path);
        for target in path.resolve(&mut document) {
            if fixed || is_unset(target) {
                *target = value.clone();
            }
        }
    }

    serde_json::from_value(document).map_err(|error| {
        one(Problem::error(
            Code::InvalidValue,
            "profile.defaults",
            format!(
                "applying this profile produced something that is not a plan, so \
                 one of its defaults is the wrong shape for where it was put: \
                 {error}"
            ),
        ))
    })
}

/// The profile's own required fields, as problems on the paths that own them.
///
/// Deliberately not a boolean. Validation already returns a report whose paths
/// the wizard routes to the step that can fix each one, so a required field
/// lands on the right screen for free. The TypeScript version needed
/// `FIELD_WIZARD_STEP`, a hand-maintained map of thirty keys to steps, and
/// `isFieldFilled`, a thirty-arm switch — both only because its paths were flat.
pub fn check_required(
    plan: &Blueprint,
    profile: &Profile,
    report: &mut Report,
) {
    let Ok(mut document) = serde_json::to_value(plan) else {
        return;
    };

    for path in &profile.required {
        let targets = path.resolve(&mut document);
        // A path over an empty list reaches nothing. That is itself the thing
        // being complained about when the list is what was required.
        if targets.is_empty() || targets.iter().any(|target| is_unset(target)) {
            report.push(Problem::error(
                Code::MissingField,
                path.as_str(),
                "this build requires this to be filled in".to_string(),
            ));
        }
    }
}

/// Whether a value counts as "nobody has said".
///
/// Null, empty text, an empty list and an empty object all do. Zero and `false`
/// do not — they are answers, and a threshold of 0 being treated as unset is how
/// a default quietly overwrites a deliberate choice.
fn is_unset(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(list) => list.is_empty(),
        Value::Object(object) => object.is_empty(),
        _ => false,
    }
}

fn one(problem: Problem) -> Report {
    let mut report = Report::default();
    report.push(problem);
    report
}

/// A plan with one of everything, for checking that a path reaches something.
///
/// Serializing [`Blueprint::default`] would give empty lists, and a path through
/// `elections[]` would reach nothing and look like a typo. This has one element
/// in each list so the shape is walkable all the way down.
fn shape_of_a_plan() -> Value {
    let mut plan = Blueprint {
        version: super::architect::BLUEPRINT_VERSION,
        ..Default::default()
    };
    plan.contacts.push(Default::default());
    plan.trustees.push(Default::default());
    plan.areas.push(Default::default());
    plan.schedule.milestones.push(Default::default());

    let mut election = super::architect::PlannedElection::default();
    let mut contest = super::architect::PlannedContest::default();
    contest.candidates.push(Default::default());
    election.contests.push(contest);
    plan.elections.push(election);

    serde_json::to_value(&plan).unwrap_or(Value::Object(Map::new()))
}

/// Whether the path lands on something in a plan-shaped document.
fn reaches_anything(path: &PlanPath, shape: &Value) -> bool {
    let mut copy = shape.clone();
    // `resolve` creates a missing final field, so asking it directly would
    // always say yes. Walk the parent instead and look for the name.
    match path.segments.split_last() {
        Some((Segment::Field(name), parents)) => {
            let parent = PlanPath {
                text: String::new(),
                segments: parents.to_vec(),
            };
            if parents.is_empty() {
                return copy
                    .as_object()
                    .is_some_and(|object| object.contains_key(name));
            }
            parent.resolve(&mut copy).into_iter().any(|value| {
                value
                    .as_object()
                    .is_some_and(|object| object.contains_key(name))
            })
        }
        // Ends in `[]`: the list itself has to exist.
        _ => !path.resolve(&mut copy).is_empty(),
    }
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod profile_tests;
