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

/// The legacy client URL selects only a configured kiosk origin. Its path and
/// content never enter receipt data, including for already queued tasks.
pub(super) fn build_ballot_tracker_url(
    portal_base: &str,
    kiosk_base: Option<&str>,
    client_url: &str,
    path: BallotTrackerPath<'_>,
) -> Result<String> {
    let mut url = portal_base_url(portal_base, "VOTING_PORTAL_URL")?;
    let requested = Url::parse(client_url).ok();
    let mut selected_kiosk = false;
    let mut permitted_origin = requested
        .as_ref()
        .is_some_and(|client| client.origin() == url.origin());
    if let Some(kiosk_base) = kiosk_base.filter(|base| !base.trim().is_empty()) {
        let kiosk_url = portal_base_url(kiosk_base, "KIOSK_VOTING_PORTAL_URL")?;
        permitted_origin |= requested
            .as_ref()
            .is_some_and(|client| client.origin() == kiosk_url.origin());
        // A shared origin cannot distinguish normal and kiosk requests.
        if kiosk_url.origin() != url.origin()
            && requested
                .as_ref()
                .is_some_and(|client| client.origin() == kiosk_url.origin())
        {
            url = kiosk_url;
            selected_kiosk = true;
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
    // Keep the established kiosk login mode without carrying arbitrary queries.
    if selected_kiosk
        || permitted_origin
            && requested
                .as_ref()
                .is_some_and(|client| client.query_pairs().any(|(key, _)| key == "kiosk"))
    {
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

    fn tracker(base: &str, kiosk: Option<&str>, client: &str) -> Result<String> {
        build_ballot_tracker_url(
            base,
            kiosk,
            client,
            BallotTrackerPath {
                tenant_id: "tenant-id",
                election_event_id: "event-id",
                election_id: "election-id",
                ballot_id: "0123456789abcdef",
            },
        )
    }

    #[test]
    fn tracker_uses_configured_portal_and_owned_ballot() {
        for base in ["https://voting.example", "https://voting.example/"] {
            assert_eq!(
                tracker(base, None, "https://voting.example/old-path").unwrap(),
                "https://voting.example/tenant/tenant-id/event/event-id/election/election-id/ballot-locator/0123456789abcdef"
            );
        }
        assert_eq!(
            tracker("http://localhost:3000/portal/", None, "ignored").unwrap(),
            "http://localhost:3000/portal/tenant/tenant-id/event/event-id/election/election-id/ballot-locator/0123456789abcdef"
        );
    }

    #[test]
    fn client_script_and_alternate_urls_do_not_enter_receipt_data() {
        let expected = tracker("https://voting.example", None, "").unwrap();
        for client in [
            "https://voting.example/\"};globalThis.receiptInjected=true;//",
            "https://voting.example/</script><script>globalThis.receiptInjected=true</script>",
            "javascript:globalThis.receiptInjected=true",
            "file:///etc/passwd",
            "http://169.254.169.254/latest/meta-data/",
            "https://voting.example/tenant/other/event/other/ballot-locator/other",
            "not a URL",
        ] {
            assert_eq!(
                tracker("https://voting.example", None, client).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn deployed_receipt_script_renders_the_server_tracker_url() {
        let client =
            "https://voting.example/</script><script>globalThis.receiptInjected=true</script>";
        let url = tracker("https://voting.example", None, client).unwrap();
        let template = include_str!(
            "../../../../../.devcontainer/minio/public-assets/ballot_receipt_user.hbs"
        );
        let script = &template[template.rfind("<script>").unwrap()..];
        let rendered = handlebars::Handlebars::new()
            .render_template(script, &serde_json::json!({"ballot_tracker_url": url}))
            .unwrap();
        assert!(!rendered.contains("receiptInjected"));
        assert!(rendered.contains(&format!("text: \"{url}\"")));
        assert_eq!(rendered.matches("<script>").count(), 1);
    }

    #[test]
    fn kiosk_receipts_use_only_the_configured_kiosk_base() {
        let kiosk = Some("https://kiosk.example/portal");
        assert_eq!(
            tracker("https://voting.example", kiosk, "https://kiosk.example/\"};globalThis.receiptInjected=true;//").unwrap(),
            "https://kiosk.example/portal/tenant/tenant-id/event/event-id/election/election-id/ballot-locator/0123456789abcdef?kiosk"
        );
        for client in [
            "https://kiosk.example.untrusted.example",
            "http://kiosk.example",
            "https://kiosk.example:8443",
        ] {
            assert!(tracker("https://voting.example", kiosk, client)
                .unwrap()
                .starts_with("https://voting.example/tenant/"));
        }
        assert_eq!(
            tracker("https://voting.example", Some(""), "https://kiosk.example").unwrap(),
            tracker("https://voting.example", None, "").unwrap()
        );
        assert_eq!(
            tracker(
                "https://voting.example",
                Some("https://voting.example/kiosk"),
                "https://voting.example/tenant/tenant-id"
            )
            .unwrap(),
            tracker("https://voting.example", None, "").unwrap()
        );
        assert_eq!(
            tracker(
                "https://voting.example",
                Some(" https://kiosk.example/portal "),
                "https://kiosk.example/tenant/tenant-id"
            )
            .unwrap(),
            tracker("https://voting.example", kiosk, "https://kiosk.example").unwrap()
        );
        for (kiosk_base, client) in [
            ("https://kiosk.example:443/portal", "https://kiosk.example"),
            ("https://kiosk.example/portal", "https://kiosk.example:443/"),
            (
                "https://kiosk.example/portal",
                " https://kiosk.example/old-path \n",
            ),
        ] {
            assert_eq!(
                tracker("https://voting.example", Some(kiosk_base), client).unwrap(),
                tracker("https://voting.example", kiosk, "https://kiosk.example").unwrap()
            );
        }
    }

    #[test]
    fn legacy_kiosk_origin_restores_the_static_login_flag() {
        let url = tracker(
            "https://voting.example",
            Some("https://kiosk.example"),
            "https://kiosk.example/legacy-path",
        )
        .unwrap();
        assert!(url.ends_with("/ballot-locator/0123456789abcdef?kiosk"));
        for (kiosk, client) in [
            (
                Some("https://kiosk.example"),
                "https://voting.example/legacy-path",
            ),
            (
                Some("https://voting.example/kiosk"),
                "https://voting.example/legacy-path",
            ),
            (
                Some("https://kiosk.example"),
                "https://untrusted.example/legacy-path",
            ),
        ] {
            let normal = tracker("https://voting.example", kiosk, client).unwrap();
            assert!(!normal.contains('?'));
        }
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
            assert!(tracker(base, None, "").is_err(), "accepted {base}");
            assert!(
                tracker("https://voting.example", Some(base), "").is_err(),
                "accepted kiosk {base}"
            );
        }
    }

    #[test]
    fn portal_settings_normalize_whitespace_and_identify_configuration_errors() {
        assert_eq!(
            tracker(" https://voting.example/portal ", None, "").unwrap(),
            tracker("https://voting.example/portal", None, "").unwrap()
        );
        for (base, kiosk, setting) in [
            ("invalid", None, "VOTING_PORTAL_URL"),
            (
                "https://voting.example",
                Some("invalid"),
                "KIOSK_VOTING_PORTAL_URL",
            ),
        ] {
            assert!(tracker(base, kiosk, "")
                .unwrap_err()
                .to_string()
                .contains(setting));
        }
    }

    #[test]
    fn kiosk_login_flag_survives_without_client_query_content() {
        for (base, kiosk, client) in [
            (
                "https://voting.example",
                None,
                "https://voting.example/old-path?kiosk&other=ignored",
            ),
            (
                "https://voting.example",
                Some("https://voting.example/kiosk"),
                "https://voting.example/old-path?kiosk",
            ),
            (
                "https://voting.example",
                Some("https://kiosk.example"),
                "https://kiosk.example/old-path?kiosk",
            ),
        ] {
            let url = tracker(base, kiosk, client).unwrap();
            assert!(url.ends_with("/ballot-locator/0123456789abcdef?kiosk"));
            assert!(!url.contains("ignored") && !url.contains("%3F"));
        }
        let normal = tracker(
            "https://voting.example",
            None,
            "https://untrusted.example/?kiosk",
        )
        .unwrap();
        assert!(!normal.contains('?'));
    }

    #[test]
    fn generated_url_is_safe_in_legacy_and_custom_script_literals() {
        let url = build_ballot_tracker_url(
            "https://voting.example/prefix'`$&",
            None,
            "ignored",
            BallotTrackerPath {
                tenant_id: "tenant-id",
                election_event_id: "event-id",
                election_id: "election-id",
                ballot_id: "\"'`\\</script>${alert(1)}?&#/\n\u{2028}",
            },
        )
        .unwrap();
        assert!(url.starts_with("https://voting.example/prefix%27%60%24%26/tenant/"));
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
