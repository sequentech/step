// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Election Architect's plan, and how it becomes a bundle.
//!
//! The architect is a wizard: somebody answers questions and gets an importable
//! election event. This module is the half of it that decides what the answers
//! mean. The other half — what the questions look like — is React, in
//! `beyond/packages/election-architect`, and it contains no mapping, no CSV and no
//! zip.
//!
//! # Why this is so small
//!
//! A [`Blueprint`] does not become a bundle here. It becomes a
//! [`Workbook`] — the same rows the spreadsheet reader produces — and the existing
//! [`super::build`] takes it from there.
//!
//! That is the whole design. A wizard is not a different kind of election event;
//! it is a different way of filling in the same fields. Going through the workbook
//! shape means the architect inherits the entity templates, the deterministic ids,
//! the CSV byte shapes, the Keycloak realm handling, the archive layout and every
//! validation rule, for free and without a second copy of any of them. What is
//! left here is only what is genuinely the architect's own: its plan, the checks
//! that apply to a plan rather than to a bundle, and the three files it produces
//! that are not part of an import at all.
//!
//! # What the TypeScript version got wrong, and why
//!
//! Its output was not the importable format: `election_config.json` inside a
//! nested `official_election_setup.zip`, where the importer looks for
//! `export_election_event-<uuid>.json` at the archive root. Its scheduled-events
//! CSV was built by string interpolation, one of three hand-written copies of that
//! byte shape. It stamped `new Date()` into every entity, so no two runs of the
//! same answers produced the same file. It embedded a Keycloak realm copied from
//! one environment, which the importer takes wholesale and would have used to
//! replace whatever the target environment had provisioned. And it validated
//! nothing.
//!
//! None of those are mistakes anybody made twice. They are what happens when a
//! format is implemented a second time, which is why this implementation does not.

use std::cmp::Ordering;

use crate::election_config::archive::{Artifact, Layout};
use crate::election_config::build::{
    build, BuildOptions, Bundle, ImageFile, MaterialFile,
};
use crate::election_config::ids::IdFactory;
use crate::election_config::paths::Cell;
use crate::election_config::policy::{Behaviour, Overrides};
use crate::election_config::problem::{Code, Problem, Report, Severity};
use crate::election_config::profile::{apply_profile, check_required, Profile};
use crate::election_config::render::TemplateSet;
use crate::election_config::schema::ImportElectionEventSchema;
use crate::election_config::sheet::{Row, Sheet, Workbook, SHEET_PARAMETERS};
use crate::election_config::time::{self, Timestamp};
use crate::election_config::validate::{ALLOW_EARLY_VOTING, NO_EARLY_VOTING};
use crate::types::ceremonies::CeremoniesPolicy;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The plan format's version.
///
/// Written into every saved plan and checked on load. A plan is a document
/// somebody spent an afternoon on; being able to say "this is from an older
/// version" beats failing to deserialize it with a serde error about a missing
/// field.
pub const BLUEPRINT_VERSION: u32 = 3;

/// Bring a version 1 plan up to date, in place.
///
/// Version 1 carried `policies`, a three-value `allowed | warn | restricted`
/// per policy, applied to every contest identically. Version 2 carries the
/// platform's own values, per contest.
///
/// The mapping below is **version 1's own**, reproduced exactly — including
/// where it was wrong. `restricted` for an under-vote produced
/// `warn-only-in-review` and `restricted` for a blank or invalid vote produced
/// `not-allowed`, and a plan that compiled to those bytes yesterday has to
/// compile to them today. Getting it *right* for an old plan would silently
/// change an election somebody has already reviewed; new plans get the
/// considered defaults instead.
fn migrate_v1(document: &mut serde_json::Value) {
    let Some(object) = document.as_object_mut() else {
        return;
    };
    if object.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
        return;
    }

    let old = object.remove("policies").unwrap_or(serde_json::Value::Null);
    let says = |key: &str| -> &str {
        old.get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("warn")
    };

    let mut policies = serde_json::Map::new();
    policies.insert(
        "over_vote".to_string(),
        serde_json::json!(match says("over_vote") {
            "allowed" => "allowed",
            "restricted" => "not-allowed-with-msg-and-disable",
            _ => "allowed-with-msg",
        }),
    );
    policies.insert(
        "under_vote".to_string(),
        serde_json::json!(match says("under_vote") {
            "allowed" => "allowed",
            "restricted" => "warn-only-in-review",
            _ => "warn",
        }),
    );
    for (key, restricted) in [
        ("blank_vote", "not-allowed"),
        ("invalid_vote", "not-allowed"),
    ] {
        policies.insert(
            key.to_string(),
            serde_json::json!(match says(key) {
                "allowed" => "allowed",
                "restricted" => restricted,
                _ => "warn",
            }),
        );
    }

    object.insert(
        "defaults".to_string(),
        serde_json::json!({"policies": policies}),
    );
    object.insert("version".to_string(), serde_json::json!(2));
}

/// Bring a version 2 plan up to date, in place.
///
/// Version 2 keyed a voter's area by the area's **name**; version 3 keys it by
/// `external_id`. Every saved plan carries names, so without this every voter in
/// every existing plan would open with an area nothing recognises — the census
/// would look intact and the build would refuse each row.
///
/// A name that matches no area is **kept as written** rather than dropped. It is
/// already broken, and moving it into the identifier field means validation says
/// "no area has external_id 'North Local 4'" against the row it is on, which is
/// visible and fixable. Dropping it would turn a loud problem into a voter who
/// quietly has no ballot.
///
/// Ambiguity cannot arise here: version 2 refused a plan whose areas shared a
/// name, so at most one area answers to any of these.
fn migrate_v2(document: &mut serde_json::Value) {
    let Some(object) = document.as_object_mut() else {
        return;
    };
    if object.get("version").and_then(serde_json::Value::as_u64) != Some(2) {
        return;
    }

    let ids: Vec<(String, String)> = object
        .get("areas")
        .and_then(serde_json::Value::as_array)
        .map(|areas| {
            areas
                .iter()
                .filter_map(|area| {
                    let text = |key: &str| {
                        area.get(key)
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string()
                    };
                    let name = text("name");
                    (!name.is_empty()).then(|| (name, text("external_id")))
                })
                .collect()
        })
        .unwrap_or_default();

    if let Some(voters) = object
        .get_mut("voters")
        .and_then(serde_json::Value::as_array_mut)
    {
        for voter in voters {
            let Some(row) = voter.as_object_mut() else {
                continue;
            };
            let Some(was) = row.remove("area_name") else {
                continue;
            };
            let name = was.as_str().unwrap_or_default().trim().to_string();
            if name.is_empty() {
                continue;
            }
            let resolved = ids
                .iter()
                .find(|(each, _)| each.trim() == name)
                .map(|(_, id)| id.clone())
                .unwrap_or(name);
            row.insert(
                "area_external_id".to_string(),
                serde_json::json!(resolved),
            );
        }
    }

    object.insert("version".to_string(), serde_json::json!(3));
}

/// Read a saved plan, bringing an older one up to date.
///
/// The only way a plan should be deserialized. A plan from a *newer* version is
/// left alone here and refused by [`validate_plan`], which can say so as a
/// problem rather than as a serde error about a field nobody has heard of.
pub fn read_plan(document: &str) -> Result<Blueprint, Problem> {
    let mut value: serde_json::Value =
        serde_json::from_str(document).map_err(|error| {
            Problem::error(
                Code::InvalidValue,
                "plan",
                format!("this is not an election plan: {error}"),
            )
            .id("plan.not-a-plan")
            .detail("error", error)
        })?;

    migrate_v1(&mut value);
    migrate_v2(&mut value);

    serde_json::from_value(value).map_err(|error| {
        Problem::error(
            Code::InvalidValue,
            "plan",
            format!("this plan cannot be read: {error}"),
        )
        .id("plan.unreadable")
        .detail("error", error)
    })
}

/// The ways of voting an event opens.
///
/// Four booleans, and each one is a *precondition* rather than a switch: turning
/// one on makes the platform offer that channel, and something else has to exist
/// before a voter can use it. That asymmetry is the whole reason this type is
/// validated rather than just written.
///
/// | Channel | What turning it on actually does |
/// | --- | --- |
/// | `online` | The Publish screen gets an Online start/stop control, and the Voting Portal's ordinary path opens. This is web voting; with it off nobody votes. |
/// | `kiosk` | The Publish screen gets a Kiosk control. Voters reach it through a `?kiosk` URL, which the portal answers with a **separate** auth client named `<client>-kiosk` — see `AuthContextProvider`. The bundle cannot create that client. |
/// | `early_voting` | The Publish screen gets an Early voting control, **and** the Admin Portal will let an area be marked early-voting — `Area/FormContent` gates that field on this exact flag. Both halves are needed. |
/// | `telephone` | The Publish screen gets a Telephone control and the event grows an **IVR tab** in the Admin Portal (`ElectionEventTabs`). The telephone flow itself is configured on that tab, and no part of it is in the bundle. |
///
/// There is a fifth field, `paper`, in `hasura_core::VotingChannels`. It is
/// deliberately **not** here: nothing reads it. It has no `VotingStatusChannel`
/// variant, no status block, no Publish control, no entry in the Admin Portal's own
/// `defaultChannels`, and no label. Writing it would put a checkbox on screen that
/// changes nothing, which is worse than the field being absent — somebody would
/// tick it and believe they had arranged something.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VotingChannelSet {
    /// Ordinary web voting. On unless somebody deliberately turns it off.
    #[serde(default = "yes")]
    pub online: bool,

    /// A shared machine at a polling place.
    #[serde(default)]
    pub kiosk: bool,

    /// Voting by telephone, configured afterwards on the event's IVR tab.
    #[serde(default)]
    pub telephone: bool,

    /// A period before the main one, for the areas that allow it.
    #[serde(default)]
    pub early_voting: bool,
}

impl Default for VotingChannelSet {
    /// Online and nothing else — what `election_event.hbs` has always emitted.
    ///
    /// Hand-written rather than derived because `bool`'s `Default` is `false` and
    /// deriving would give an event nobody can vote in. `#[serde(default = "yes")]`
    /// only applies when *deserialising*, so it would not have covered this.
    fn default() -> Self {
        Self {
            online: true,
            kiosk: false,
            telephone: false,
            early_voting: false,
        }
    }
}

impl VotingChannelSet {
    /// Whether any way of voting is open at all.
    pub fn any(&self) -> bool {
        self.online || self.kiosk || self.telephone || self.early_voting
    }

    /// The columns and cells for a sheet that carries the whole set.
    ///
    /// Both the event and every election get them, from this one method, because
    /// the Publish screen reads `voting_channels` off whichever record it is
    /// showing — `useRecordContext<ElectionEvent | Election>` — so an event that
    /// allows kiosk and an election that does not is an event whose Kiosk button
    /// is present at one level and missing at the other.
    pub fn columns_for_sheet(&self) -> Vec<(&'static str, Cell)> {
        vec![
            ("voting_channels.online", Cell::Bool(self.online)),
            ("voting_channels.kiosk", Cell::Bool(self.kiosk)),
            ("voting_channels.telephone", Cell::Bool(self.telephone)),
            (
                "voting_channels.early_voting",
                Cell::Bool(self.early_voting),
            ),
        ]
    }
}

