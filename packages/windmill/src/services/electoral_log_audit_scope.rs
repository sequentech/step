// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Only boards bound to current event/election rows may remain active.
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditLogOrigin<'a> {
    Current { expected_signer: &'a str },
    Imported,
}

pub fn classify<'a>(name: &str, current: &'a BTreeMap<String, String>) -> AuditLogOrigin<'a> {
    match current.get(name) {
        Some(key) => AuditLogOrigin::Current {
            expected_signer: key,
        },
        None => AuditLogOrigin::Imported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_post_log_uses_its_exact_registered_signing_identity() {
        let current = BTreeMap::from([("post-board".into(), "post-key".into())]);
        assert_eq!(
            classify("post-board", &current),
            AuditLogOrigin::Current {
                expected_signer: "post-key"
            }
        );
    }
    #[test]
    fn prefix_or_unrelated_log_never_becomes_a_current_post() {
        let current = BTreeMap::from([("post-board".into(), "post-key".into())]);
        for name in [
            "post-board-other",
            "old-post-board",
            "event-board",
            "foreign-board",
        ] {
            assert_eq!(classify(name, &current), AuditLogOrigin::Imported);
        }
    }
}
