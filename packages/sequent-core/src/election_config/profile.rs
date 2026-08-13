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

    /// Named sets of ballot rules this client is offered, instead of ours.
    ///
    /// The wizard offers three — permissive, standard, strict — which are a
    /// sensible spread for elections in general and frequently not the spread
    /// for *one organisation*. A client whose rules describe two ways they run
    /// a ballot is better served by those two, named after their own rules,
    /// than by three they have to translate.
    ///
    /// Additive to the built-in three rather than replacing them, unless
    /// [`Self::only_our_presets`] says otherwise — a profile author fixing one
    /// client's vocabulary should not have to re-describe the general case to
    /// keep it.
    ///
    /// Each value is a partial set: anything it omits takes the platform's
    /// default, exactly like the built-in presets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<NamedPreset>,

    /// Offer only the profile's own presets, not ours as well.
    ///
    /// For a client whose rules genuinely admit two possibilities and no
    /// others, where showing a third invites choosing it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub only_our_presets: bool,

    /// How voters may prove who they are, and the Keycloak configuration behind
    /// each way.
    ///
    /// **The list a wizard offers comes from here and nowhere else.** A client's
    /// identity provider is theirs, so the four Sequent ships are a *default*
    /// rather than a ceiling — they are what
    /// [`DEFAULT_PROFILE_JSON`] carries, and a profile naming its own replaces
    /// them entirely rather than adding to them.
    ///
    /// Replacing rather than adding, unlike [`Self::presets`], and the difference
    /// is worth stating: a ballot-rule set a client does not choose is a button
    /// nobody presses, while a *sign-in flow* a client's realm cannot provision
    /// is an election nobody can log into, discovered on the morning voting
    /// opens. Offering ours alongside theirs would be offering exactly that.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auth_presets: Vec<super::preset_doc::AuthPresetDoc>,
}

/// The profile a build with no `?profile=` uses.
///
/// In git, and `include_str!`'d rather than transcribed into Rust, so the
/// Keycloak configuration exists exactly once. It is generated from
/// [`super::presets::PRESETS`] and pinned by
/// `default_profile_json_matches_the_shipped_presets` — nobody can read a
/// two-hundred-line realm patch and be sure a transcription is faithful, so
/// nobody is asked to.
pub const DEFAULT_PROFILE_JSON: &str =
    include_str!("presets/default_profile.json");

/// The shipped profile, parsed.
///
/// Parsed on each call rather than held in a `OnceLock`: this is read once when a
/// wizard starts, `include_str!` means there is no IO, and a global would be a
/// second place for a mutated copy to hide.
pub fn default_profile() -> Result<ClientProfile, serde_json::Error> {
    serde_json::from_str(DEFAULT_PROFILE_JSON)
}