/// What the wizard collected.
///
/// This is the artifact worth keeping. The bundle is derived from it and is
/// disposable; the plan is what somebody edits next month when a candidate
/// withdraws.
///
/// The TypeScript version had no such thing — it reconstructed the wizard's state
/// by parsing its own generated bundle back in. That loses every answer the bundle
/// has no field for (the trustee threshold, the ceremony dates, the points of
/// contact) and breaks whenever the bundle's shape changes. Saving the plan is both
/// simpler and lossless.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Blueprint {
    /// [`BLUEPRINT_VERSION`] at the time it was saved.
    pub version: u32,

    /// Stable identifier for the event, and the seed for every generated id.
    ///
    /// Not shown to voters. Two plans with the same one produce the same
    /// identifiers, which is what makes regenerating diffable.
    pub external_id: String,

    /// The event's name, per language. `en` is the fallback.
    #[serde(default)]
    pub name: Translated,

    /// Whether a voter can look up the record of a ballot they cast.
    ///
    /// `show-logs-tab` or `hide-logs-tab`, per `EShowCastVoteLogsPolicy`. The
    /// Voting Portal's ballot-locator page reads it and shows a tab where somebody
    /// with a ballot identifier can see that it was received.
    ///
    /// A real decision about how much a voter can verify for themselves, exposed in
    /// the Admin Portal's event form and unsettable from a plan.
    #[serde(default = "show_logs")]
    pub show_cast_vote_logs: String,

    /// The order the elections appear in, for a voter with more than one.
    ///
    /// The same three values again. `custom` is the order on the Ballot screen.
    #[serde(default = "custom_order")]
    pub elections_order: String,

    /// Which ways of voting this event opens.
    ///
    /// On the event, and written to every election as well — see
    /// [`VotingChannelSet::columns_for_sheet`] for why both.
    #[serde(default)]
    pub voting_channels: VotingChannelSet,

    /// What the whole event is, for voters.
    ///
    /// Translatable, like the name: the platform keeps it at
    /// `presentation.i18n.<lang>.description` and mirrors the English one into a
    /// flat `description` column, which is what the Admin Portal's list views
    /// read. Both are written.
    #[serde(default)]
    pub description: Translated,

    /// BCP 47 or ISO 639-2/T codes, in the order the picker should show them.
    #[serde(default)]
    pub languages: Vec<String>,

    /// Which of [`Self::languages`] a voter gets before choosing.
    ///
    /// `None` means the first one, which is what this used to be unconditionally
    /// — `event_sheet` wrote `languages.first()` and nothing could say otherwise.
    /// So the languages were configurable and the default was not, and a client
    /// wanting Spanish first had to reorder the list without being told that was
    /// what the order meant.
    ///
    /// It is the default *ballot* language, not the wizard's own. Those are
    /// different things and the guide says so: this one is what voters read.
    #[serde(default)]
    pub default_language: Option<String>,

    /// Whether a voter's own browser decides which language they start in.
    ///
    /// `browser-detect` reads the browser's preference; `force-default` ignores it
    /// and uses [`Self::default_language`]. `None` leaves the platform's own
    /// default, which is `browser-detect`.
    ///
    /// It is the other half of the language question and was unreachable: a client
    /// could say which languages the ballot offers and which one is the default,
    /// and then the default only applied to browsers that asked for nothing else.
    #[serde(default)]
    pub language_detection_policy: Option<String>,

    /// A link to the client's logo, for a plan that already has one.
    ///
    /// Kept, and no longer what the wizard collects: [`Self::logo`] is the file, and
    /// a url is a promise about a server nothing here can check. A plan written
    /// before the upload existed still carries its link and still builds to the same
    /// bytes, which is the same rule `migrate_v1` follows — an approved election does
    /// not change because the tool improved.
    #[serde(default)]
    pub logo_url: Option<String>,

    /// The client's logo as a file, which is what the wizard collects now.
    ///
    /// The same [`CandidateImage`] a candidate's photograph uses, and the same
    /// journey: `images/document_<id>_<file>` in the archive, `is_public: true` on
    /// the way in, and a bucket-relative url composed by the builder — which is the
    /// only place that knows the tenant. The document identifier is derived from the
    /// event's own `external_id` by [`logo_document_id`], so nothing stores it and
    /// the url and the archive entry cannot drift apart.
    ///
    /// Both this and [`Self::logo_url`] set is not an error, and the file wins:
    /// `check_logo` says so rather than leaving a plan whose two answers disagree.
    #[serde(default)]
    pub logo: Option<CandidateImage>,

    /// Send a voter straight to the ballot instead of a list of elections.
    ///
    /// Worth having where an event holds one election, which is most of them: the
    /// list is then a screen with one thing on it between a voter and voting.
    #[serde(default)]
    pub skip_election_list: Option<bool>,

    /// Whether a voter can open their own profile from the voting portal.
    #[serde(default)]
    pub show_user_profile: Option<bool>,

    /// Whether the voting portal shows a support-materials tab at all.
    ///
    /// The tab, not its contents: [`Self::materials`] carries the documents, and they
    /// do travel in the bundle. This comment used to say they could not — reasoning
    /// from `ElectionEventMaterials` holding one boolean and `ImportElectionEventSchema`
    /// having no documents array, and never asking what the importer does with zip
    /// entries. `EA-119` established that it does, and built the round trip.
    #[serde(default)]
    pub materials_activated: Option<bool>,

    /// The heading over the support materials tab, per language.
    #[serde(default)]
    pub materials_title: Translated,

    /// The line under that heading, per language.
    #[serde(default)]
    pub materials_subtitle: Translated,

    /// The documents themselves — rules, statements, a guide to voting.
    ///
    /// These *do* travel in the bundle. See
    /// `engineering/how-a-support-material-travels-in-a-bundle`: the rows go into
    /// `support_materials` and each file into `export_S3_files/`, which is the
    /// private counterpart of the `images/` branch a candidate photograph uses.
    #[serde(default)]
    pub materials: Vec<PlannedMaterial>,

    #[serde(default)]
    pub contacts: Vec<Contact>,

    #[serde(default)]
    pub trustees: Vec<Trustee>,

    /// How many trustees must take part to open the tally.
    #[serde(default = "default_threshold")]
    pub trustee_threshold: u32,

    /// Whether the key ceremony is run by people or by the platform.
    ///
    /// The platform's own [`CeremoniesPolicy`], carried into the ceremony's
    /// `settings` where `KeysCeremony::policy()` reads it. Absent means
    /// `manual-ceremonies`, which is the platform's default and what every bundle
    /// this tool has built so far has silently been.
    ///
    /// **Not [`crate::ballot::KeysCeremonyPolicy`]**, whose name is one word away
    /// and means something else entirely: that one is `ELECTION_EVENT` or
    /// `ELECTION` — how many ceremonies there are — while this is who runs them.
    #[serde(default)]
    pub ceremony_policy: CeremoniesPolicy,

    #[serde(default)]
    pub schedule: Schedule,

    /// The areas voters belong to.
    ///
    /// Empty means one ballot for everybody, and one is synthesised. See
    /// [`DEFAULT_AREA_EXTERNAL_ID`].
    #[serde(default)]
    pub areas: Vec<PlannedArea>,

    #[serde(default)]
    pub elections: Vec<PlannedElection>,

    /// The voters, when the plan carries them.
    ///
    /// Optional on purpose. A plan describes an election and a census is a
    /// separate, much larger, much more sensitive thing — so most plans have
    /// none and the wizard says so. But a client whose membership does not
    /// change between elections has every reason to keep the two together, and
    /// telling them to hold the list somewhere else is telling them to hold it
    /// somewhere worse.
    ///
    /// The columns are `build_tables::VOTER_LEADING_COLUMNS` plus whatever else
    /// the client carries; anything unrecognised becomes a Keycloak user
    /// attribute, which is how a reporting breakout arrives without a code
    /// change. That is the same passthrough the workbook path has, so a census
    /// exported from one route imports through the other.
    #[serde(default)]
    pub voters: Vec<PlannedVoter>,

    /// The messages voters are sent, and when somebody should send them.
    ///
    /// Empty is the ordinary case and means the client sends nothing through us.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<PlannedMessage>,

    /// Which of the four authentication flows this event uses.
    ///
    /// `None` leaves the environment's own configuration alone, which is the
    /// safe default and what every plan written before this field did.
    ///
    /// Carried in the plan rather than only in `CompileOptions` because a
    /// choice that vanishes when you reopen a plan is a choice somebody makes
    /// twice and gets differently the second time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_preset: Option<String>,

    /// How contests behave unless they say otherwise.
    ///
    /// Was the only place policies could be set, and applied to every contest
    /// identically; now the bottom of a three-level resolution.
    #[serde(default)]
    pub defaults: Behaviour,

    /// A stylesheet for the voting portal, applied to what a voter sees.
    ///
    /// A raw string, because that is what the platform stores and applies:
    /// `election_event.presentation.css`, interpolated into a styled component
    /// by the portal's own shell. There is no token set to model — no primary
    /// colour, no branding object — anywhere in the platform, and inventing one
    /// here would be a second opinion the importer does not share.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub css: String,

    /// Wording overrides for the voting portal, per language.
    ///
    /// `{language: {dotted.key: text}}` — the shape
    /// `presentation.i18n` already carries and `overwriteTranslations` already
    /// applies, splitting each key on `.` and deep-merging over the shipped
    /// catalogue. Flat keys rather than a nested map because that is what the
    /// Admin Portal's own editor writes, and the two have to be the same file.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub i18n: BTreeMap<String, BTreeMap<String, String>>,

    /// Wording overrides for the **sign-in page**, per language.
    ///
    /// Same shape as [`Blueprint::i18n`] and a different destination, because they
    /// are different programs. `i18n` reaches everything that reads the event's
    /// presentation — the Voting Portal, and the ballot verifier with it — through
    /// `overwriteTranslations`. The sign-in page is Keycloak: a Java application
    /// that never sees the event's presentation and looks its wording up in the
    /// realm's own `localizationTexts`.
    ///
    /// So these are emitted as `keycloak_event_realm.localizationTexts.<locale>.<key>`
    /// parameters, which is the road `login_custom_css` already travels
    /// (`branding::login_css_patch`) and the prefix `PARAMETER_PREFIXES` already
    /// carries into the realm patch. Nothing in the realm builder had to change:
    /// the patch merges by path, so a plan's wording lands beside whatever a base
    /// export brought rather than replacing the object it lives in.
    ///
    /// **Not MessageFormat-escaped**, unlike the CSS beside it. `login_css_patch`
    /// escapes because a stylesheet is full of braces and Keycloak reads a message
    /// as a MessageFormat pattern; a *translation* may legitimately carry `{0}`,
    /// and escaping it would put the literal characters on the page where the
    /// voter's name should be. A brace that is not a well-formed placeholder is
    /// worth a warning rather than a silent rewrite — see `check_keycloak_messages`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub keycloak_messages: BTreeMap<String, BTreeMap<String, String>>,

    /// Anything the wizard has no field for. Carried, not interpreted.
    #[serde(default)]
    pub notes: String,

    /// Sheets this wizard has no screens for, exactly as they arrived.
    ///
    /// Parameters, Admin Users, Permissions, Templates and Reports describe the
    /// *platform* rather than the ballot: a tenant, an authentication type, realm
    /// patches, administrator accounts, the permission matrix, communication
    /// templates, scheduled reports. The wizard asks about none of it, and a
    /// delivery engineer who opens a real janitor workbook here would otherwise
    /// lose all of it the moment they rebuilt.
    ///
    /// So it is carried verbatim and handed straight back to [`to_workbook`],
    /// where `build` reads it exactly as it reads a janitor's own file. **That is
    /// the whole implementation**: `build` already turns a Parameters row into an
    /// `election_event.*` patch and a realm patch, and already turns the other four
    /// into `admin_users.csv`, `export_permissions-<tenant>.csv`, `templates/*.hbs`
    /// and the reports table. Interpreting any of it a second time here would be a
    /// second answer to a question that already has one.
    ///
    /// Empty for a plan the wizard built from nothing, which is why it is skipped
    /// when serialising: an existing `blueprint.json` is unchanged by this field
    /// existing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platform: Vec<Sheet>,
}

fn default_threshold() -> u32 {
    2
}

/// One voter, as a plan carries them.
///
/// Deliberately close to a row of `export_voters.csv` rather than to a domain
/// model: this is a census on its way through, and every field it does not
/// recognise it keeps. Reshaping it into something tidier would mean deciding
/// which of a client's own columns matter, which is not a decision this has any
/// basis for making.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedVoter {
    /// What they sign in as. The only field that must be there.
    pub username: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub email: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub first_name: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_name: String,

    /// Which area's ballot they get, by the area's `external_id`.
    ///
    /// **By identifier, not by name**, and the change of key is the point. A name
    /// is a label somebody edits: renaming an area used to silently strip every
    /// voter in it of a ballot, and two areas could not share a display name
    /// because the name *was* the key — which is why `area.duplicate-name` had to
    /// exist. A parent-scoped org chart with two locals called "North Local 2" is
    /// ordinary, and it was refused.
    ///
    /// It also removes a translation nobody asked for: the plan keyed by name, the
    /// sheet carried both columns, and the builder read the identifier — so a
    /// voter's area went name → id → name on one round trip, with two consumers
    /// disagreeing about which column was authoritative.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub area_external_id: String,

    /// Everything else the client carries, passed through as user attributes.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, String>,
}

/// Text in as many languages as the plan enables.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Translated {
    /// `language code -> text`.
    #[serde(flatten)]
    pub by_language: std::collections::BTreeMap<String, String>,
}

impl Translated {
    pub fn new(english: &str) -> Self {
        let mut by_language = std::collections::BTreeMap::new();
        by_language.insert("en".to_string(), english.to_string());
        Translated { by_language }
    }

    /// The text in `language`, falling back to English and then to anything.
    ///
    /// A missing translation shows the English rather than an empty ballot line.
    /// Blank is never the right answer for a candidate's name.
    pub fn get(&self, language: &str) -> Option<&str> {
        self.by_language
            .get(language)
            .or_else(|| self.by_language.get("en"))
            .or_else(|| self.by_language.values().next())
            .map(String::as_str)
            .filter(|text| !text.is_empty())
    }

    pub fn is_empty(&self) -> bool {
        self.by_language.values().all(|text| text.is_empty())
    }
}

/// Which message this is.
///
/// Two, because two are what an election actually sends: one telling somebody
/// their ballot is ready, one telling them it is nearly too late. A free-text
/// alias would let a client invent a third that nothing downstream knows about.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum MessageKind {
    /// Sent when voting opens: here is your ballot.
    ///
    /// The default only so that a `PlannedMessage` can be built without naming
    /// every field — nothing chooses a kind by defaulting.
    #[default]
    InvitationToVote,
    /// Sent before it closes: you have not voted yet.
    GetOutTheVote,
}

impl MessageKind {
    /// The alias the platform's own template table uses.
    pub fn alias(self) -> &'static str {
        match self {
            MessageKind::InvitationToVote => "invitation-to-vote",
            MessageKind::GetOutTheVote => "get-out-the-vote",
        }
    }
}

/// When a message should go out.
///
/// A wall clock and a zone like every other moment in a plan, plus an optional
/// weekly repeat. **Nothing here sends anything** — see [`validate_plan`]'s
/// warning — so this is a schedule for a person to keep, which is why it carries
/// a human's timezone rather than an instant.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MessageSchedule {
    /// The dates to send it on.
    ///
    /// A list rather than one date, because a reminder campaign is several sends
    /// — three days before, one day before, the morning of — and one field forced
    /// a client to choose which of those to write down.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on: Vec<Timestamp>,

    /// Days of the week to repeat on, 1 = Monday, as ISO-8601 numbers them.
    ///
    /// Empty means send once. Numbers rather than names because a name is a
    /// language, and this file is read by whatever the client uses to send.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weekly: Vec<u8>,

    /// The time of day the weekly repeat goes out, `HH:MM`.
    ///
    /// **A wall clock rather than a `Timestamp`, and that is the whole
    /// argument.** A repeat has no date, so it has no instant to resolve an
    /// offset at — the offset for "every Monday at 09:30" is two different
    /// numbers either side of a daylight-saving change, and a `Timestamp` would
    /// force one of them to be written down as if it were the answer all year.
    /// Read on the plan's own schedule zone, which is the clock the dates in
    /// `on` already carry.
    ///
    /// One time for the whole repeat, not one per day. "Every Monday and
    /// Thursday at 09:30" is what a reminder campaign is; a time per day would
    /// change the shape of `weekly` for a case nobody has asked for.
    ///
    /// Empty means the plan predates this field, or a person left it blank.
    /// `validate_plan` says so out loud rather than inventing an hour, because a
    /// send-at-midnight nobody chose is worse than a question.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub weekly_at: String,
}

/// One message, in every language the ballot offers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedMessage {
    pub kind: MessageKind,

    /// The subject line, per language.
    #[serde(default)]
    pub subject: Translated,

    /// The body as plain text, per language.
    #[serde(default)]
    pub body: Translated,

    /// The body as HTML, per language.
    ///
    /// Separate from `body` rather than a flag on it: a sender that cannot do
    /// HTML still needs something to send, and the plain version is not
    /// derivable from the markup without deciding what a link becomes.
    #[serde(default, skip_serializing_if = "Translated::is_empty")]
    pub html: Translated,

    #[serde(default)]
    pub schedule: MessageSchedule,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Trustee {
    pub name: String,
    #[serde(default)]
    pub email: String,
}

/// The dates an election runs to.
///
/// Each moment carries the zone it was written in, because the platform acts on
/// an instant and a wall clock is not one. See [`super::time`] — a plan that
/// says `2027-03-01T09:00` produced a `scheduled_date` the scheduler could not
/// parse, so voting never opened and nothing said why.
///
/// A plan written before zones existed still opens: a bare string reads as UTC,
/// which is what it always meant, and validation says so rather than guessing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    /// When the trustees generate the election key. Not an imported event: it is
    /// something people have to attend, so it travels in the ceremony file.
    #[serde(default)]
    pub key_ceremony: Option<Timestamp>,

    #[serde(default)]
    pub voting_opens: Option<Timestamp>,

    #[serde(default)]
    pub voting_closes: Option<Timestamp>,

    #[serde(default)]
    pub tally_ceremony: Option<Timestamp>,

    /// Anything else with a date on it, for the schedule the client is handed.
    #[serde(default)]
    pub milestones: Vec<Milestone>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    pub event: String,
    pub date: String,
}

