// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic identifiers for a generated bundle.
//!
//! Every id is a version 5 UUID over the event's `external_id`, the entity kind,
//! and the row's `external_id`. Two consequences matter:
//!
//! * Regenerating an unchanged source produces byte-identical output, so a diff
//!   between two runs shows only what the author actually changed.
//! * Two events built from different sources never collide, because the event's
//!   `external_id` is mixed into the namespace.
//!
//! The importer rewrites every UUID it receives, so these are not the ids the
//! platform ends up storing. They exist to make the file reproducible and its
//! internal references consistent.
//!
//! Written out rather than taken from the `uuid` crate, for two reasons. The
//! crate's v4 feature pulls `getrandom`, whose WASM support is version-specific
//! and already pinned elsewhere in this workspace — nothing here needs randomness
//! and it should not acquire a reason to. And the byte layout below is the thing
//! that must never change: alter it and every event ever generated renumbers, so
//! it is better read than trusted.

use sha1::{Digest, Sha1};

/// Root namespace. Arbitrary but **frozen**: changing it renumbers every event
/// ever generated. `8f2b6c41-5d3e-5a7f-9c18-3ea1b7d40f62`.
pub const ROOT_NAMESPACE: [u8; 16] = [
    0x8f, 0x2b, 0x6c, 0x41, 0x5d, 0x3e, 0x5a, 0x7f, 0x9c, 0x18, 0x3e, 0xa1,
    0xb7, 0xd4, 0x0f, 0x62,
];

/// A version 5 UUID: SHA-1 over the namespace's bytes followed by the name.
///
/// RFC 9562 §5.5. The two masked bytes are the version and the variant, and they
/// are what make the digest a UUID rather than just a hash.
pub fn uuid5(namespace: &[u8; 16], name: &str) -> [u8; 16] {
    let mut hasher = Sha1::new();
    hasher.update(namespace);
    hasher.update(name.as_bytes());
    let digest = hasher.finalize();

    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50; // version 5
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant
    bytes
}

