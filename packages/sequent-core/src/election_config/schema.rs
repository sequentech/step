// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The election event import bundle.
//!
//! This is the shape of `export_election_event-<uuid>.json`, and the single
//! definition of it. It was previously declared in
//! `windmill::services::import::import_election_event`, which meant every tool
//! that *wrote* an import had to reproduce it — janitor in Handlebars templates,
//! the Election Architect in a hand-built TypeScript object — and each
//! reproduction drifted.
//!
//! windmill re-exports this, so its own call sites are unchanged.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::hasura::core::{
    Application, Area, AreaContest, Candidate, Contest, Election, ElectionEvent, KeysCeremony, SupportMaterial,
};
use crate::types::scheduled_event::ScheduledEvent;
use crate::util::version::HISTORICAL_DEFAULT_VERSION;

use super::report::Report;

/// Everything an election event import carries.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportElectionEventSchema {
    /// The tenant the bundle was exported from.
    ///
    /// A `String` rather than a `Uuid` on purpose. Import replaces this value
    /// with the tenant of the importing request regardless of what it says
    /// (`replace_ids`: "always replace to ensure consistency"), so it is a
    /// placeholder rather than a destination, and every consumer only ever
    /// stringifies it.
    ///
    /// Keeping it a `String` also keeps `uuid` out of this module. That crate is
    /// only enabled by the `keycloak` feature here, and pulling it into
    /// `default_features` would put `getrandom` in the WASM build for no benefit.
    /// Its format is checked by validation instead, which reports a readable
    /// problem where a `Uuid` field would have produced an opaque serde error.
    pub tenant_id: String,

    /// The Keycloak realm, carried opaquely.
    ///
    /// Deliberately a `Value` and not a `RealmRepresentation`: that type comes
    /// from the `keycloak` crate, which pulls `reqwest`, and this module has to
    /// compile to WASM. Nothing is lost — serde round-trips it exactly, and
    /// windmill deserializes it into the typed form where it actually talks to
    /// Keycloak.
    ///
    /// Validating a realm needs a live Keycloak, so it is not something the
    /// shared validation could check even if the type were available.
    pub keycloak_event_realm: Option<Value>,

    pub election_event: ElectionEvent,
    pub elections: Vec<Election>,
    pub contests: Vec<Contest>,
    pub candidates: Vec<Candidate>,
    pub areas: Vec<Area>,
    pub area_contests: Vec<AreaContest>,

    /// The voting window.
    ///
    /// Normally `None`: the importer reads scheduled events from the
    /// `export_scheduled_events-<uuid>.csv` member of the zip, not from here.
    pub scheduled_events: Option<Vec<ScheduledEvent>>,

    /// Report definitions.
    ///
    /// Normally empty for the same reason: `insert_reports` is only ever called
    /// from `process_reports_file`, so reports travel in
    /// `export_reports-<uuid>.csv` and a populated array here is silently
    /// dropped. The field is required by the format, so it must still be present.
    pub reports: Vec<Report>,

    pub keys_ceremonies: Option<Vec<KeysCeremony>>,
    pub applications: Option<Vec<Application>>,

    /// Voter-facing help documents — rules, candidate statements, a guide to voting.
    ///
    /// `Option`, like the two above and for the same reason: every bundle written
    /// before this field existed still deserialises.
    ///
    /// These are the *rows*. The files they point at travel separately, as
    /// `export_S3_files/document_<id>_<name>` members of the archive, and the
    /// `document_id` here is what puts that identifier into the importer's
    /// replacement map — `process_s3_file` fails the whole import for a zip entry
    /// whose uuid the JSON never mentions. See
    /// `engineering/how-a-support-material-travels-in-a-bundle`.
    pub support_materials: Option<Vec<SupportMaterial>>,

    /// The platform version that wrote the bundle.
    ///
    /// Defaults to the first version that recorded one, so bundles predating the
    /// field still import.
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    HISTORICAL_DEFAULT_VERSION.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest bundle that deserializes. Every field here is one the format
    /// requires; anything omitted below has a serde default.
    const MINIMAL: &str = r#"{
        "tenant_id": "9384db41-1b21-4b93-a6aa-edfc007136d8",
        "keycloak_event_realm": null,
        "election_event": {
            "id": "11111111-1111-5111-8111-111111111111",
            "tenant_id": "9384db41-1b21-4b93-a6aa-edfc007136d8",
            "is_archived": false,
            "encryption_protocol": "RSA256"
        },
        "elections": [],
        "contests": [],
        "candidates": [],
        "areas": [],
        "area_contests": [],
        "scheduled_events": null,
        "reports": [],
        "keys_ceremonies": [],
        "applications": []
    }"#;

    #[test]
    fn a_minimal_bundle_deserializes() {
        let parsed: ImportElectionEventSchema =
            serde_json::from_str(MINIMAL).unwrap();
        assert_eq!(parsed.election_event.encryption_protocol, "RSA256");
        assert!(parsed.keycloak_event_realm.is_none());
    }

    #[test]
    fn a_missing_version_falls_back_to_the_historical_default() {
        // Bundles written before the field existed must still import.
        let parsed: ImportElectionEventSchema =
            serde_json::from_str(MINIMAL).unwrap();
        assert_eq!(parsed.version, HISTORICAL_DEFAULT_VERSION);
    }

    #[test]
    fn the_realm_round_trips_untouched() {
        // Carried opaquely, so whatever the exporter wrote comes back byte for
        // byte — including keys this crate has never heard of.
        let source = MINIMAL.replace(
            "\"keycloak_event_realm\": null",
            r#""keycloak_event_realm": {"realm":"r","displayName":"D","somethingNew":[1,2]}"#,
        );
        let parsed: ImportElectionEventSchema =
            serde_json::from_str(&source).unwrap();
        let realm = parsed.keycloak_event_realm.as_ref().unwrap();
        assert_eq!(realm["displayName"], "D");
        assert_eq!(realm["somethingNew"], serde_json::json!([1, 2]));

        let round_tripped = serde_json::to_value(&parsed).unwrap();
        assert_eq!(&round_tripped["keycloak_event_realm"], realm);
    }

    #[test]
    fn a_missing_required_field_is_rejected() {
        // encryption_protocol is not Option, so its absence fails the whole file
        // at parse time. This is the error the shared validation exists to
        // pre-empt with something readable.
        //
        // Built by removing the key from the parsed tree rather than by editing
        // the text: a string replacement that stops matching would silently pass
        // this test instead of failing it.
        let mut tree: serde_json::Value =
            serde_json::from_str(MINIMAL).unwrap();
        let removed = tree["election_event"]
            .as_object_mut()
            .unwrap()
            .remove("encryption_protocol");
        assert!(removed.is_some(), "the fixture should have had the field");

        let parsed: Result<ImportElectionEventSchema, _> =
            serde_json::from_value(tree);
        assert!(parsed.is_err());
    }

    #[test]
    fn tenant_id_is_carried_as_written() {
        // Import replaces it, so it is a placeholder; it must survive unaltered
        // so that replace_ids can map it.
        let parsed: ImportElectionEventSchema =
            serde_json::from_str(MINIMAL).unwrap();
        assert_eq!(parsed.tenant_id, "9384db41-1b21-4b93-a6aa-edfc007136d8");
    }
}