/// A group of voters who get the same ballot.
///
/// The name is plain text rather than translated, and that is not an oversight:
/// the voters CSV identifies a voter's area *by name*, so it is an identifier the
/// importer matches on. Two areas sharing a name would silently put voters in
/// whichever one the importer found first.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedArea {
    pub external_id: String,

    /// What the importer matches a voter's `area_name` against.
    #[serde(default)]
    pub name: String,

    /// The area this one sits inside, if any.
    ///
    /// A tree, because that is how districting is actually described — a local
    /// inside a region inside a state — and because the platform models it that
    /// way.
    ///
    /// **A contest assigned to a parent is on every descendant's ballot.** That
    /// is not this module's choice: when a publication is generated, windmill
    /// walks the path from the root down to each area and gathers every
    /// `area_contest` on the way — see
    /// [`crate::ballot_style::elections_contests_for_area`], which is now the one
    /// implementation of that walk and is what the preview calls. So assigning a
    /// contest to "National" and nothing else still puts it in front of every
    /// voter in North and South.
    ///
    /// This doc comment used to claim the opposite, and nothing tested it either
    /// way. `preview_tests::each_area_gets_the_contests_it_and_its_parents_vote_on`
    /// does now.
    #[serde(default)]
    pub parent_external_id: Option<String>,

    /// Whether voters in this area may vote in the early period.
    ///
    /// The other half of the event's `early_voting` channel, and the reason that
    /// channel is not a switch on its own. The Voting Portal opens early voting
    /// only when **both** are true — `ElectionSelectionScreen`'s
    /// `isEarlyVotingOpen` is `area_presentation.allow_early_voting ==
    /// allow_early_voting && early_voting_status == OPEN` — so an event with the
    /// channel on and no area allowing it has an Early voting button that opens a
    /// period nobody can vote in.
    ///
    /// The Admin Portal will not even show this field unless the event's channel is
    /// on (`Area/FormContent`'s `isEarlyVotingChannelEnabled`), which is why
    /// `check_voting_channels` refuses the reverse combination too.
    #[serde(default)]
    pub allow_early_voting: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedElection {
    pub external_id: String,
    #[serde(default)]
    pub name: Translated,

    /// What this election is about, for voters. Translatable.
    #[serde(default)]
    pub description: Translated,

    /// How many times a voter may change a vote they have already cast.
    ///
    /// One means cast once and that is final; two means one change. The platform
    /// keeps the *last* ballot and discards the earlier ones, so this is not "how
    /// many votes" — it is how many attempts.
    ///
    /// A real client decision, and the Admin Portal's election form exposes it,
    /// so a wizard-built election took whatever `election.hbs` said.
    #[serde(default = "one")]
    pub num_allowed_revotes: i64,

    /// Whether a voter may throw away a ballot they have already cast.
    ///
    /// Different from a contest's "I spoil my ballot" option, which is a choice on
    /// the paper. This discards a *cast* ballot and lets the voter start again, so
    /// it only means anything alongside a revote allowance.
    #[serde(default)]
    pub spoil_ballot_option: bool,

    /// Whether voting closes on the deadline or a little after it.
    ///
    /// `no-grace-period` or `grace-period-without-alert`, per
    /// `EGracePeriodPolicy`. A grace period lets a voter who opened the ballot
    /// before the close finish casting it, which is a decision about fairness
    /// rather than about software.
    #[serde(default = "no_grace_period")]
    pub grace_period_policy: String,

    /// How long that grace period lasts, in seconds. Zero without one.
    #[serde(default)]
    pub grace_period_secs: i64,

    /// Whether the voting screen is titled after this election or the whole event.
    ///
    /// `election` or `election-event`, per `EStartScreenTitlePolicy`. For an event
    /// with one election the event's name usually reads better; for one with
    /// several, the election's does.
    #[serde(default = "title_from_event")]
    pub start_screen_title_policy: String,

    /// The order this election's contests appear in.
    ///
    /// `custom`, `alphabetical` or `random`, the same three values as a contest's
    /// `candidates_order` — the Voting Portal sorts both through the same WASM
    /// helper. `custom` is the order on the Ballot screen, which the wizard
    /// already writes as each contest's `presentation.sort_order`; the other two
    /// tell the portal to ignore it.
    ///
    /// Exposed in the Admin Portal's election form and unsettable from a plan, so
    /// a wizard-built election could arrange its contests and not say whether the
    /// arrangement was honoured.
    #[serde(default = "custom_order")]
    pub contests_order: String,

    /// Which permission label gates access to this election.
    ///
    /// Internal: it is how the importer links a voter's authorisation to an
    /// election, and a client has no reason to choose one. Present because a
    /// delivery occasionally has to match an existing label, and empty means the
    /// builder derives it.
    #[serde(default)]
    pub permission_label: String,

    /// One set for every contest here, replacing whatever the contests say.
    ///
    /// `None` — the normal case — means each contest resolves the event default
    /// against its own overrides. `Some` is the old wizard's
    /// `samePolicyForAllContests`: edited once, and a contest's own overrides
    /// are not consulted. One field rather than a flag beside a value, because
    /// "shared is on but there is no shared value" is a state somebody would
    /// eventually produce and nobody could explain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<Overrides>,
    #[serde(default)]
    pub contests: Vec<PlannedContest>,
}

/// The values `#[serde(default = "…")]` gives a plan that omits them.
///
/// Written out rather than derived, because `derive(Default)` would give zero
/// revote attempts — an election nobody can vote in — and an empty
/// `grace_period_policy`, which is not one of the platform's two values. Those
/// attributes apply only when *deserialising*; a struct literal in Rust gets
/// this, which is what every fixture uses.
impl Default for PlannedElection {
    fn default() -> Self {
        PlannedElection {
            external_id: String::new(),
            name: Translated::default(),
            description: Translated::default(),
            num_allowed_revotes: 1,
            spoil_ballot_option: false,
            grace_period_policy: no_grace_period(),
            grace_period_secs: 0,
            start_screen_title_policy: title_from_event(),
            contests_order: custom_order(),
            permission_label: String::new(),
            shared: None,
            contests: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedContest {
    pub external_id: String,
    #[serde(default)]
    pub name: Translated,
    /// What this contest is about, shown under its name on the ballot.
    ///
    /// Translatable. It was a `String`, which was wrong: the Admin Portal edits
    /// `presentation.i18n.<lang>.description` per language and keeps the flat
    /// column as a mirror of the English one, so a plan carrying one text put the
    /// same words in front of every voter whatever language they read.
    #[serde(default, deserialize_with = "translated_or_plain")]
    pub description: Translated,

    /// How many candidates a voter may choose.
    #[serde(default = "one")]
    pub max_votes: i64,

    /// How many candidates the contest elects.
    ///
    /// The TypeScript hard-coded this to 1 while letting `max_votes` be anything,
    /// so a "choose 3" contest silently elected one person.
    #[serde(default = "one")]
    pub winners: i64,

    #[serde(default)]
    pub candidates: Vec<PlannedCandidate>,

    /// What this contest says about how it behaves, over the event's defaults.
    #[serde(default, skip_serializing_if = "Overrides::is_empty")]
    pub overrides: Overrides,

    /// Whether a voter may type a name the ballot does not list.
    ///
    /// The ballot codec has carried write-ins from the beginning —
    /// `raw_ballot::encode` packs the text into the ballot integer a byte at a
    /// time and `contest_context` reserves the bases for it — and no plan could
    /// ask for one, so the feature was reachable only by editing a bundle.
    ///
    /// It is a pair, and both halves have to agree: this switch, and one
    /// candidate marked `is_write_in` per slot a voter may fill. `validate`
    /// refuses either half alone, because the codec silently does nothing useful
    /// with a mismatch — a write-in candidate with the switch off is a nameless
    /// option on the ballot, and the switch on with no such candidate offers a
    /// voter nothing to write in.
    #[serde(default)]
    pub allow_writeins: bool,

    /// How many names a voter may add, when write-ins are allowed.
    ///
    /// One candidate row is minted per slot. Zero with `allow_writeins` on is
    /// the mismatch above, and `validate` says so rather than emitting a contest
    /// the codec cannot use.
    #[serde(default)]
    pub write_in_slots: i64,

    /// Which areas put this contest on their ballot.
    ///
    /// Empty means every area, which is what a plan that has never thought about
    /// districting wants and what one with a single area always wants. Naming
    /// areas explicitly is how a contest becomes local to some of them.
    #[serde(default)]
    pub areas: Vec<String>,
}

/// Shown, because a voter being able to confirm their own ballot arrived is the
/// kind of thing an election should have to argue itself out of rather than into.
/// Whether text is shaped like an email address.
///
/// One `@`, something before it, and a dotted something after it with no spaces
/// anywhere. Not a parser: the addresses this is given come from a person typing into
/// a form, and the mistakes worth catching are a name in the email box, a missing
/// domain and a stray space — not the exotic corners of RFC 5322, where being stricter
/// than the specification would refuse a valid address and be a worse defect than the
/// one it prevented.
fn looks_like_email(text: &str) -> bool {
    if text.chars().any(char::is_whitespace) {
        return false;
    }
    let mut halves = text.split('@');
    let (Some(local), Some(domain), None) =
        (halves.next(), halves.next(), halves.next())
    else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

fn yes() -> bool {
    true
}

fn show_logs() -> String {
    "show-logs-tab".to_string()
}

/// The order somebody arranged, which is what the wizard is for.
fn custom_order() -> String {
    "custom".to_string()
}

fn no_grace_period() -> String {
    "no-grace-period".to_string()
}

/// The event's name, not the election's.
///
/// Most events have one election, where the event's name is the one the client
/// wrote and the election's is a subdivision nobody outside the platform thinks
/// about.
fn title_from_event() -> String {
    "election-event".to_string()
}

fn one() -> i64 {
    1
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedCandidate {
    pub external_id: String,
    #[serde(default)]
    pub name: Translated,

    /// A line about the candidate, shown under their name. Translatable.
    #[serde(default)]
    pub description: Translated,
    /// A "none of the above" option rather than a person.
    #[serde(default)]
    pub explicit_blank: bool,
    /// A "spoil my ballot" option rather than a person.
    #[serde(default)]
    pub explicit_invalid: bool,
    /// On the ballot, drawn but not selectable — `presentation.is_disabled`.
    ///
    /// A candidate who stood down after the ballot was approved. The platform's own
    /// concept, not one invented here: `CandidatePresentation::is_disabled` carries
    /// it, `Candidate::is_disabled()` reads it, the voting portal draws such a
    /// candidate at half opacity and `categoryService` leaves them out of a
    /// category's list. The wizard has offered the checkbox for a while under the
    /// name *Withdrawn*, and this struct had no field for it and no catch-all — so
    /// every tick was accepted on screen, dropped by the core, absent from the
    /// delivery, and gone when the event was reopened.
    #[serde(default)]
    pub disabled: bool,

    /// A photograph, shown beside the name on the ballot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<CandidateImage>,
}

/// One voter-facing help document in a plan.
///
/// The document identifier is **not** here, for the same reason it is not on
/// [`CandidateImage`]: it is derived from `external_id` by
/// [`material_document_id`], so it is stable across rebuilds and cannot drift out
/// of step with the archive entry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedMaterial {
    /// Names the material within this plan, and seeds its document identifier.
    pub external_id: String,

    /// What a voter sees in the list, per language.
    #[serde(default)]
    pub title: Translated,

    /// The platform's own `kind`, carried opaquely.
    #[serde(default)]
    pub kind: String,

    /// The file's own name. Kept free of anything UUID-shaped, because import runs
    /// it through `replace_ids_in_filename`.
    #[serde(default)]
    pub file_name: String,

    /// The document itself, base64 in the saved plan and raw everywhere else.
    #[serde(default, with = "base64_bytes")]
    pub bytes: Vec<u8>,

    /// Uploaded but not shown to voters yet.
    #[serde(default)]
    pub is_hidden: bool,
}

/// A candidate's photograph, as the plan carries it.
///
/// The bytes are in the plan on purpose. A plan is the document somebody reopens
/// next month when a candidate withdraws, and a plan that lost its photographs on
/// reopening would mean uploading forty of them again — which is exactly the
/// tedium this is meant to remove. It does make a plan with photographs large;
/// that is the right trade, and the alternative is a plan that is not a complete
/// description of the election.
///
/// The document identifier is **not** here. It is derived from the candidate's
/// `external_id` by [`image_document_id`], so it is stable across rebuilds without
/// anybody storing it, and cannot drift out of step with the archive entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateImage {
    /// The file's own name, which becomes the last segment of both the public path
    /// and the archive entry.
    ///
    /// Worth keeping free of anything UUID-shaped: import runs the name through
    /// `replace_ids_in_filename`, so an identifier inside it would be rewritten and
    /// the stored name would stop matching the url.
    pub file_name: String,

    /// The image itself, base64 in the saved plan and raw everywhere else.
    #[serde(with = "base64_bytes")]
    pub bytes: Vec<u8>,
}

/// The document identifier for a candidate's photograph.
///
/// Derived rather than stored, from the same factory as every other identifier in a
/// bundle, so two runs of one plan produce the same archive. Its own `kind`, so a
/// candidate and their photograph do not collide.
pub fn image_document_id(
    ids: &IdFactory,
    candidate_external_id: &str,
) -> String {
    ids.uid("document", &[candidate_external_id])
}

/// The document identifier for the event's logo.
///
/// Its own `kind`, like every other derived id here, so an event and its logo do not
/// collide on one uuid — the event's `external_id` seeds both.
pub fn logo_document_id(ids: &IdFactory, event_external_id: &str) -> String {
    ids.uid("logo-document", &[event_external_id])
}

/// Every photograph in a plan, with the identifier the sheet will name it by.
///
/// The identifier is derived here and in `candidates_sheet` from the same
/// [`image_document_id`], which is what keeps the archive entry and the JSON in
/// step. Deriving it in two places rather than storing it once is deliberate: a
/// stored identifier is a thing that can be edited into disagreement, and this one
/// has no reason to be editable.
/// A support material's document identifier.
///
/// Its own `kind`, so a material and a candidate sharing an `external_id` do not
/// collide on the same derived uuid.
pub fn material_document_id(ids: &IdFactory, external_id: &str) -> String {
    ids.uid("material-document", &[external_id])
}

/// Every support material file in a plan, with the identifier its row will name.
pub fn plan_materials(plan: &Blueprint) -> Vec<MaterialFile> {
    let Some(ids) = IdFactory::new(&plan.external_id) else {
        return Vec::new();
    };
    plan.materials
        .iter()
        .filter(|material| !material.bytes.is_empty())
        .map(|material| MaterialFile {
            // Keyed by name from here on: the builder matches the sheet's `file`
            // column against these and derives the identifier itself, so the JSON
            // and the archive cannot disagree about which is which.
            document_id: String::new(),
            file_name: material.file_name.clone(),
            bytes: material.bytes.clone(),
        })
        .collect()
}

pub fn plan_images(plan: &Blueprint) -> Vec<ImageFile> {
    let Some(ids) = IdFactory::new(&plan.external_id) else {
        return Vec::new();
    };
    let mut images = Vec::new();
    // The logo rides with the photographs rather than with the support materials,
    // and the difference is not cosmetic: `images/` is uploaded public and
    // `export_S3_files/` private, so a logo in the private branch would 404 for every
    // voter the ballot is drawn for.
    if let Some(logo) = &plan.logo {
        images.push(ImageFile {
            document_id: logo_document_id(&ids, &plan.external_id),
            file_name: logo.file_name.clone(),
            bytes: logo.bytes.clone(),
        });
    }
    for election in &plan.elections {
        for contest in &election.contests {
            for candidate in &contest.candidates {
                if let Some(image) = &candidate.image {
                    images.push(ImageFile {
                        document_id: image_document_id(
                            &ids,
                            &candidate.external_id,
                        ),
                        file_name: image.file_name.clone(),
                        bytes: image.bytes.clone(),
                    });
                }
            }
        }
    }
    images
}

/// Base64 for the saved plan, raw bytes in memory.
///
/// JSON has no byte string, and a plan is JSON. `serde_json` would happily write a
/// `Vec<u8>` as an array of numbers, which is four times the size and unreadable.
mod base64_bytes {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        bytes: &[u8],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(deserializer)?;
        STANDARD.decode(text).map_err(serde::de::Error::custom)
    }
}

/// What a contest ends up with.
///
/// Most specific last, with one exception: an election that has claimed the
/// decision does not consult its contests at all. That is a statement about the
/// election rather than a value copied onto each contest and then silently
/// stale, which is what would happen if "shared" merely pre-filled them.
pub fn resolve(
    plan: &Blueprint,
    election: &PlannedElection,
    contest: &PlannedContest,
) -> Behaviour {
    let claimed = election.shared.as_ref().unwrap_or(&contest.overrides);
    plan.defaults.apply(claimed)
}

/// The area used when a plan names none.
///
/// A bundle needs an area and a ballot link or no voter sees anything, so a plan
/// that has never thought about districting gets one covering everybody.
///
/// Named rather than anonymous because the voters CSV resolves an area by name,
/// and because a delivery engineer opening the generated event should be able to
/// tell nobody chose it.
pub const DEFAULT_AREA_EXTERNAL_ID: &str = "all-voters";
pub const DEFAULT_AREA_NAME: &str = "All voters";

/// Check a plan, before anything is built from it.
///
/// These are questions about the *plan*, in the wizard's own vocabulary — a
/// trustee threshold higher than the number of trustees, a voting window that
/// closes before it opens. [`super::validate`] then checks the bundle, and a
/// problem there is phrased in the bundle's vocabulary. Both run; they are asking
/// different questions and an author needs both answers.
pub fn validate_plan(plan: &Blueprint) -> Report {
    let mut report = Report::default();

    if plan.version > BLUEPRINT_VERSION {
        report.push(Problem::error(
            Code::InvalidValue,
            "version",
            format!(
                "this plan was saved by a newer version ({} against {}). Opening \
                 it here would silently drop whatever that version added.",
                plan.version, BLUEPRINT_VERSION
            ),
        )
.id("plan.saved-by-newer-version").detail("saved", plan.version).detail("supported", BLUEPRINT_VERSION));
    }

    if plan.external_id.trim().is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "external_id",
            "the event needs an identifier: every generated id is derived from it, \
             so without one nothing can be built twice the same way",
        )
.id("event.no-identifier"));
    }

    if plan.name.is_empty() {
        report.push(Problem::error(
            Code::MissingField,
            "name",
            "the event needs a name. Voters see it above the ballot, and it \
             becomes the login page's title.",
        )
.id("event.no-name"));
    }

    check_languages(plan, &mut report);
    check_language_detection(plan, &mut report);
    check_logo(plan, &mut report);
    check_trustees(plan, &mut report);
    check_schedule(plan, &mut report);
    check_areas(plan, &mut report);
    check_census(plan, &mut report);
    check_ballot(plan, &mut report);
    check_unique_identifiers(plan, &mut report);

    // There was a `messages.not-automatic` warning here, on every plan carrying a
    // message: `scheduled_events.rs` handles `SEND_TEMPLATE` with an empty arm, so
    // nothing sent them and the wizard said so.
    //
    // Removed deliberately, and not because the sentence was wrong. It is a
    // statement about a gap in the *platform* rather than about anything in the
    // plan, and it fired on every message anybody wrote — so the one screen where
    // a client does creative work always ended in an amber panel about somebody
    // else's roadmap. That is a warning nobody can act on, which is how a list of
    // warnings comes to be scrolled past.
    //
    // The gap is being closed. When it is, nothing here needs changing; if it is
    // not, the people who need to know are the delivery team, and the delivery
    // guide tells them. `validate_plan` is for what is wrong with *this plan*.

    // A repeat with no hour in it.
    //
    // The screen makes this unreachable — ticking *Repeat every week* writes an
    // hour alongside the day — so this is here for the plans the screen did not
    // write: one from before the field existed, one hand-edited, one round-tripped
    // through a workbook somebody cleared a cell in. Whoever sends it would
    // otherwise have to guess, and the guess that costs least to make is midnight.
    for message in &plan.messages {
        if !message.schedule.weekly.is_empty()
            && message.schedule.weekly_at.is_empty()
        {
            report.push(
                Problem::warning(
                    Code::MissingField,
                    "messages",
                    "a message repeats every week but does not say at what time, \
                     so whoever sends it has to choose an hour nobody wrote down.",
                )
                .id("messages.weekly-no-time"),
            );
            break;
        }
    }

    if plan.contacts.is_empty() {
        report.push(Problem::warning(
            Code::MissingField,
            "contacts",
            "nobody is listed as a point of contact. On election day this is who \
             gets called.",
        )
.id("contacts.none"));
    }

    report
}

