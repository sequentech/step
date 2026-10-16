// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use reqwest::Url;

// Older deployed and custom templates embed the tracker URL in raw JavaScript
// string literals, where URL parsing alone leaves characters such as
// apostrophes and backticks unchanged. Every byte outside this set is
// percent-encoded.
const SCRIPT_LITERAL_URL_BYTES: &[u8] = b"-._~:/%[]?";

pub(super) struct BallotTrackerPath<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub election_id: &'a str,
    pub ballot_id: &'a str,
}

fn portal_base_url(base: &str, setting: &str) -> Result<Url> {
    let base = base.trim();
    let url = Url::parse(base).with_context(|| format!("Invalid {setting}"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || base.chars().any(|c| c.is_control())
    {
        return Err(anyhow!(
            "{setting} must be an HTTP(S) base URL without credentials, query or fragment"
        ));
    }
    Ok(url)
}

/// Builds the receipt's tracker link from portal configuration and the
/// receipt's authorized identifiers. `is_kiosk` comes from the voter's token,
/// so nothing the client sends enters receipt data.
pub(super) fn build_ballot_tracker_url(
    portal_base: &str,
    kiosk_base: Option<&str>,
    is_kiosk: bool,
    path: BallotTrackerPath<'_>,
) -> Result<String> {
    let mut url = portal_base_url(portal_base, "VOTING_PORTAL_URL")?;
    if let Some(kiosk_base) = kiosk_base.filter(|base| !base.trim().is_empty()) {
        let kiosk_url = portal_base_url(kiosk_base, "KIOSK_VOTING_PORTAL_URL")?;
        if is_kiosk {
            url = kiosk_url;
        }
    }
    url.path_segments_mut()
        .map_err(|_| anyhow!("Voting portal URL must support hierarchical paths"))?
        .pop_if_empty()
        .extend([
            "tenant",
            path.tenant_id,
            "event",
            path.election_event_id,
            "election",
            path.election_id,
            "ballot-locator",
            path.ballot_id,
        ]);
    // The portal's kiosk login flag.
    if is_kiosk {
        url.set_query(Some("kiosk"));
    }

    let mut safe_url = String::new();
    for byte in url.as_str().bytes() {
        if byte.is_ascii_alphanumeric() || SCRIPT_LITERAL_URL_BYTES.contains(&byte) {
            safe_url.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(&mut safe_url, "%{byte:02X}")?;
        }
    }
    Ok(safe_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCATOR: &str =
        "/tenant/tenant-id/event/event-id/election/election-id/ballot-locator/0123456789abcdef";

    fn tracker_for(
        base: &str,
        kiosk_base: Option<&str>,
        is_kiosk: bool,
        ballot_id: &str,
    ) -> Result<String> {
        build_ballot_tracker_url(
            base,
            kiosk_base,
            is_kiosk,
            BallotTrackerPath {
                tenant_id: "tenant-id",
                election_event_id: "event-id",
                election_id: "election-id",
                ballot_id,
            },
        )
    }

    fn tracker(base: &str, kiosk_base: Option<&str>, is_kiosk: bool) -> Result<String> {
        tracker_for(base, kiosk_base, is_kiosk, "0123456789abcdef")
    }

    #[test]
    fn tracker_uses_configured_portal_and_owned_ballot() {
        for base in ["https://voting.example", "https://voting.example/"] {
            assert_eq!(
                tracker(base, None, false).unwrap(),
                format!("https://voting.example{LOCATOR}")
            );
        }
        assert_eq!(
            tracker("http://localhost:3000/portal/", None, false).unwrap(),
            format!("http://localhost:3000/portal{LOCATOR}")
        );
    }

    #[test]
    fn kiosk_receipts_use_the_kiosk_portal_and_login_flag() {
        let kiosk = Some("https://kiosk.example/portal");
        assert_eq!(
            tracker("https://voting.example", kiosk, true).unwrap(),
            format!("https://kiosk.example/portal{LOCATOR}?kiosk")
        );
        assert_eq!(
            tracker("https://voting.example", kiosk, false).unwrap(),
            format!("https://voting.example{LOCATOR}")
        );
        assert_eq!(
            tracker(
                "https://voting.example",
                Some(" https://kiosk.example:443/portal "),
                true
            )
            .unwrap(),
            tracker("https://voting.example", kiosk, true).unwrap()
        );
        // The voter's client, not the origin, selects a kiosk portal that
        // shares the normal portal's origin.
        assert_eq!(
            tracker(
                "https://voting.example",
                Some("https://voting.example/kiosk"),
                true
            )
            .unwrap(),
            format!("https://voting.example/kiosk{LOCATOR}?kiosk")
        );
    }

    #[test]
    fn kiosk_receipts_without_a_kiosk_portal_keep_the_login_flag() {
        for kiosk in [None, Some(""), Some("  ")] {
            assert_eq!(
                tracker("https://voting.example", kiosk, true).unwrap(),
                format!("https://voting.example{LOCATOR}?kiosk")
            );
            assert_eq!(
                tracker("https://voting.example", kiosk, false).unwrap(),
                format!("https://voting.example{LOCATOR}")
            );
        }
    }

    #[test]
    fn deployed_receipt_script_renders_the_server_tracker_url() {
        let url = tracker_for(
            "https://voting.example",
            None,
            false,
            "</script><script>globalThis.receiptInjected=true</script>",
        )
        .unwrap();
        let template = include_str!(
            "../../../../../.devcontainer/minio/public-assets/ballot_receipt_user.hbs"
        );
        let script = &template[template.rfind("<script>").unwrap()..];
        let rendered = handlebars::Handlebars::new()
            .render_template(script, &serde_json::json!({"ballot_tracker_url": url}))
            .unwrap();
        assert!(rendered.contains(&format!("text: \"{url}\"")));
        assert_eq!(rendered.matches("<script>").count(), 1);
    }

    #[test]
    fn invalid_portal_configuration_is_rejected() {
        for base in [
            "javascript:alert(1)",
            "file:///tmp/receipt",
            "not a URL",
            "https://user:password@voting.example",
            "https://voting.example/?query=1",
            "https://voting.example/#fragment",
            "https://voting.example/\npath",
        ] {
            assert!(tracker(base, None, false).is_err(), "accepted {base}");
            for is_kiosk in [false, true] {
                assert!(
                    tracker("https://voting.example", Some(base), is_kiosk).is_err(),
                    "accepted kiosk {base}"
                );
            }
        }
    }

    #[test]
    fn portal_settings_normalize_whitespace_and_identify_configuration_errors() {
        assert_eq!(
            tracker(" https://voting.example/portal ", None, false).unwrap(),
            tracker("https://voting.example/portal", None, false).unwrap()
        );
        for (base, kiosk, setting) in [
            ("invalid", None, "VOTING_PORTAL_URL"),
            (
                "https://voting.example",
                Some("invalid"),
                "KIOSK_VOTING_PORTAL_URL",
            ),
        ] {
            assert!(tracker(base, kiosk, false)
                .unwrap_err()
                .to_string()
                .contains(setting));
        }
    }

    #[test]
    fn generated_url_is_safe_in_legacy_and_custom_script_literals() {
        let url = build_ballot_tracker_url(
            "https://voting.example/prefix'`$&",
            None,
            true,
            BallotTrackerPath {
                tenant_id: "tenant-id",
                election_event_id: "event-id",
                election_id: "election-id",
                ballot_id: "\"'`\\</script>${alert(1)}?&#/\n\u{2028}",
            },
        )
        .unwrap();
        assert!(url.starts_with("https://voting.example/prefix%27%60%24%26/tenant/"));
        assert!(url.ends_with("?kiosk"));
        assert!(!url
            .chars()
            .any(|c| "\"'`\\<>&$\n\r\u{2028}\u{2029}".contains(c)));
        assert!(url.contains("%2F"));
        for template in [
            "<script>const tracker = \"{{{ballot_tracker_url}}}\";</script>",
            "<script>const tracker = '{{{ballot_tracker_url}}}';</script>",
            "<script>const tracker = `{{{ballot_tracker_url}}}`;</script>",
            "<a href=\"{{ballot_tracker_url}}\">Tracker</a>",
        ] {
            let rendered = handlebars::Handlebars::new()
                .render_template(template, &serde_json::json!({"ballot_tracker_url": url}))
                .unwrap();
            assert!(rendered.contains(&url));
        }
    }
}