/// A named set of ballot rules a profile offers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NamedPreset {
    /// What the button says. The client's own word for it, not ours.
    pub name: String,

    /// One line on when to pick it, shown under the buttons.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub about: String,

    /// `policy field -> value`, as `policyCatalog()` spells them. Validated
    /// against the real value space, so a profile cannot invent a behaviour.
    #[serde(default)]
    pub values: BTreeMap<String, String>,
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
    /// The ballot-rule sets this client is offered, and whether ours go too.
    pub presets: Vec<NamedPreset>,
    pub only_our_presets: bool,
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
        for (text, value) in document.defaults.iter() {
            match PlanPath::parse(text) {
                Ok(path) => {
                    if !reaches_anything(&path, &shape) {
                        report.push(Problem::error(
                            Code::DanglingReference,
                            format!("defaults.{text}"),
                            format!("'{path}' names nothing a plan has"),
                        ));
                    }
                    defaults.push((path, value.clone()));
                }
                Err(why) => report.push(Problem::error(
                    Code::InvalidValue,
                    format!("defaults.{text}"),
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

        // Locked only, and the asymmetry is the point.
        //
        // An error, not a warning. `apply_profile` writes only what `defaults`
        // names, so a lock with nothing to lock *to* fixes the field at whatever
        // the plan happens to say — which for a new plan is nothing. A client is
        // shown that value, greyed out, above a caption saying their
        // organisation set it. There is no case where that is useful.
        //
        // **Hidden is not the same question, and requiring a value there was a
        // mistake.** Nothing is drawn, so there is nothing to be wrong about on
        // screen, and this module's own opening says `hidden` is not access
        // control — a saved plan can carry anything in a field the wizard never
        // showed. Demanding a default in the name of enforcement claimed a
        // guarantee the design explicitly disclaims.
        //
        // What it cost: hiding a whole screen is the commonest thing a delivery
        // profile does, and the builder writes the step's *prefixes* to do it —
        // `schedule`, `messages`, `voting_channels` — none of which is a
        // settings row, so none of them had a default to seed. Every such
        // profile was refused, and the only symptom was a wizard that would not
        // start.
        //
        // A hidden path with a value still fixes it: this is about what a
        // profile must say, not about what it may.
        for path in locked.iter() {
            if !defaults.iter().any(|(each, _)| each == path) {
                report.push(Problem::error(
                    Code::MissingField,
                    "defaults",
                    format!(
                        "'{path}' is locked but has no default, so nothing \
                         would be enforced. Give it a value."
                    ),
                ));
            }
        }

        // A preset may only name real policies, with real values. This is the
        // whole reason presets live in Rust rather than being a list of strings
        // the browser renders: the wizard offers exactly what the core hands
        // over, so a client-facing button cannot select a behaviour no importer
        // accepts. Two earlier versions of this tool shipped three such values.
        for (index, preset) in document.presets.iter().enumerate() {
            if preset.name.trim().is_empty() {
                report.push(Problem::error(
                    Code::MissingField,
                    format!("presets[{index}].name"),
                    "a preset needs a name; it is what the button says",
                ));
            }
            for (field, value) in preset.values.iter() {
                match super::policy::Behaviour::default().accepts(field, value)
                {
                    Ok(()) => {}
                    Err(why) => report.push(Problem::error(
                        Code::InvalidValue,
                        format!("presets[{index}].values.{field}"),
                        why,
                    )),
                }
            }
        }

        if document.only_our_presets && document.presets.is_empty() {
            report.push(Problem::error(
                Code::InvalidValue,
                "only_our_presets",
                "this hides our presets and offers none of its own, so the \
                 client would have no set to choose",
            ));
        }

        if report.has_errors() {
            return Err(report);
        }

        Ok(Profile {
            id: document.id.clone(),
            display_name: document.display_name.clone(),
            warnings: report,
            presets: document.presets.clone(),
            only_our_presets: document.only_our_presets,
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

    /// The paths this build additionally insists on.
    ///
    /// Enforcing them is [`check_required`]'s job, in Rust. This is so a form
    /// can mark them — a required field with no asterisk is a form somebody
    /// fills in twice.
    pub fn required_paths(&self) -> Vec<&str> {
        self.required.iter().map(PlanPath::as_str).collect()
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
    use super::policy::{LayoutPatch, Overrides, PolicyPatch, TallyPatch};

    let mut plan = Blueprint {
        version: super::architect::BLUEPRINT_VERSION,
        ..Default::default()
    };
    plan.contacts.push(Default::default());
    // `messages` carries `skip_serializing_if`, so an empty plan has no such
    // key and `hidden: ["messages"]` — which is how the Voter Messaging screen
    // is dropped — read as a typo. Same trap as `Overrides` and `shared` below.
    plan.messages.push(Default::default());
    plan.trustees.push(Default::default());
    plan.areas.push(Default::default());
    plan.schedule.milestones.push(Default::default());
    // Every moment, filled in. `Option` serialises as `null`, which has no keys
    // to walk into, and `zone` is skipped while empty — so a shape built from
    // defaults makes `schedule.voting_opens.zone` look like a typo.
    let moment =
        Some(super::time::Timestamp::new("2027-01-01T00:00", "UTC", 0));
    plan.schedule.key_ceremony = moment.clone();
    plan.schedule.voting_opens = moment.clone();
    plan.schedule.voting_closes = moment.clone();
    plan.schedule.tally_ceremony = moment;
    // The same trap once more. `auth_preset` is an `Option` that skips
    // serialising while empty, so a shape built from defaults has no such key
    // and every profile naming *How voters sign in* was refused —
    // `'auth_preset' names nothing a plan has` — even though the plan has it and
    // the wizard offers it. A profile downloaded from the client profile builder
    // with that setting touched could not be loaded at all.
    plan.auth_preset = Some(String::new());

    // Filled rather than defaulted, because `Overrides` and `Option<Overrides>`
    // both carry `skip_serializing_if`. Left empty they vanish from the shape,
    // and every path through them — including
    // `elections[].contests[].overrides.tally.counting_algorithm`, the example
    // this whole design exists for — is refused as naming nothing.
    let filled = Overrides {
        policies: PolicyPatch {
            over_vote: Some(Default::default()),
            blank_vote: Some(Default::default()),
            under_vote: Some(Default::default()),
            invalid_vote: Some(Default::default()),
            duplicated_rank: Some(Default::default()),
            preference_gaps: Some(Default::default()),
            candidates_order: Some(Default::default()),
        },
        tally: TallyPatch {
            voting_type: Some(String::new()),
            counting_algorithm: Some(String::new()),
            min_votes: Some(0),
            is_encrypted: Some(true),
            tie_breaking_policy: Some(String::new()),
        },
        layout: LayoutPatch {
            columns: Some(1),
            collapsible_lists: Some(String::new()),
            enable_checkable_lists: Some(String::new()),
            max_selections_per_type: Some(0),
        },
    };

    let mut contest = super::architect::PlannedContest {
        overrides: filled.clone(),
        ..Default::default()
    };
    contest.candidates.push(Default::default());

    let mut election = super::architect::PlannedElection {
        shared: Some(filled),
        ..Default::default()
    };
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