/// The ballot's languages, and which one a voter starts in.
fn check_languages(plan: &Blueprint, report: &mut Report) {
    if plan.languages.is_empty() {
        report.push(Problem::warning(
            Code::MissingField,
            "languages",
            "no languages, so the ballot falls back to English. That is a safety \
             net rather than a choice.",
        )
.id("languages.none"));
        return;
    }

    let Some(chosen) = plan.default_language.as_deref() else {
        return;
    };

    if !plan.languages.iter().any(|each| each == chosen) {
        // An error, not a warning. The builder would fall back to the first
        // language, so this imports cleanly and opens in a language nobody
        // chose — which is exactly the class of failure nobody notices until a
        // voter says the ballot came up in the wrong language.
        report.push(
            Problem::error(
                Code::InvalidValue,
                "default_language",
                format!(
                "'{chosen}' is not one of the languages this ballot is offered \
                 in ({}). Voters would get the first one instead.",
                plan.languages.join(", ")
            ),
            )
            .id("languages.default-not-offered")
            .detail("chosen", chosen)
            .detail("offered", plan.languages.join(", ")),
        );
    }
}

/// The one language setting whose value space is not a list of the plan's own
/// languages, so it is the one that can carry a word the platform never heard of.
fn check_language_detection(plan: &Blueprint, report: &mut Report) {
    let Some(policy) = plan
        .language_detection_policy
        .as_deref()
        .filter(|each| !each.is_empty())
    else {
        return;
    };

    if !super::validate::LANGUAGE_DETECTION_POLICIES.contains(&policy) {
        report.push(
            Problem::error(
                Code::InvalidValue,
                "language_detection_policy",
                format!(
                    "'{policy}' is not a language detection policy. The platform \
                     knows {}.",
                    super::validate::LANGUAGE_DETECTION_POLICIES.join(" and ")
                ),
            )
            .id("languages.detection-unknown")
            .detail("policy", policy),
        );
    }
}

/// What the plan says about the logo, when it says two things.
///
/// The precedence is real and one-sided — [`event_sheet`] writes the file and drops
/// the link — so the risk is a plan that carries a link somebody typed months ago,
/// gains a file, and builds without the link ever being mentioned again. Said out
/// loud rather than resolved silently, because the two answers usually differ and
/// only one of them is on the ballot.
///
/// A file with no bytes is the same as no file: the wizard clears `file_name` and
/// `bytes` together, and a name with nothing behind it would name an archive entry
/// that does not exist — which fails the whole import rather than losing a picture.
fn check_logo(plan: &Blueprint, report: &mut Report) {
    let file = plan.logo.as_ref().filter(|file| !file.bytes.is_empty());
    let url = plan.logo_url.as_deref().filter(|url| !url.is_empty());

    if let (Some(file), Some(url)) = (file, url) {
        report.push(
            Problem::warning(
                Code::InvalidValue,
                "logo",
                format!(
                    "this plan carries both an uploaded logo ('{}') and a link \
                     ('{url}'). The file is what ships; the link is ignored.",
                    file.file_name
                ),
            )
            .id("logo.file-and-link")
            .detail("file", &file.file_name)
            .detail("url", url),
        );
    }

    if let Some(logo) = plan.logo.as_ref() {
        if logo.bytes.is_empty() && !logo.file_name.is_empty() {
            report.push(
                Problem::error(
                    Code::MissingField,
                    "logo",
                    format!(
                        "'{}' is named as the logo and carries no bytes. An \
                         archive entry with nothing in it fails the import \
                         rather than losing a picture.",
                        logo.file_name
                    ),
                )
                .id("logo.no-bytes")
                .detail("file", &logo.file_name),
            );
        }
    }
}

fn check_trustees(plan: &Blueprint, report: &mut Report) {
    // Two is the floor, and both halves of it are errors rather than warnings.
    //
    // The guarantee threshold encryption buys is that *no single party can read the
    // votes*. One trustee — or a threshold of one, whatever the list length — hands
    // that away: one person can decrypt every ballot, alone, and nobody else can
    // tell. It is not recoverable either, because by the time it matters the votes
    // have already been cast under that key.
    //
    // The previous version warned and let it build. A warning on the screen somebody
    // reads last, about a property they cannot check afterwards, is the shape of
    // defect this whole validation pass exists to refuse. `EA-70` reverses it
    // deliberately: `a_threshold_of_one_is_allowed_but_said_out_loud` is now
    // `a_threshold_of_one_is_refused`, and this comment is the record of why
    // (`INV-26`).
    //
    // Two named complaints rather than one with a conditional lead, for the same
    // reason as the threshold below: "no trustees" and "one trustee" are different
    // sentences, and a count that crosses the wasm boundary as a string cannot
    // pluralise a translation the way a number would.
    let too_few =
        "an election key needs at least two people to hold it. With one, that \
         person can decrypt every ballot on their own, which is the guarantee the \
         encryption is there to provide.";

    // A row with no name and no address is nobody.
    //
    // The wizard keeps the list at least as long as the threshold, so a new plan
    // arrives with two blank rows rather than an empty list and an instruction to
    // add some. Counting those as trustees would mean the commonest state of a new
    // plan — two empty boxes — reported as *complete*, and the real complaint
    // ("nobody is holding the key") replaced by two lesser ones about missing
    // fields. Blank rows are still checked individually below, so nothing is
    // silently accepted; they simply do not count towards having anybody.
    let named = plan
        .trustees
        .iter()
        .filter(|trustee| {
            !trustee.name.trim().is_empty() || !trustee.email.trim().is_empty()
        })
        .count();

    if named == 0 {
        report.push(
            Problem::error(
                Code::MissingField,
                "trustees",
                format!("no trustees — {too_few}"),
            )
            .id("trustees.none"),
        );
    } else if named == 1 {
        report.push(
            Problem::error(
                Code::MissingField,
                "trustees",
                format!("one trustee — {too_few}"),
            )
            .id("trustees.only-one"),
        );
    }

    if plan.trustees.is_empty() {
        // Nothing below can say anything useful without a list.
        return;
    }

    // Two pushes rather than one with a conditional message. They are different
    // complaints — a threshold of zero opens the tally to nobody, a threshold of one
    // opens it to anybody on the list — and one `id` cannot name two sentences, which
    // is the whole reason `id` exists.
    if plan.trustee_threshold == 0 {
        report.push(
            Problem::error(
                Code::InvalidValue,
                "trustee_threshold",
                "a threshold of zero means the tally can be opened by nobody at all",
            )
            .id("threshold.zero"),
        );
    } else if plan.trustee_threshold == 1 {
        report.push(
            Problem::error(
                Code::InvalidValue,
                "trustee_threshold",
                "a threshold of one means any single trustee can open the tally \
                 alone, whatever the list says — the same guarantee as having one \
                 trustee, and none at all against that person",
            )
            .id("threshold.one"),
        );
    }

    if plan.trustee_threshold as usize > named {
        // The failure mode is the worst kind: everything works until the tally,
        // and then the result cannot be decrypted by anyone.
        report.push(
            Problem::error(
                Code::ContestArithmetic,
                "trustee_threshold",
                format!(
                "{} of {} trustees are required, which cannot be met. The key \
                 would be generated and the result could never be decrypted.",
                plan.trustee_threshold, named
            ),
            )
            .id("threshold.above-trustees")
            .detail("threshold", plan.trustee_threshold)
            .detail("trustees", named),
        );
    }

    // The names have to already exist in the tenant, and nothing downstream says
    // so. `import_election_event.rs` builds a `HashMap<name, id>` from
    // `get_all_trustees(tenant_id)` and maps the bundle's `trustee_ids` through
    // it with `.unwrap_or_default()` — so a name that matches nothing becomes an
    // **empty string** and the ceremony imports with a member who does not exist.
    //
    // Nothing fails. The ceremony is created, the trustee count looks right, and
    // the missing member is discovered when the key is generated or, worse, when
    // the threshold cannot be met at the tally.
    //
    // A warning rather than an error because this side cannot see the tenant:
    // whether "Ada Lovelace" is provisioned is not knowable from a plan file.
    for (index, trustee) in plan.trustees.iter().enumerate() {
        // The address the ceremony invitation goes to. A trustee who never gets it
        // does not attend, and a threshold that cannot be met is discovered at the
        // tally — the same failure as a name that resolves to nobody, arriving by a
        // different route.
        //
        // Deliberately a shape check and not an RFC 5322 parser: this cannot know
        // whether an address receives mail, and refusing a valid-but-unusual address
        // would be worse than accepting one that bounces. What it catches is the
        // mistakes people actually make — a name typed into the email box, a missing
        // domain, a stray space.
        let email = trustee.email.trim();
        if email.is_empty() {
            report.push(Problem::error(
                Code::MissingField,
                format!("trustees[{index}].email"),
                "a trustee needs an email address; it is how they are invited to \
                 the key ceremony",
            )
.id("trustee.no-email").detail("row", index + 1));
        } else if !looks_like_email(email) {
            report.push(
                Problem::error(
                    Code::InvalidValue,
                    format!("trustees[{index}].email"),
                    format!("'{email}' is not an email address"),
                )
                .id("trustee.email-malformed")
                .detail("email", email),
            );
        }

        if trustee.name.trim().is_empty() {
            report.push(Problem::error(
                Code::MissingField,
                format!("trustees[{index}].name"),
                "a trustee needs a name. The importer resolves the key ceremony's \
                 members by name against the trustees already provisioned in the \
                 tenant, and an empty one silently becomes a member who does not \
                 exist.",
            )
.id("trustee.no-name").detail("row", index + 1));
        }
    }

    // Deliberately *not* a warning on every plan that has trustees.
    //
    // A first version pushed one unconditionally, saying that the ceremony's
    // members are resolved by name against the tenant. True, and important — and a
    // warning that fires on every sound plan is noise that teaches people to skim
    // the report. `a_sound_plan_has_nothing_to_report` caught it.
    //
    // It is a handover fact rather than a defect, so it lives where somebody reads
    // it once: the trustees section's own hint, and the reference page.
}

fn check_schedule(plan: &Blueprint, report: &mut Report) {
    let schedule = &plan.schedule;

    // Each moment on its own first: an ordering complaint about a time that is
    // not a time would send somebody looking at the wrong field.
    let mut found = Vec::new();
    for (at, moment) in [
        ("schedule.key_ceremony", &schedule.key_ceremony),
        ("schedule.voting_opens", &schedule.voting_opens),
        ("schedule.voting_closes", &schedule.voting_closes),
        ("schedule.tally_ceremony", &schedule.tally_ceremony),
    ] {
        if let Some(moment) = moment {
            time::check(moment, at, &mut found);
        }
    }
    let unreadable = found
        .iter()
        .any(|problem| problem.severity == Severity::Error);
    for problem in found {
        report.push(problem);
    }
    if unreadable {
        return;
    }

    match (&schedule.voting_opens, &schedule.voting_closes) {
        (None, _) | (_, None) => report.push(Problem::warning(
            Code::MissingSchedule,
            "schedule",
            "the voting window is incomplete, so the period will have to be opened \
             or closed by hand in the Admin Portal",
        )
.id("schedule.window-incomplete")),
        (Some(opens), Some(closes)) => {
            // By instant, not by text. Two moments in different zones sort by
            // their strings in whatever order the digits happen to fall.
            if !opens.is_empty()
                && !closes.is_empty()
                && time::compare(closes, opens) != Ordering::Greater
            {
                report.push(Problem::error(
                    Code::InvalidValue,
                    "schedule.voting_closes",
                    "voting closes before it opens, so it would never be open",
                )
.id("schedule.crosses-daylight-saving")
.id("schedule.closes-before-opens"));
            }

            // Both endpoints in one zone but at different offsets means the
            // window crosses a daylight-saving change — legitimate, and worth
            // knowing about, because an hour moves under whoever planned it.
            if opens.zone == closes.zone
                && !opens.zone.trim().is_empty()
                && opens.offset_minutes != closes.offset_minutes
            {
                report.push(Problem::warning(
                    Code::InvalidValue,
                    "schedule",
                    "the voting window crosses a daylight-saving change, so it is \
                     an hour longer or shorter than the clock times suggest",
                ));
            }
        }
    }

    if let (Some(ceremony), Some(opens)) =
        (&schedule.key_ceremony, &schedule.voting_opens)
    {
        if !ceremony.is_empty()
            && !opens.is_empty()
            && time::compare(ceremony, opens) != Ordering::Less
        {
            report.push(Problem::error(
                Code::InvalidValue,
                "schedule.key_ceremony",
                "the key ceremony is not before voting opens. The election key has \
                 to exist before a vote can be encrypted with it.",
            )
.id("schedule.key-ceremony-not-first"));
        }
    }

    if let (Some(tally), Some(closes)) =
        (&schedule.tally_ceremony, &schedule.voting_closes)
    {
        if !tally.is_empty()
            && !closes.is_empty()
            && time::compare(tally, closes) != Ordering::Greater
        {
            report.push(Problem::error(
                Code::InvalidValue,
                "schedule.tally_ceremony",
                "the tally ceremony is not after voting closes, so it would count \
                 votes that had not been cast yet",
            )
.id("schedule.tally-ceremony-too-early"));
        }
    }
}