/// The canonical 8-4-4-4-12 lowercase hex form.
pub fn format_uuid(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Mints stable ids scoped to one election event.
#[derive(Debug, Clone)]
pub struct IdFactory {
    namespace: [u8; 16],
}

impl IdFactory {
    /// Scoped to the event's `external_id`, which must not be empty — without it
    /// two unrelated events would share a namespace.
    pub fn new(event_external_id: &str) -> Option<Self> {
        if event_external_id.is_empty() {
            return None;
        }
        Some(IdFactory {
            namespace: uuid5(&ROOT_NAMESPACE, event_external_id),
        })
    }

    pub fn namespace(&self) -> String {
        format_uuid(&self.namespace)
    }

    /// Id for one entity — `uid("contest", &["statewide-president"])`.
    ///
    /// `kind` keeps the per-entity keyspaces apart, so an area and a contest may
    /// share an `external_id` without colliding.
    ///
    /// Parts are length-prefixed rather than joined by a separator. Joined by
    /// one, `["a/b"]` and `["a", "b"]` would hash alike — which for an
    /// area/contest link means two different pairs sharing an id, and one
    /// silently overwriting the other. An `external_id` holding a slash is
    /// unusual, not forbidden.
    ///
    /// The prefix counts **characters, not bytes**, matching the Python this was
    /// ported from. A byte count would be the more obvious choice and would
    /// renumber every id derived from a non-ASCII `external_id`, for no gain:
    /// either count is unambiguous.
    pub fn uid(&self, kind: &str, parts: &[&str]) -> String {
        let mut name = String::new();
        for part in std::iter::once(&kind).chain(parts.iter()) {
            name.push_str(&part.chars().count().to_string());
            name.push(':');
            name.push_str(part);
        }
        format_uuid(&uuid5(&self.namespace, &name))
    }

    /// A tenant id derived from the event, for when none was supplied.
    ///
    /// Only a fallback: importing into an existing tenant needs that tenant's
    /// real id, which is why a caller should offer a way to pass one and should
    /// say out loud when it had to invent one.
    pub fn tenant_id(&self) -> String {
        self.uid("tenant", &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The event id used throughout, so the pinned values below are all one
    /// factory's output.
    const EVENT: &str = "seiu1000-leadership-2027";

    fn factory() -> IdFactory {
        IdFactory::new(EVENT).unwrap()
    }

    #[test]
    fn the_root_namespace_is_the_uuid_it_claims_to_be() {
        // The bytes are written out; this is what keeps them honest.
        assert_eq!(
            format_uuid(&ROOT_NAMESPACE),
            "8f2b6c41-5d3e-5a7f-9c18-3ea1b7d40f62"
        );
    }

    #[test]
    fn it_agrees_with_the_python_it_replaces() {
        // Every value here was produced by janitor's ids.py. Byte-identical ids
        // are what let the Rust take over without renumbering a single event
        // anyone has already generated — including the SEIU1000 bundle.
        let ids = factory();
        assert_eq!(ids.namespace(), "a2e05988-3ddc-509e-aa7e-837d598f9b68");
        assert_eq!(
            ids.uid("election_event", &[]),
            "7af38708-879f-5010-8de2-efe3d30c2b9d"
        );
        assert_eq!(
            ids.uid("election", &["statewide-officers"]),
            "cf433085-801f-56b4-ac9e-24245d4d516a"
        );
        assert_eq!(
            ids.uid("contest", &["statewide-president"]),
            "582381fe-c453-579c-92fb-0b325a2081c6"
        );
        assert_eq!(
            ids.uid("area_contest", &["area-statewide", "statewide-president"]),
            "815d65c4-4ec2-5cba-80e6-f88a46111772"
        );
        assert_eq!(ids.tenant_id(), "f79036c6-f1d6-5c6f-9b4e-4b866395c438");
    }

    #[test]
    fn a_non_ascii_external_id_hashes_the_way_the_python_does() {
        // The one place a byte count and a character count differ: "José-Muñoz"
        // is ten characters and twelve bytes. Getting this wrong renumbers every
        // id derived from an accented name, silently.
        assert_eq!(
            IdFactory::new("e")
                .unwrap()
                .uid("candidate", &["José-Muñoz"]),
            "dbaf7119-7510-5112-801c-09e6f72860f5"
        );
    }

    #[test]
    fn a_slash_in_one_part_does_not_collide_with_two_parts() {
        // What the length prefix is for. Joined by a separator these would be the
        // same id, and one area/contest link would overwrite another.
        let ids = IdFactory::new("e").unwrap();
        assert_eq!(
            ids.uid("area_contest", &["a/b"]),
            "855fb0e7-d9f9-50f1-bcd5-0d4d07767d3f"
        );
        assert_eq!(
            ids.uid("area_contest", &["a", "b"]),
            "e2f08132-5876-524f-8658-a0e982123176"
        );
        assert_ne!(
            ids.uid("area_contest", &["a/b"]),
            ids.uid("area_contest", &["a", "b"])
        );
    }

    #[test]
    fn a_version_5_uuid_says_so_in_its_bits() {
        let id = factory().uid("election", &["x"]);
        // Version nibble, then the variant nibble, at the positions RFC 9562
        // puts them.
        assert_eq!(id.as_bytes()[14], b'5', "version nibble in {id}");
        assert!(
            ['8', '9', 'a', 'b'].contains(&(id.as_bytes()[19] as char)),
            "variant nibble in {id}"
        );
    }

    #[test]
    fn the_kind_keeps_keyspaces_apart() {
        // An area and a contest may share an external_id; their ids must differ.
        let ids = factory();
        assert_ne!(ids.uid("area", &["board"]), ids.uid("contest", &["board"]));
    }

    #[test]
    fn two_events_never_share_an_id() {
        // The event's external_id is mixed into the namespace for exactly this.
        let one = IdFactory::new("event-a").unwrap();
        let two = IdFactory::new("event-b").unwrap();
        assert_ne!(
            one.uid("election", &["board"]),
            two.uid("election", &["board"])
        );
    }

    #[test]
    fn the_same_input_gives_the_same_id_every_time() {
        // The property that makes a regenerated bundle diffable.
        assert_eq!(
            factory().uid("contest", &["president"]),
            factory().uid("contest", &["president"])
        );
    }

    #[test]
    fn an_event_with_no_external_id_gets_no_factory() {
        // Without it, two unrelated events would share a namespace.
        assert!(IdFactory::new("").is_none());
    }

    #[test]
    fn a_formatted_uuid_is_lowercase_hex_in_the_canonical_groups() {
        let formatted = format_uuid(&[
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa,
            0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ]);
        assert_eq!(formatted, "00112233-4455-6677-8899-aabbccddeeff");
    }
}