/// Districting: the areas themselves, before any contest points at one.
fn check_areas(plan: &Blueprint, report: &mut Report) {
    for (index, area) in plan.areas.iter().enumerate() {
        let at = format!("areas[{index}]");

        if area.external_id.trim().is_empty() {
            report.push(
                Problem::error(
                    Code::MissingField,
                    &at,
                    "an area needs an identifier",
                )
                .id("area.no-identifier")
                .detail("row", index + 1),
            );
        }

        if area.name.trim().is_empty() {
            report.push(
                Problem::error(
                    Code::MissingField,
                    format!("{at}.name"),
                    "an area needs a name: the voters CSV identifies a voter's \
                     area by name, not by id, so an unnamed area is one no voter \
                     can be put in",
                )
                .about(Some(&area.external_id))
.id("area.no-name").detail("row", index + 1),
            );
        }

        // The voters CSV resolves by name, so a duplicate silently assigns voters
        // to whichever one the importer happens to find first.
        if let Some(earlier) = plan.areas[..index].iter().find(|other| {
            !other.name.trim().is_empty() && other.name == area.name
        }) {
            report.push(
                Problem::error(
                    Code::DuplicateId,
                    format!("{at}.name"),
                    format!(
                        "two areas are both named '{}' ('{}' and '{}'). The voters \
                         CSV resolves an area by name, so voters would land in \
                         whichever the importer found first.",
                        area.name, earlier.external_id, area.external_id
                    ),
                )
                .about(Some(&area.external_id))
.id("area.duplicate-name").detail("name", &area.name).detail("first", &earlier.external_id).detail("second", &area.external_id),
            );
        }

        if let Some(parent) = area
            .parent_external_id
            .as_ref()
            .filter(|parent| !parent.is_empty())
        {
            if parent == &area.external_id {
                report.push(
                    Problem::error(
                        Code::AreaCycle,
                        format!("{at}.parent_external_id"),
                        "an area cannot be inside itself",
                    )
                    .about(Some(&area.external_id))
                    .id("area.inside-itself"),
                );
            } else if !plan
                .areas
                .iter()
                .any(|other| &other.external_id == parent)
            {
                report.push(
                    Problem::error(
                        Code::DanglingReference,
                        format!("{at}.parent_external_id"),
                        format!("no area has the identifier '{parent}'"),
                    )
                    .about(Some(&area.external_id))
                    .id("area.parent-unknown")
                    .detail("parent", parent),
                );
            }
        }
    }
}

/// The census, when the plan carries one.
///
/// Two failures matter, and both are the kind nobody sees until a voter cannot
/// vote:
///
///   - **A duplicate username.** The importer derives a voter's id from it, so
///     two rows sharing one produce one account, and whichever row came second
///     silently replaced the first.
///   - **An area name no area has.** Voters are matched to areas *by name*, so
///     a misspelling gives a voter no ballot at all — and this is the one place
///     where a name is doing real work rather than labelling something.
///
/// A census with no voters in it is not an error: a plan may carry the shape of
/// an election long before anybody has the membership list.
fn check_census(plan: &Blueprint, report: &mut Report) {
    if plan.voters.is_empty() {
        return;
    }

    let named: std::collections::BTreeSet<&str> = plan
        .areas
        .iter()
        .map(|area| area.external_id.as_str())
        .collect();

    let mut seen: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();

    for (index, voter) in plan.voters.iter().enumerate() {
        let username = voter.username.trim();

        if username.is_empty() {
            report.push(Problem::error(
                Code::MissingField,
                format!("voters[{index}].username"),
                "a voter needs a username; it is what they sign in as and what                  their account is derived from",
            )
.id("voter.no-username").detail("row", index + 1));
            continue;
        }

        if let Some(first) = seen.insert(username, index) {
            report.push(Problem::error(
                Code::DuplicateId,
                format!("voters[{index}].username"),
                format!(
                    "'{username}' is also row {first}. Two voters sharing a                      username become one account, and this one would replace                      the other without saying so."
                ),
            )
.id("voter.duplicate-username").detail("username", username).detail("first", first + 1));
        }

        // Blank is not "the default area", which is what this used to assume.
        //
        // `build_tables::voter_area_name` reads `area.external_id` off every voter
        // row and refuses the whole bundle when it is absent, so a voter with no
        // area is a plan that cannot be built — and this screen was calling it
        // Ready to build. There is no default: an area is how a voter is given a
        // ballot at all.
        let area = voter.area_external_id.trim();
        if area.is_empty() {
            report.push(Problem::error(
                Code::MissingField,
                format!("voters[{index}].area.external_id"),
                "a voter needs an area: it is what decides which ballot they are \
                 handed, and the build refuses a census row without one",
            )
            .id("voter.no-area")
            .detail("row", index + 1));
        } else if !named.contains(area) {
            report.push(Problem::error(
                Code::DanglingReference,
                format!("voters[{index}].area.external_id"),
                format!(
                    "no area has external_id '{area}', so this voter would get no \
                     ballot. Copy the identifier from the area rather than \
                     retyping it — it is the column headed `area.external_id`, \
                     not the area's name."
                ),
            )
.id("voter.area-unknown").detail("area", area).detail("row", index + 1));
        }
    }

    // Said once rather than per voter: a census loaded against the wrong plan
    // produces one of these for every row, and ten thousand copies of the same
    // sentence is a report nobody reads.
    if !plan.areas.is_empty()
        && plan
            .voters
            .iter()
            .all(|voter| voter.area_external_id.trim().is_empty())
    {
        report.push(Problem::warning(
            Code::MissingField,
            "voters",
            "this election has areas but no voter names one, so every voter \
             gets the default ballot. If the districting is meant to apply, the \
             census needs an area column.",
        )
.id("census.no-area-column"));
    }
}

/// Identifiers are unique across the whole event, and nothing here said so.
///
/// The builder's `check_unique_ids` refuses a bundle whose entities collide, and it
/// is right to: an `external_id` is what every other sheet points at, so the second
/// of two silently replaces the first — a candidate disappears from a ballot, or a
/// contest ends up with somebody else's options on it. The wizard had no equivalent,
/// so the screen said Ready to build and the download produced nothing.
///
/// Event-wide rather than per contest, which is the part that surprises people: two
/// contests may not each have a `yes`. That is why the message names both places.
fn check_unique_identifiers(plan: &Blueprint, report: &mut Report) {
    /// Where an identifier was found, for a message that can point at both.
    let mut seen: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();

    let mut claim = |id: &str, at: String, what: &str, report: &mut Report| {
        let id = id.trim();
        if id.is_empty() {
            // Reported by whoever owns the field; a second sentence here would
            // put the same problem on two screens.
            return;
        }
        if let Some(first) = seen.get(id) {
            report.push(
                Problem::error(
                    Code::DuplicateId,
                    &at,
                    format!(
                        "'{id}' is already used by {first}. Identifiers are unique \
                         across the whole election event, so the second one \
                         replaces the first instead of being added."
                    ),
                )
                .id("identifier.duplicate")
                .detail("identifier", id)
                .detail("first", first.clone()),
            );
        } else {
            seen.insert(id.to_string(), format!("{what} at {at}"));
        }
    };

    for (index, area) in plan.areas.iter().enumerate() {
        claim(
            &area.external_id,
            format!("areas[{index}]"),
            "an area",
            report,
        );
    }

    for (index, election) in plan.elections.iter().enumerate() {
        let at = format!("elections[{index}]");
        claim(&election.external_id, at.clone(), "an election", report);

        for (contest_index, contest) in election.contests.iter().enumerate() {
            let at = format!("{at}.contests[{contest_index}]");
            claim(&contest.external_id, at.clone(), "a contest", report);

            for (candidate_index, candidate) in
                contest.candidates.iter().enumerate()
            {
                claim(
                    &candidate.external_id,
                    format!("{at}.candidates[{candidate_index}]"),
                    "a candidate",
                    report,
                );
            }
        }
    }
}

fn check_ballot(plan: &Blueprint, report: &mut Report) {
    if plan.elections.is_empty() {
        report.push(
            Problem::error(
                Code::MissingField,
                "elections",
                "an election event needs at least one election",
            )
            .id("ballot.no-elections"),
        );
        return;
    }

    for (index, election) in plan.elections.iter().enumerate() {
        let at = format!("elections[{index}]");

        if election.contests.is_empty() {
            // An error, not a warning, because `build_election_event` refuses it.
            // It read as advice for a while and the screen said Ready to build,
            // which is the one thing this verdict may not do: the button then
            // produces nothing and there is no way to find out why.
            report.push(
                Problem::error(
                    Code::BallotCoverage,
                    &at,
                    "this election has no contests, so nobody votes in it and it \
                     cannot be built",
                )
                .about(Some(&election.external_id))
                .id("election.no-contests"),
            );
        }

        for (contest_index, contest) in election.contests.iter().enumerate() {
            let at = format!("{at}.contests[{contest_index}]");
            let choices = contest
                .candidates
                .iter()
                .filter(|candidate| {
                    !candidate.explicit_blank && !candidate.explicit_invalid
                })
                .count();

            if contest.max_votes < 1 {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        &at,
                        "a voter may choose fewer than one candidate, so there is \
                         nothing to vote for",
                    )
                    .about(Some(&contest.external_id))
.id("contest.max-votes-below-one"),
                );
            }

            // `max_votes` above the number of real options.
            //
            // The builder refuses it, and it is worth its own message rather than
            // being folded into the arithmetic below: "choose up to 5" on a contest
            // with three candidates is usually a contest somebody trimmed without
            // revisiting the rule, and saying which two numbers disagree is what
            // makes it a one-line fix.
            if choices > 0 && contest.max_votes as usize > choices {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        &at,
                        format!(
                            "a voter may choose up to {} but there are only \
                             {choices} to choose from",
                            contest.max_votes
                        ),
                    )
                    .about(Some(&contest.external_id))
                    .id("contest.chooses-more-than-offered")
                    .detail("chosen", contest.max_votes)
                    .detail("offered", choices),
                );
            }

            if contest.winners < 1 {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        &at,
                        "the contest elects nobody",
                    )
                    .about(Some(&contest.external_id))
                    .id("contest.elects-nobody"),
                );
            }

            // The bug the TypeScript shipped: winners was fixed at 1 while
            // max_votes was free, so "choose 3" quietly elected one person.
            if contest.winners > contest.max_votes {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        &at,
                        format!(
                            "the contest elects {} but a voter may only choose {}",
                            contest.winners, contest.max_votes
                        ),
                    )
                    .about(Some(&contest.external_id))
.id("contest.elects-more-than-chosen").detail("winners", contest.winners).detail("chosen", contest.max_votes),
                );
            }

            for area in &contest.areas {
                if !plan
                    .areas
                    .iter()
                    .any(|planned| &planned.external_id == area)
                {
                    report.push(
                        Problem::error(
                            Code::DanglingReference,
                            format!("{at}.areas"),
                            format!("no area has the identifier '{area}'"),
                        )
                        .about(Some(&contest.external_id))
                        .id("contest.area-unknown")
                        .detail("area", area),
                    );
                }
            }

            if choices == 0 {
                report.push(
                    Problem::warning(
                        Code::BallotCoverage,
                        &at,
                        "no candidates yet",
                    )
                    .about(Some(&contest.external_id))
                    .id("contest.no-candidates"),
                );
            } else if (contest.winners as usize) > choices {
                report.push(
                    Problem::error(
                        Code::ContestArithmetic,
                        &at,
                        format!(
                            "the contest elects {} from a field of {choices}",
                            contest.winners
                        ),
                    )
                    .about(Some(&contest.external_id))
                    .id("contest.elects-more-than-standing")
                    .detail("winners", contest.winners)
                    .detail("standing", choices),
                );
            }
        }
    }
}

/// Turn a plan into the rows the builder reads.
///
/// This is the only mapping in the architect, and it produces a
/// [`Workbook`] rather than a bundle so that everything downstream — templates,
/// ids, CSV shapes, the realm, the archive — is the code the workbook reader
/// already uses.
pub fn to_workbook(plan: &Blueprint) -> Result<Workbook, Problem> {
    let languages = plan.languages_or_english();

    let mut sheets = vec![
        event_sheet(plan, &languages)?,
        elections_sheet(plan, &languages)?,
        contests_sheet(plan, &languages)?,
        candidates_sheet(plan, &languages)?,
        areas_sheet(plan)?,
        area_contests_sheet(plan)?,
    ];

    if let Some(voters) = voters_sheet(plan)? {
        sheets.push(voters);
    }

    if let Some(materials) = materials_sheet(plan, &languages) {
        sheets.push(materials?);
    }

    if let Some(schedule) = scheduled_events_sheet(plan)? {
        sheets.push(schedule);
    }

    // What only a plan holds. `build` never reads these.
    for maybe in [
        contacts_sheet(plan),
        trustees_sheet(plan),
        ceremony_sheet(plan),
        milestones_sheet(plan),
        messages_sheet(plan, &languages),
        notes_sheet(plan),
    ] {
        if let Some(sheet) = maybe {
            sheets.push(sheet?);
        }
    }

    // The sign-in page's wording, which is a *parameter* rather than a sheet of
    // its own — see `parameters_sheet`. Emitted before the carried sheets and
    // excluded from them below, because a plan opened from a workbook already has
    // that sheet and `Workbook::new` refuses a duplicate key.
    let parameters = match parameters_sheet(plan) {
        Some(Ok(sheet)) => Some(sheet),
        Some(Err(why)) => return Err(why),
        None => None,
    };
    let replaced = parameters.is_some();
    if let Some(sheet) = parameters {
        sheets.push(sheet);
    }

    // Last, and untouched. These are the sheets the wizard has no screens for,
    // carried through so `build` can do to them exactly what it does to a
    // janitor's own file. `Workbook::new` refuses a duplicate key, so a plan that
    // somehow carried a second ElectionEvent is a refusal rather than a silent
    // choice between two.
    //
    // `parameters` is the one exception, and only when there is wording to add:
    // the copy pushed above *is* the carried sheet with rows appended, so passing
    // the original through as well would be that refusal.
    sheets.extend(
        plan.platform
            .iter()
            .filter(|sheet| !(replaced && sheet.key == SHEET_PARAMETERS))
            .cloned(),
    );

    Workbook::new(sheets)
}

impl Blueprint {
    /// The languages to write, never empty.
    ///
    /// A plan with no language still has to produce a ballot somebody can read.
    fn languages_or_english(&self) -> Vec<String> {
        let chosen: Vec<String> = self
            .languages
            .iter()
            .map(|code| code.trim().to_string())
            .filter(|code| !code.is_empty())
            .collect();
        if chosen.is_empty() {
            vec!["en".to_string()]
        } else {
            chosen
        }
    }
}

/// A header and its column of values, as the sheet reader wants them.
/// Build one sheet, and refuse a ragged one.
///
/// Every sheet in this file is a `Vec<String>` of column names built in one place
/// and a `Vec<Cell>` per row built in another, and nothing tied the two together:
/// a column added without its value — or the reverse — shifted every later cell
/// one place left, so `presentation.sort_order` was read as a language code and
/// the failure surfaced as "no entry found for key" in an unrelated test.
///
/// That happened once while adding `elections_order`. The two lists are still
/// separate, because pairing them would mean a `Vec<(String, Cell)>` per row and
/// the column names repeated on every row; this catches the mistake at the one
/// point every sheet passes through instead.
fn sheet_of(
    name: &str,
    columns: Vec<String>,
    rows: Vec<Vec<Cell>>,
) -> Result<Sheet, Problem> {
    for (index, row) in rows.iter().enumerate() {
        if row.len() != columns.len() {
            return Err(Problem::error(
                Code::ConflictingColumns,
                format!("{name}[{index}]"),
                format!(
                    "{} cells under {} columns — a column was added without its \
                     value, or a value without its column",
                    row.len(),
                    columns.len()
                ),
            ));
        }
    }

    let mut grid =
        vec![columns.iter().map(|c| Cell::text(c.clone())).collect()];
    grid.extend(rows);
    Sheet::from_grid(name, &grid)
}

/// `presentation.i18n.<lang>.<field>` columns, one per language.
fn i18n_columns(
    prefix: &str,
    field: &str,
    languages: &[String],
) -> Vec<String> {
    languages
        .iter()
        .map(|language| format!("{prefix}.i18n.{language}.{field}"))
        .collect()
}

/// A `Translated`, or the plain string an older plan carried.
///
/// `PlannedContest.description` was a `String` for one release. A plan saved then
/// has `"description": "One seat"` where a map now belongs, and serde would refuse
/// it — so a client who saved a plan and came back would be told their own file was
/// not a plan.
///
/// Read into English, which is what the flat column meant.
fn translated_or_plain<'de, D>(input: D) -> Result<Translated, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        Plain(String),
        ByLanguage(Translated),
    }

    Ok(match Either::deserialize(input)? {
        Either::ByLanguage(text) => text,
        Either::Plain(text) if text.is_empty() => Translated::default(),
        Either::Plain(text) => Translated::new(&text),
    })
}

/// The English text, for the flat column beside the i18n block.
///
/// Both are written because the Admin Portal keeps both in step and its list
/// views read the flat one. `Translated::get` falls back, so a plan that lists no
/// English still puts something there rather than a blank.
fn english_of(text: &Translated) -> Cell {
    match text.get("en") {
        Some(value) => Cell::text(value),
        None => Cell::Blank,
    }
}

fn i18n_values(text: &Translated, languages: &[String]) -> Vec<Cell> {
    languages
        .iter()
        .map(|language| match text.get(language) {
            Some(value) => Cell::text(value),
            None => Cell::Blank,
        })
        .collect()
}

fn event_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Result<Sheet, Problem> {
    let mut columns = vec!["external_id".to_string()];
    // **Before** the named `presentation.i18n.<lang>.name` columns, and the
    // order is load-bearing. `set_path` inserts rather than merges, so this
    // whole-object column written *after* them would replace the object it had
    // just filled and take the event's own name and description with it. First,
    // the object exists and the named columns descend into it.
    columns.extend(
        languages
            .iter()
            .map(|language| format!("presentation.i18n.{language}")),
    );
    columns.extend(i18n_columns("presentation", "name", languages));
    columns.extend(i18n_columns("presentation", "description", languages));
    // The flat column too, mirroring the English one. The Admin Portal keeps
    // both in step — `newEvent.description = presentation.i18n.en.description` —
    // and a bundle with only the i18n block leaves every list view blank.
    columns.push("description".to_string());
    columns.push("presentation.css".to_string());
    columns.push("presentation.elections_order".to_string());
    columns.push("presentation.show_cast_vote_logs".to_string());
    columns
        .push("presentation.language_conf.enabled_language_codes".to_string());
    columns
        .push("presentation.language_conf.default_language_code".to_string());

    let mut row = vec![Cell::text(plan.external_id.clone())];
    // A JSON object per language, which `coerce_scalar` parses because it is
    // bracketed. One column per *key* is not available: `split_path` splits on
    // `.` and portal keys are dotted — `candidate.preferential.none` would
    // become three levels of nesting nobody asked for.
    row.extend(languages.iter().map(|language| {
        let written = plan.i18n.get(language);
        match written {
            Some(keys) if !keys.is_empty() => Cell::text(
                serde_json::to_string(keys)
                    .unwrap_or_else(|_| "{}".to_string()),
            ),
            // Empty rather than `{}`: a blank cell is skipped by the reader, and
            // writing `{}` would replace whatever a base export already had.
            _ => Cell::text(String::new()),
        }
    }));
    row.extend(i18n_values(&plan.name, languages));
    row.extend(i18n_values(&plan.description, languages));
    row.push(english_of(&plan.description));
    row.push(Cell::text(plan.css.clone()));
    row.push(Cell::text(plan.elections_order.clone()));
    row.push(Cell::text(plan.show_cast_vote_logs.clone()));
    // A JSON array in one cell: the reader parses bracketed text as JSON, which is
    // how a list fits in a spreadsheet and therefore in a synthesised one too.
    row.push(Cell::text(
        serde_json::to_string(languages).unwrap_or_else(|_| "[]".to_string()),
    ));
    // The chosen default, or the first language when nobody chose. A default
    // naming a language the ballot is not offered in is refused by
    // `check_languages` rather than written here, so this cannot emit one.
    row.push(Cell::text(
        plan.default_language
            .clone()
            .filter(|chosen| languages.iter().any(|each| each == chosen))
            .or_else(|| languages.first().cloned())
            .unwrap_or_else(|| "en".to_string()),
    ));

    for (column, cell) in plan.voting_channels.columns_for_sheet() {
        columns.push(column.to_string());
        row.push(cell);
    }

    // The file, if there is one, wins over a typed link. Named rather than embedded,
    // the way the Materials sheet names its documents: a cell cannot hold bytes, so
    // the sheet says `logo.png` and the file travels beside the workbook. `build.rs`
    // matches the name, derives the identifier and composes the url — the url embeds
    // the tenant, and the builder is the only place that knows which one this bundle
    // will carry.
    if let Some(logo) = plan.logo.as_ref().filter(|file| !file.bytes.is_empty())
    {
        columns.push("presentation.logo_file".to_string());
        row.push(Cell::text(logo.file_name.clone()));
    } else if let Some(logo) =
        plan.logo_url.as_ref().filter(|url| !url.is_empty())
    {
        columns.push("presentation.logo_url".to_string());
        row.push(Cell::text(logo.clone()));
    }

    // Each of these is written only when the plan says something, so a plan made
    // before the field existed compiles to the bytes it used to: `election_event.hbs`
    // already carries a value for every one of them, and an absent column leaves the
    // template's own.
    if let Some(policy) = plan
        .language_detection_policy
        .as_ref()
        .filter(|each| !each.is_empty())
    {
        columns.push(
            "presentation.language_conf.language_detection_policy".to_string(),
        );
        row.push(Cell::text(policy.clone()));
    }

    if let Some(skip) = plan.skip_election_list {
        columns.push("presentation.skip_election_list".to_string());
        row.push(Cell::Bool(skip));
    }

    if let Some(show) = plan.show_user_profile {
        columns.push("presentation.show_user_profile".to_string());
        row.push(Cell::Bool(show));
    }

    if let Some(activated) = plan.materials_activated {
        columns.push("presentation.materials.activated".to_string());
        row.push(Cell::Bool(activated));
    }

    // Per language, like the name and the description. Only when something is
    // written: a title in no language is a heading over an empty tab.
    if !plan.materials_title.is_empty() {
        columns.extend(i18n_columns(
            "presentation",
            "materialsTitle",
            languages,
        ));
        row.extend(i18n_values(&plan.materials_title, languages));
    }
    if !plan.materials_subtitle.is_empty() {
        columns.extend(i18n_columns(
            "presentation",
            "materialsSubtitle",
            languages,
        ));
        row.extend(i18n_values(&plan.materials_subtitle, languages));
    }

    sheet_of("ElectionEvent", columns, vec![row])
}

/// The support materials, as rows a workbook could equally have carried.
///
/// The wizard's documents and a janitor's spreadsheet now describe the same thing
/// in the same place: the sheet names the file, and the bytes travel beside it.
/// Before this the wizard emitted rows directly and a workbook could not carry a
/// material at all, which is two routes to one entity and the usual result.
fn materials_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Option<Result<Sheet, Problem>> {
    if plan.materials.is_empty() {
        return None;
    }

    let mut columns = vec![
        "external_id".to_string(),
        "kind".to_string(),
        "file".to_string(),
        "is_hidden".to_string(),
    ];
    columns.extend(i18n_columns("presentation", "title", languages));

    let rows = plan
        .materials
        .iter()
        .map(|material| {
            let mut row = vec![
                Cell::text(material.external_id.clone()),
                Cell::text(if material.kind.is_empty() {
                    "document".to_string()
                } else {
                    material.kind.clone()
                }),
                Cell::text(material.file_name.clone()),
                Cell::Bool(material.is_hidden),
            ];
            row.extend(i18n_values(&material.title, languages));
            row
        })
        .collect();

    Some(sheet_of("Materials", columns, rows))
}

fn elections_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Result<Sheet, Problem> {
    let mut columns = vec!["external_id".to_string()];
    columns.extend(i18n_columns("presentation", "name", languages));
    columns.extend(i18n_columns("presentation", "description", languages));
    columns.push("description".to_string());
    columns.push("num_allowed_revotes".to_string());
    columns.push("spoil_ballot_option".to_string());
    columns.push("permission_label".to_string());
    columns.push("presentation.grace_period_policy".to_string());
    columns.push("presentation.grace_period_secs".to_string());
    columns.push("presentation.start_screen_title_policy".to_string());
    columns.push("presentation.contests_order".to_string());
    // The event's set, on every election. Named once here and filled from the
    // same method below so the two cannot drift apart.
    let channels = plan.voting_channels.columns_for_sheet();
    columns.extend(channels.iter().map(|(column, _)| (*column).to_string()));

    let rows = plan
        .elections
        .iter()
        .map(|election| {
            let mut row = vec![Cell::text(election.external_id.clone())];
            row.extend(i18n_values(&election.name, languages));
            row.extend(i18n_values(&election.description, languages));
            row.push(english_of(&election.description));
            row.push(Cell::Int(election.num_allowed_revotes));
            row.push(Cell::Bool(election.spoil_ballot_option));
            // Blank rather than an empty string, so the builder derives one
            // instead of setting the label to "".
            row.push(if election.permission_label.is_empty() {
                Cell::Blank
            } else {
                Cell::text(election.permission_label.clone())
            });
            row.push(Cell::text(election.grace_period_policy.clone()));
            row.push(Cell::Int(election.grace_period_secs));
            row.push(Cell::text(election.start_screen_title_policy.clone()));
            row.push(Cell::text(election.contests_order.clone()));
            row.extend(channels.iter().map(|(_, cell)| cell.clone()));
            row
        })
        .collect();

    sheet_of("Elections", columns, rows)
}

fn contests_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Result<Sheet, Problem> {
    // The behaviour columns come from the policy module rather than being
    // listed here, so a new policy is one declaration rather than a column, a
    // cell and two places to forget.
    let behaviour = Behaviour::default().columns();

    let mut columns = vec![
        "external_id".to_string(),
        "election.external_id".to_string(),
        "max_votes".to_string(),
        "winning_candidates_num".to_string(),
        "description".to_string(),
        "presentation.sort_order".to_string(),
        "presentation.allow_writeins".to_string(),
    ];
    columns.extend(i18n_columns("presentation", "description", languages));
    columns.extend(behaviour.iter().map(|(column, _)| (*column).to_string()));
    columns.extend(i18n_columns("presentation", "name", languages));

    let mut rows = Vec::new();
    for election in &plan.elections {
        for (order, contest) in election.contests.iter().enumerate() {
            let mut row = vec![
                Cell::text(contest.external_id.clone()),
                Cell::text(election.external_id.clone()),
                Cell::Int(contest.max_votes),
                Cell::Int(contest.winners),
                english_of(&contest.description),
                Cell::Int(order as i64),
                Cell::Bool(contest.allow_writeins),
            ];
            row.extend(i18n_values(&contest.description, languages));
            row.extend(
                resolve(plan, election, contest)
                    .columns()
                    .into_iter()
                    .map(|(_, cell)| cell),
            );
            row.extend(i18n_values(&contest.name, languages));
            rows.push(row);
        }
    }

    sheet_of("Contests", columns, rows)
}

fn candidates_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Result<Sheet, Problem> {
    let mut columns = vec![
        "external_id".to_string(),
        "contest.external_id".to_string(),
        "presentation.sort_order".to_string(),
        "presentation.is_explicit_blank".to_string(),
        "presentation.is_explicit_invalid".to_string(),
        // Drawn but not selectable. The `candidate.hbs` template already defaults it
        // to `false`, which is what makes this a column rather than a format change:
        // the platform's shape has carried `is_disabled` all along and the sheet was
        // simply never asked to fill it in.
        "presentation.is_disabled".to_string(),
        "presentation.is_write_in".to_string(),
        // The photograph's half that lives in the JSON. Its `presentation.urls`
        // twin is composed by the builder rather than written here, because the url
        // embeds the tenant and `resolve_tenant_id` draws on the explicit option,
        // the Parameters sheet, the base export and the id-factory fallback — so
        // the builder is the only place that knows which one the bundle will carry.
        "image_document_id".to_string(),
    ];
    columns.extend(i18n_columns("presentation", "name", languages));
    columns.extend(i18n_columns("presentation", "description", languages));
    columns.push("description".to_string());

    // `None` only when the event has no `external_id`, which `check_identity`
    // reports before this runs. An image on such a plan simply gets no identifier.
    let ids = IdFactory::new(&plan.external_id);

    let mut rows = Vec::new();
    for election in &plan.elections {
        for contest in &election.contests {
            for (order, candidate) in contest.candidates.iter().enumerate() {
                let mut row = vec![
                    Cell::text(candidate.external_id.clone()),
                    Cell::text(contest.external_id.clone()),
                    Cell::Int(order as i64),
                    Cell::Bool(candidate.explicit_blank),
                    Cell::Bool(candidate.explicit_invalid),
                    Cell::Bool(candidate.disabled),
                    Cell::Bool(false),
                    match (&candidate.image, ids.as_ref()) {
                        (Some(_), Some(ids)) => Cell::text(image_document_id(
                            ids,
                            &candidate.external_id,
                        )),
                        _ => Cell::Blank,
                    },
                ];
                row.extend(i18n_values(&candidate.name, languages));
                row.extend(i18n_values(&candidate.description, languages));
                row.push(english_of(&candidate.description));
                rows.push(row);
            }

            // One row per write-in slot, after the real candidates.
            //
            // Minted here rather than kept in the plan, because a write-in slot
            // is not a candidate anybody named: it is a blank line, and the
            // number of them is the whole decision. Keeping them in
            // `contest.candidates` would put empty rows in the ballot's own list
            // and make "how many candidates are standing" wrong.
            if contest.allow_writeins {
                let start = contest.candidates.len();
                for slot in 0..contest.write_in_slots {
                    let mut row = vec![
                        Cell::text(format!(
                            "{}-write-in-{}",
                            contest.external_id,
                            slot + 1
                        )),
                        Cell::text(contest.external_id.clone()),
                        Cell::Int((start as i64) + slot),
                        Cell::Bool(false),
                        Cell::Bool(false),
                        // Never disabled: a slot nobody can write in is a slot that
                        // should not have been asked for.
                        Cell::Bool(false),
                        Cell::Bool(true),
                        // A write-in slot is a blank line, not a person, so it has
                        // no photograph. The cell is still written: the sheet guard
                        // refuses a ragged row, and it is right to.
                        Cell::Blank,
                    ];
                    // Named, because the Voting Portal draws the name as the
                    // slot's label and an unnamed one is a blank box with no
                    // indication of what it is for.
                    let mut named = Translated::default();
                    for language in languages {
                        named.by_language.insert(
                            language.clone(),
                            format!("Write-in {}", slot + 1),
                        );
                    }
                    row.extend(i18n_values(&named, languages));
                    // A slot has no description; the columns still have to line
                    // up or the sheet is ragged and `build_tables` reads the
                    // wrong value into the wrong field.
                    row.extend(i18n_values(&Translated::default(), languages));
                    row.push(Cell::Blank);
                    rows.push(row);
                }
            }
        }
    }

    sheet_of("Candidates", columns, rows)
}

/// The plan's areas, or the one that covers everybody.
///
/// A plan that has never thought about districting still needs an area and a
/// ballot link, or no voter sees anything. See [`DEFAULT_AREA_EXTERNAL_ID`].
/// The census, when the plan carries one.
///
/// `None` rather than an empty sheet when it does not: `build_tables` treats a
/// present-but-empty Voters sheet as "this election has no voters", which is a
/// different claim from "this plan does not carry the census" and produces a
/// bundle that imports an election nobody can vote in.
///
/// Columns are `VOTER_LEADING_COLUMNS` minus the two the builder derives — `id`
/// comes from `ids::uid` and `authorized-election-ids` from the areas — plus
/// whatever else the client carries, in a stable order so two builds of one
/// census diff cleanly. Everything unrecognised is passed through, which is how
/// a reporting breakout column survives the round trip.
fn voters_sheet(plan: &Blueprint) -> Result<Option<Sheet>, Problem> {
    if plan.voters.is_empty() {
        return Ok(None);
    }

    /// The columns `PlannedVoter` names in its own right.
    ///
    /// Spelled here rather than borrowed from `build_tables::VOTER_LEADING_COLUMNS`
    /// because that module sits behind `election_config_templates` and this one
    /// does not — reaching for it would drag the whole builder into a front end
    /// that only wants to describe a plan. The two lists are checked against
    /// each other by `the_voters_sheet_matches_what_the_builder_reads`.
    ///
    /// **`area.external_id` and no `area_name`.** The sheet used to carry both, and
    /// each end read a different one: reopening a plan read `area_name`, the builder
    /// read `area.external_id`, and the writer derived the second from the first by
    /// matching names. So a voter's area went name → id → name on one round trip,
    /// nothing said which column was authoritative, and editing the wrong one of the
    /// two did nothing at all — the worst kind of silent, because the spreadsheet
    /// looked like it had been corrected.
    ///
    /// `build_tables::voter_area_name` still turns the identifier back into a name
    /// for the finished CSV, because that is what the platform's importer matches on
    /// ([`PlannedArea::name`]). That translation belongs there, at the boundary,
    /// rather than in the authoring format.
    const NAMED: &[&str] = &[
        "username",
        "email",
        "first_name",
        "last_name",
        "area.external_id",
    ];

    // Sorted, so a census that gains a column does not reorder the ones it had.
    let mut extra: Vec<&str> = plan
        .voters
        .iter()
        .flat_map(|voter| voter.extra.keys().map(String::as_str))
        .filter(|key| {
            !NAMED.contains(key)
                && !["id", "enabled", "email_verified"].contains(key)
                && *key != "area.external_id"
                && *key != "authorized-election-ids"
        })
        .collect();
    extra.sort_unstable();
    extra.dedup();

    let mut columns: Vec<String> =
        NAMED.iter().map(|name| (*name).to_string()).collect();
    columns.extend(extra.iter().map(|key| (*key).to_string()));

    let rows = plan
        .voters
        .iter()
        .map(|voter| {
            let mut row = vec![
                Cell::text(voter.username.clone()),
                text_or_blank(&voter.email),
                text_or_blank(&voter.first_name),
                text_or_blank(&voter.last_name),
                text_or_blank(&voter.area_external_id),
            ];
            row.extend(extra.iter().map(|key| {
                voter
                    .extra
                    .get(*key)
                    .map(|value| Cell::text(value.clone()))
                    .unwrap_or(Cell::Blank)
            }));
            row
        })
        .collect();

    sheet_of("Voters", columns, rows).map(Some)
}

// -- the sheets the plan holds and the importer does not ------------------------
//
// Six sheets `build` never reads. They are in `KNOWN_SHEETS` so a workbook
// carrying them does not report six renamed tabs, and they exist so the
// spreadsheet inside a delivery is the *whole* plan rather than the importable
// part of it — the part a delivery engineer has to email around otherwise.
//
// Each returns `None` when the plan says nothing, because an empty sheet is a tab
// somebody opens and closes again.

fn contacts_sheet(plan: &Blueprint) -> Option<Result<Sheet, Problem>> {
    if plan.contacts.is_empty() {
        return None;
    }
    let columns =
        vec!["name".to_string(), "role".to_string(), "email".to_string()];
    let rows: Vec<Vec<Cell>> = plan
        .contacts
        .iter()
        .map(|contact| {
            vec![
                text_or_blank(&contact.name),
                text_or_blank(&contact.role),
                text_or_blank(&contact.email),
            ]
        })
        .collect();
    Some(sheet_of("Contacts", columns, rows))
}

fn trustees_sheet(plan: &Blueprint) -> Option<Result<Sheet, Problem>> {
    if plan.trustees.is_empty() {
        return None;
    }
    let columns = vec!["name".to_string(), "email".to_string()];
    let rows: Vec<Vec<Cell>> = plan
        .trustees
        .iter()
        .map(|trustee| {
            vec![text_or_blank(&trustee.name), text_or_blank(&trustee.email)]
        })
        .collect();
    Some(sheet_of("Trustees", columns, rows))
}

/// `key | value`, not a column per setting.
///
/// Load-bearing, and the reason is `build`: every non-control column on the
/// ElectionEvent row is deep-merged onto the event template as a dotted path, so a
/// `trustee_threshold` column there would land inside the exported event JSON. A
/// sheet of its own cannot leak.
///
/// A [`Timestamp`] is three rows rather than one, because it is three things — the
/// wall clock somebody typed, the zone they were in, and the offset that was
/// resolved at that moment. Writing only the first would make a ceremony drift by
/// an hour when the file is reopened somewhere else.
///
/// The **voting window's zone** is here too, which looks out of place on a sheet
/// called Ceremony and is not. That window goes to the platform's own
/// ScheduledEvents sheet as RFC3339, because its scheduler parses it that way — and
/// RFC3339 carries an *offset*, `-08:00`, not a zone *name*. The instant survives
/// the round trip either way; `America/Los_Angeles` does not, and without it a
/// reopened plan says "09:00" where it used to say "09:00 — Los Angeles time", and
/// a later edit resolves its offset against the wrong place. This sheet is the
/// wizard's own, so it is where anything the platform's sheets cannot express goes.
fn ceremony_sheet(plan: &Blueprint) -> Option<Result<Sheet, Problem>> {
    let mut rows: Vec<Vec<Cell>> = vec![
        vec![
            Cell::text("threshold"),
            Cell::Int(i64::from(plan.trustee_threshold)),
        ],
        vec![
            Cell::text("policy"),
            Cell::text(plan.ceremony_policy.to_string()),
        ],
    ];

    for (name, when) in [
        ("key_ceremony", &plan.schedule.key_ceremony),
        ("tally_ceremony", &plan.schedule.tally_ceremony),
    ] {
        if let Some(stamp) = when {
            rows.push(vec![
                Cell::text(name.to_string()),
                Cell::text(stamp.local.clone()),
            ]);
            if !stamp.zone.is_empty() {
                rows.push(vec![
                    Cell::text(format!("{name}.zone")),
                    Cell::text(stamp.zone.clone()),
                ]);
            }
            rows.push(vec![
                Cell::text(format!("{name}.offset_minutes")),
                Cell::Int(i64::from(stamp.offset_minutes)),
            ]);
        }
    }

    // The zone name only: the wall clock and the offset are both in the RFC3339
    // value on ScheduledEvents, and writing them twice would let two copies of one
    // fact disagree.
    for (name, when) in [
        ("voting_opens", &plan.schedule.voting_opens),
        ("voting_closes", &plan.schedule.voting_closes),
    ] {
        if let Some(stamp) = when {
            if !stamp.zone.is_empty() {
                rows.push(vec![
                    Cell::text(format!("{name}.zone")),
                    Cell::text(stamp.zone.clone()),
                ]);
            }
        }
    }

    let columns = vec!["key".to_string(), "value".to_string()];
    Some(sheet_of("Ceremony", columns, rows))
}

fn milestones_sheet(plan: &Blueprint) -> Option<Result<Sheet, Problem>> {
    if plan.schedule.milestones.is_empty() {
        return None;
    }
    let columns = vec!["event".to_string(), "date".to_string()];
    let rows: Vec<Vec<Cell>> = plan
        .schedule
        .milestones
        .iter()
        .map(|milestone| {
            vec![
                text_or_blank(&milestone.event),
                text_or_blank(&milestone.date),
            ]
        })
        .collect();
    Some(sheet_of("Milestones", columns, rows))
}

/// One row per message, in every language the ballot offers.
///
/// `schedule` is a JSON cell, which is the format's own convention for a structured
/// value — `presentation.language_conf.enabled_language_codes` is `["en","es"]` in
/// one cell for the same reason. A send schedule is a list of timestamps each
/// carrying a zone and an offset; spreading that across parallel `||` columns would
/// be exact only while every send shared a zone, and silently wrong the day one did
/// not.
fn messages_sheet(
    plan: &Blueprint,
    languages: &[String],
) -> Option<Result<Sheet, Problem>> {
    if plan.messages.is_empty() {
        return None;
    }

    let mut columns = vec!["kind".to_string()];
    for part in ["subject", "body", "html"] {
        for language in languages {
            columns.push(format!("presentation.i18n.{language}.{part}"));
        }
    }
    columns.push("schedule".to_string());

    let rows: Vec<Vec<Cell>> = plan
        .messages
        .iter()
        .map(|message| {
            let mut row = vec![Cell::text(message.kind.alias().to_string())];
            for translated in [&message.subject, &message.body, &message.html] {
                for language in languages {
                    row.push(text_or_blank(
                        translated.get(language).unwrap_or(""),
                    ));
                }
            }
            row.push(
                serde_json::to_string(&message.schedule)
                    .map(Cell::text)
                    .unwrap_or(Cell::Blank),
            );
            row
        })
        .collect();

    Some(sheet_of("Messages", columns, rows))
}

/// The sign-in page's wording, as realm parameters.
///
/// `keycloak_event_realm.localizationTexts.<locale>.<key>`, one row each, which is
/// a prefix `PARAMETER_PREFIXES` already carries into the realm patch — so this
/// emitter is the whole of the feature on the build side and `build_realm.rs` did
/// not change.
///
/// Returns nothing when the plan says nothing, and that matters more than it looks:
/// `parameters` is one of the sheets a plan carries through from a workbook it was
/// opened from, and `Workbook::new` refuses a duplicate key. A plan with no
/// sign-in wording therefore emits no sheet at all, so opening a janitor's workbook
/// and rebuilding it is byte-for-byte what it was before this existed. Where the
/// plan *does* have wording and *did* come from such a workbook, the rows are
/// merged into that sheet by {@link merge_parameters} rather than added beside it.
fn keycloak_message_rows(plan: &Blueprint) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for (locale, messages) in &plan.keycloak_messages {
        if locale.trim().is_empty() {
            continue;
        }
        for (key, text) in messages {
            if key.trim().is_empty() {
                continue;
            }
            // A stylesheet is escaped; a sentence is not.
            //
            // Keycloak resolves every `localizationTexts` value through
            // `java.text.MessageFormat`, where `{` opens a placeholder and `'`
            // quotes. That is harmless for wording — and fatal for CSS, which
            // is mostly braces. `login_css_patch` already escapes on the
            // workbook path; doing it here keeps the two paths agreeing about
            // one key rather than about one entry point.
            //
            // Only this key. Escaping every message would break a translation
            // that legitimately carries `{0}`, which is the whole reason the
            // rule is per-key rather than per-channel.
            let value = if key.trim() == super::branding::LOGIN_CUSTOM_CSS_KEY {
                super::branding::escape_message_format(
                    super::branding::unwrap_quoted(text),
                )
            } else {
                text.clone()
            };
            rows.push((
                format!(
                    "keycloak_event_realm.localizationTexts.{}.{}",
                    locale.trim(),
                    key.trim()
                ),
                value,
            ));
        }
    }
    rows
}

/// The parameters sheet the plan carries, with the sign-in wording added to it.
///
/// Two shapes to reconcile: a plan opened from a workbook brings that workbook's
/// own `parameters` sheet through `platform`, and a plan authored in the wizard has
/// none. Either way the result is exactly one parameters sheet, because two would
/// be refused — and refusing a plan somebody assembled by opening a real workbook
/// and adding a translation would be the worst possible time to find that out.
///
/// The carried sheet's own columns are kept. Its rows are `key`/`value`/`type`, so
/// the added rows are laid out to match by position, and a sheet whose columns are
/// in another order is left alone with the wording appended by header name.
fn parameters_sheet(plan: &Blueprint) -> Option<Result<Sheet, Problem>> {
    let rows = keycloak_message_rows(plan);
    if rows.is_empty() {
        return None;
    }

    // The sheet a plan opened from a workbook brought with it, if any.
    let carried = plan
        .platform
        .iter()
        .find(|sheet| sheet.key == SHEET_PARAMETERS);

    let Some(carried) = carried else {
        return Some(sheet_of(
            "Parameters",
            vec!["key".to_string(), "value".to_string()],
            rows.into_iter()
                .map(|(key, value)| vec![Cell::text(key), Cell::text(value)])
                .collect(),
        ));
    };

    // Appended to what the workbook already said, keyed by the headers that sheet
    // uses. `Row.cells` holds only non-blank cells, keyed by raw header, so there
    // is nothing to pad and no column order to guess at.
    let mut merged = carried.clone();
    let key_header = merged
        .headers
        .iter()
        .find(|header| header.as_str() == "key")
        .cloned();
    let value_header = merged
        .headers
        .iter()
        .find(|header| header.as_str() == "value")
        .cloned();
    let (Some(key_header), Some(value_header)) = (key_header, value_header)
    else {
        // A `parameters` sheet with no `key` column is not one the builder reads
        // either, and quietly dropping the wording is the failure this file spends
        // most of its comments arguing against.
        return Some(Err(Problem::error(
            Code::MissingField,
            "keycloak_messages",
            "the workbook this plan came from has a Parameters sheet without \
             `key` and `value` columns, so the sign-in page's wording has \
             nowhere to go. Remove that sheet or give it those columns.",
        )));
    };

    let mut number =
        merged.rows.iter().map(|row| row.number).max().unwrap_or(1);
    for (key, value) in rows {
        number += 1;
        merged.rows.push(Row {
            sheet: merged.name.clone(),
            number,
            cells: vec![
                (key_header.clone(), serde_json::Value::String(key)),
                (value_header.clone(), serde_json::Value::String(value)),
            ],
        });
    }
    Some(Ok(merged))
}

fn notes_sheet(plan: &Blueprint) -> Option<Result<Sheet, Problem>> {
    if plan.notes.trim().is_empty() {
        return None;
    }
    Some(sheet_of(
        "Notes",
        vec!["notes".to_string()],
        vec![vec![Cell::text(plan.notes.clone())]],
    ))
}

/// A cell, or nothing at all — so a blank stays distinguishable from `""`.
fn text_or_blank(value: &str) -> Cell {
    if value.is_empty() {
        Cell::Blank
    } else {
        Cell::text(value.to_string())
    }
}

fn areas_sheet(plan: &Blueprint) -> Result<Sheet, Problem> {
    let columns = vec![
        "external_id".to_string(),
        "name".to_string(),
        "parent.external_id".to_string(),
        "presentation.allow_early_voting".to_string(),
    ];

    // The synthesised single area, when the plan describes no districting. It gets
    // `no_early_voting` like every other area that did not ask for it: a plan with
    // no areas has nowhere to have asked.
    if plan.areas.is_empty() {
        return sheet_of(
            "Areas",
            columns,
            vec![vec![
                Cell::text(DEFAULT_AREA_EXTERNAL_ID),
                Cell::text(DEFAULT_AREA_NAME),
                Cell::Blank,
                Cell::text(NO_EARLY_VOTING),
            ]],
        );
    }

    let rows = plan
        .areas
        .iter()
        .map(|area| {
            vec![
                Cell::text(area.external_id.clone()),
                Cell::text(area.name.clone()),
                match &area.parent_external_id {
                    Some(parent) if !parent.is_empty() => {
                        Cell::text(parent.clone())
                    }
                    _ => Cell::Blank,
                },
                // Written either way rather than left blank for the negative
                // case. `area.hbs` already says `no_early_voting`, so a blank
                // would produce the same bytes — but it would mean the sheet
                // states the answer for one area and stays silent for the next,
                // and somebody reading the workbook could not tell "no" from
                // "unanswered".
                Cell::text(if area.allow_early_voting {
                    ALLOW_EARLY_VOTING
                } else {
                    NO_EARLY_VOTING
                }),
            ]
        })
        .collect();

    sheet_of("Areas", columns, rows)
}

/// Which contests appear on which area's ballot.
fn area_contests_sheet(plan: &Blueprint) -> Result<Sheet, Problem> {
    let every_area: Vec<String> = if plan.areas.is_empty() {
        vec![DEFAULT_AREA_EXTERNAL_ID.to_string()]
    } else {
        plan.areas
            .iter()
            .map(|area| area.external_id.clone())
            .collect()
    };

    let mut rows = Vec::new();
    for contest in plan
        .elections
        .iter()
        .flat_map(|election| &election.contests)
    {
        // An empty list means everywhere. A contest nobody assigned is one
        // somebody has not got to yet, and dropping it off every ballot would be
        // a silent way of losing it.
        let on: Vec<&String> = if contest.areas.is_empty() {
            every_area.iter().collect()
        } else {
            contest.areas.iter().collect()
        };

        for area in on {
            // A contest may not be listed twice for the same area: both rows
            // would mint the same id and one would overwrite the other.
            let link = vec![
                Cell::text(area.clone()),
                Cell::text(contest.external_id.clone()),
            ];
            if !rows.contains(&link) {
                rows.push(link);
            }
        }
    }

    sheet_of(
        "AreaContests",
        vec![
            "area.external_id".to_string(),
            "contest.external_id".to_string(),
        ],
        rows,
    )
}

/// The voting window, or nothing.
///
/// Event-wide rather than per election: the wizard asks once, and an event-wide
/// scheduled event covers every election in it.
fn scheduled_events_sheet(plan: &Blueprint) -> Result<Option<Sheet>, Problem> {
    let mut rows = Vec::new();

    for (name, processor, at) in [
        (
            "Voting opens",
            "START_VOTING_PERIOD",
            &plan.schedule.voting_opens,
        ),
        (
            "Voting closes",
            "END_VOTING_PERIOD",
            &plan.schedule.voting_closes,
        ),
    ] {
        if let Some(at) = at.as_ref().filter(|at| !at.is_empty()) {
            // An instant, not the wall clock somebody typed. The scheduler reads
            // this back through `DateTime::parse_from_rfc3339`, which requires an
            // offset — so a bare `2027-03-01T09:00` yields no date, the poller
            // drops the event, and the voting period never opens. Nothing on that
            // path reports anything.
            rows.push(vec![
                Cell::text(name),
                Cell::text(processor),
                Cell::text(at.to_rfc3339()?),
            ]);
        }
    }

    if rows.is_empty() {
        return Ok(None);
    }

    sheet_of(
        "ScheduledEvents",
        vec![
            "event_name".to_string(),
            "event_type".to_string(),
            "scheduled_datetime".to_string(),
        ],
        rows,
    )
    .map(Some)
}

/// The files the architect produces that are not part of an import.
///
/// A ceremony schedule, a list of who to call, and a list of who holds the key.
/// None of them has a home in an election event, and all three are what the client
/// actually asks for — so they travel beside the archive, like the administrator
/// and template files the workbook reader produces.
///
/// The plan itself is written out too. That is what makes the wizard resumable
/// without parsing its own output back, which is how the TypeScript version did it
/// and why its round trip lost the trustee threshold and every ceremony date.
pub fn side_files(plan: &Blueprint) -> Vec<(String, String)> {
    let mut files = Vec::new();

    let plan_json =
        serde_json::to_string_pretty(plan).unwrap_or_else(|_| "{}".to_string());
    files.push(("blueprint.json".to_string(), plan_json + "\n"));

    let ceremony = serde_json::json!({
        "_comment": "Dates people have to attend. Not part of the import.",
        "key_ceremony": plan.schedule.key_ceremony,
        "tally_ceremony": plan.schedule.tally_ceremony,
        "voting_opens": plan.schedule.voting_opens,
        "voting_closes": plan.schedule.voting_closes,
        "milestones": plan.schedule.milestones,
    });
    files.push(("ceremony_schedule.json".to_string(), pretty(&ceremony)));

    if !plan.contacts.is_empty() {
        files.push((
            "points_of_contact.json".to_string(),
            pretty(&serde_json::json!(plan.contacts)),
        ));
    }

    if !plan.trustees.is_empty() {
        files.push((
            "trustees_list.json".to_string(),
            pretty(&serde_json::json!({
                "threshold": plan.trustee_threshold,
                "trustees": plan.trustees,
            })),
        ));
    }

    if !plan.messages.is_empty() {
        // **Beside the bundle, not inside it.** The platform's own templates are a
        // property of the *tenant*, and an election event import that could rewrite
        // them would let one election change what every other election sends. So
        // this is an artifact somebody loads into the Admin Portal deliberately.
        //
        // Shaped as the Admin Portal's own template rows — an alias, a
        // communication method, and the text per language — rather than as this
        // plan's vocabulary, because the file is for that screen rather than for us.
        let templates: Vec<serde_json::Value> = plan
            .messages
            .iter()
            .map(|message| {
                serde_json::json!({
                    "alias": message.kind.alias(),
                    "communication_method": "email",
                    "template_type": "voter",
                    "subject": message.subject,
                    "body": message.body,
                    "body_html": message.html,
                })
            })
            .collect();
        files.push((
            "admin_portal/communication_templates.json".to_string(),
            pretty(&serde_json::json!({
                "_comment": "Load into the Admin Portal's tenant templates. \
                             Deliberately outside the import zip: templates belong \
                             to the tenant, not to one election event.",
                "templates": templates,
            })),
        ));

        // The other half, and the one that stops a client believing a schedule is
        // a promise. `scheduled_events.rs` handles `SEND_TEMPLATE` with an empty
        // arm, so nothing in the platform will send these — the file says so in
        // writing, because the screen that said it is not there when somebody opens
        // this zip three weeks later.
        let sends: Vec<serde_json::Value> = plan
            .messages
            .iter()
            .map(|message| {
                // Named one at a time rather than serialising the schedule
                // whole, which means every field added to `MessageSchedule` has
                // to be added here too or it is silently dropped from the file a
                // delivery engineer actually reads. `weekly_at` is the first one
                // to find that out. Left as it is on purpose — this file's shape
                // is something people script against — and recorded as its own
                // item rather than restructured in a change about the hour.
                serde_json::json!({
                    "alias": message.kind.alias(),
                    "on": message.schedule.on,
                    "weekly": message.schedule.weekly,
                    "weekly_at": message.schedule.weekly_at,
                })
            })
            .collect();
        files.push((
            "voter_messaging.json".to_string(),
            pretty(&serde_json::json!({
                "_comment": "Not part of the import, and not automatic. Nothing in \
                             the platform sends these: the scheduled-event processor \
                             for SEND_TEMPLATE does nothing. Somebody sends them, on \
                             the dates below.",
                "sends_automatically": false,
                "schedule": sends,
            })),
        ));
    }

    files
}

fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
        + "\n"
}

/// Everything a plan produces.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub bundle: Bundle,

    /// The files, split into what goes inside the importable archive and what
    /// must not. [`side_files`] is already folded into `auxiliary`.
    pub layout: Layout,

    /// Warnings from every pass. Errors are returned as `Err` instead, so a
    /// caller holding a `Compiled` is holding something worth writing out.
    pub report: Report,
}

/// Turn a plan into what it is for.
///
/// This is the function the wizard and the CLI both call, and until it existed
/// there was nothing joining the two halves of this module: [`to_workbook`] and
/// [`side_files`] had no callers outside the tests, so a plan could be validated
/// and mapped but never actually built.
///
/// Six steps, none of them new — which is the point. A plan becomes the same rows
/// a spreadsheet produces, and everything after that is the existing builder:
///
/// 1. [`validate_plan`], in the plan's own vocabulary. Errors stop here, because
///    a problem phrased as `contests[2].winning_candidates_num` is no use to
///    somebody looking at a wizard.
/// 2. [`to_workbook`].
/// 3. [`super::build`].
/// 4. Deserialize the export into [`ImportElectionEventSchema`] and
///    [`super::validate`] it — the same second pass `step-cli` and
///    `buildFromWorkbook` each make. Two implementations that merely looked
///    similar would not survive this.
/// 5. [`super::archive::layout`].
/// 6. [`side_files`] into `auxiliary`, because a ceremony schedule is not part of
///    an import and putting it inside the archive would suggest otherwise.
pub fn compile_plan(
    plan: &Blueprint,
    templates: &TemplateSet,
    options: &BuildOptions,
    profile: Option<&Profile>,
) -> Result<Compiled, Report> {
    // The profile first, so a locked value is the one that gets validated and
    // the one that gets built. Checking the plan as written and then forcing the
    // value afterwards would report problems about text nobody will ship.
    let applied;
    let plan = match profile {
        Some(profile) => {
            applied = apply_profile(plan, profile)?;
            &applied
        }
        None => plan,
    };

    let mut report = validate_plan(plan);
    if let Some(profile) = profile {
        for problem in profile.warnings.problems.clone() {
            report.push(problem);
        }
        check_required(plan, profile, &mut report);
    }
    if report.has_errors() {
        return Err(report);
    }

    let workbook = to_workbook(plan).map_err(|problem| {
        let mut failed = Report::default();
        failed.push(problem);
        failed
    })?;

    // The trustees the plan collected become the key ceremony. Through
    // `BuildOptions` rather than patched onto the built bundle afterwards, so
    // anything that calls `build` — including this file's own test helper — gets
    // the same document the wizard ships. The first version patched afterwards and
    // every test asserting on `compiled()` saw an empty array.
    //
    // The photographs travel the same way, and for a second reason: a workbook cell
    // cannot hold bytes. The sheet carries each one's identifier and the builder
    // composes the url; these are the files themselves, on their way to the archive.
    let with_ceremony = BuildOptions {
        // No keys ceremony in the archive, and this is a considered omission rather
        // than a gap.
        //
        // `trustee_ids` on a ceremony carries trustee **names**, which the importer
        // resolves against trustees already provisioned in the target tenant
        // (`import_election_event.rs`: `trustee_map` from `get_all_trustees`). A name
        // it cannot find becomes an empty string through `unwrap_or_default`, and the
        // insert then parses that as a `Uuid` — so a bundle naming trustees the tenant
        // does not have fails the *whole* import with `Error parsing trustee_ids as
        // UUIDs`, which says nothing about trustees and stops the event being created
        // at all.
        //
        // The wizard cannot know which trustees a tenant has: it runs in a browser with
        // no connection to the environment being imported into. So it cannot emit this
        // safely, and emitting it optimistically trades a working import for one that
        // fails opaquely — which is what a real deployment hit.
        //
        // Nothing is lost. The names, the threshold and the ceremony dates travel in
        // `auxiliary` as `ceremony_schedule.json` and the trustee list, which is where
        // a person reads them, and the ceremony itself is made in the Admin Portal
        // after import — where the trustees exist and can be picked rather than
        // spelled. `check_trustees` says so in the report.
        keys_ceremony: None,
        images: plan_images(plan),
        ceremony_policy: plan.ceremony_policy.clone(),
        materials: plan_materials(plan),
        ..options.clone()
    };
    let bundle = build(&workbook, templates, &with_ceremony)?;

    for problem in bundle.warnings.problems.clone() {
        report.push(problem);
    }

    match serde_json::from_value::<ImportElectionEventSchema>(
        bundle.export.clone(),
    ) {
        Ok(schema) => {
            for problem in super::validate(&schema).problems {
                report.push(problem);
            }
        }
        Err(error) => report.push(Problem::error(
            Code::InvalidValue,
            "bundle",
            format!(
                "the built bundle does not match the import schema, which is a \
                 bug in this tool rather than in the plan: {error}"
            ),
        )),
    }

    if report.has_errors() {
        return Err(report);
    }

    let mut layout = super::archive::layout(&bundle);
    for (name, contents) in side_files(plan) {
        layout.auxiliary.push(Artifact {
            name,
            bytes: contents.into_bytes(),
        });
    }

    // The same configuration, in the format a delivery engineer works in.
    //
    // Gated, and the gate is not decoration: this function is behind
    // `election_config_templates`, the writer is behind `election_config_xlsx`, and
    // `election_config_archive` implies the first but not the second. Without it,
    // the `election_config_archive` feature check stops compiling.
    //
    // A write failure loses the spreadsheet rather than the build — the same
    // choice `delivery()` already makes. Everything it would have carried is in
    // `blueprint.json` beside it, so nobody loses work over a tab name.
    #[cfg(feature = "election_config_xlsx")]
    if let Ok(bytes) = super::xlsx_write::write_xlsx(&workbook) {
        layout.auxiliary.push(Artifact {
            name: super::archive::WORKBOOK_MEMBER.to_string(),
            bytes,
        });
    }

    Ok(Compiled {
        bundle,
        layout,
        report,
    })
}

#[cfg(test)]
#[path = "architect_tests.rs"]
mod architect_tests;
