// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use sequent_core::ballot::I18nContent;

fn presentation(entries: &[(&str, &[(&str, Option<&str>)])]) -> ElectionEventPresentation {
    let i18n: I18nContent<I18nContent<Option<String>>> = entries
        .iter()
        .map(|(language, overrides)| {
            (
                language.to_string(),
                overrides
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.map(str::to_string)))
                    .collect(),
            )
        })
        .collect();
    ElectionEventPresentation {
        i18n: Some(i18n),
        ..Default::default()
    }
}

fn texts(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

#[test]
fn copies_the_global_and_voting_portal_timezone_and_enrollment_overrides() {
    // COMELEC: PHT for Manila, a Dubai name, the opening sentence reworded.
    let event = presentation(&[(
        "en",
        &[
            ("global:timezones.abbr.Asia/Manila", Some("PHT")),
            ("votingPortal:timezones.name.Asia/Dubai", Some("Gulf Time")),
            (
                "votingPortal:enrollment.opensOn",
                Some("{0}: enrollment opens {1}, closes {2}."),
            ),
            // Other scopes and keys stay out of the realm.
            ("adminPortal:timezones.abbr.Asia/Manila", Some("PST")),
            ("timezones.abbr.Asia/Dubai", Some("GST")),
            ("global:electionSelectionScreen.title", Some("Elections")),
            ("templates:timezones.name.Asia/Manila", Some("Manila")),
            // An empty override sets nothing.
            ("global:timezones.city.Asia/Dubai", Some(" ")),
            ("global:timezones.city.Asia/Manila", None),
        ],
    )]);
    assert_eq!(
        realm_texts(Some(&event)),
        RealmTexts::from([(
            "en".to_string(),
            texts(&[
                ("timezones.abbr.Asia/Manila", "PHT"),
                ("timezones.name.Asia/Dubai", "Gulf Time"),
                (
                    "enrollment.opensOn",
                    "{0}: enrollment opens {1}, closes {2}."
                ),
            ])
        )])
    );
}

#[test]
fn the_voting_portal_scope_wins_over_global() {
    // Madrid association: a Spanish name for the Canary zone in both scopes.
    let event = presentation(&[(
        "es",
        &[
            (
                "votingPortal:timezones.name.Atlantic/Canary",
                Some("Hora canaria"),
            ),
            (
                "global:timezones.name.Atlantic/Canary",
                Some("Hora de Canarias"),
            ),
            (
                "global:timezones.name.Europe/Madrid",
                Some("Hora peninsular"),
            ),
        ],
    )]);
    assert_eq!(
        realm_texts(Some(&event)),
        RealmTexts::from([(
            "es".to_string(),
            texts(&[
                ("timezones.name.Atlantic/Canary", "Hora canaria"),
                ("timezones.name.Europe/Madrid", "Hora peninsular"),
            ])
        )])
    );
}

#[test]
fn catalan_goes_to_the_keycloak_locale_and_apostrophes_survive_message_format() {
    let event = presentation(&[(
        "cat",
        &[
            (
                "global:timezones.name.Europe/Madrid",
                Some("Hora d'Espanya"),
            ),
            (
                "global:timezones.voterDateTimeZone",
                Some("{{dateTime}} ({{zoneName}})"),
            ),
            (
                "global:enrollment.openUntil",
                Some("L'inscripció és oberta fins al {0} ({1}), d'acord."),
            ),
        ],
    )]);
    assert_eq!(
        realm_texts(Some(&event)),
        RealmTexts::from([(
            "ca".to_string(),
            texts(&[
                ("timezones.name.Europe/Madrid", "Hora d''Espanya"),
                ("timezones.voterDateTimeZone", "{0} ({1})"),
                (
                    "enrollment.openUntil",
                    "L''inscripció és oberta fins al {0} ({1}), d''acord."
                ),
            ])
        )])
    );
}

#[test]
fn no_presentation_or_no_overrides_asks_for_nothing() {
    assert!(realm_texts(None).is_empty());
    assert!(realm_texts(Some(&ElectionEventPresentation::default())).is_empty());
    assert!(realm_texts(Some(&presentation(&[(
        "en",
        &[("global:other", Some("x"))]
    )])))
    .is_empty());
}

#[test]
fn realm_texts_under_the_synced_prefixes_that_are_gone_are_removed() {
    let current: HashMap<String, String> = [
        ("timezones.abbr.Asia/Manila", "PHT"),
        ("timezones.name.Asia/Dubai", "Gulf Time"),
        ("enrollment.opensOn", "old"),
        ("loginTitle", "Kept: not ours"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect();
    let wanted = texts(&[("timezones.abbr.Asia/Manila", "PHT")]);
    assert_eq!(
        stale_keys(&current, &wanted),
        vec!["enrollment.opensOn", "timezones.name.Asia/Dubai"]
    );
}

#[test]
fn invalid_combined_text_does_not_replace_the_theme_default() {
    for text in ["{{dateTime}}", "{{zoneName}}", "Hidden"] {
        let event = presentation(&[("en", &[("global:timezones.voterDateTimeZone", Some(text))])]);
        assert!(realm_texts(Some(&event)).is_empty());
    }
}

#[test]
fn accepted_placeholder_spellings_become_keycloak_arguments() {
    assert_eq!(
        keycloak_text(
            "timezones.voterDateTimeZone",
            "{{ dateTime }} ({{- zoneName}})"
        ),
        "{0} ({1})"
    );
}

#[test]
fn failed_override_removal_cannot_report_successful_synchronization() {
    for status in [
        reqwest::StatusCode::FORBIDDEN,
        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
    ] {
        let error = check_delete_status(status, "event-realm", "en", "timezones.name.Asia/Dubai")
            .unwrap_err();
        assert!(error.to_string().contains("old override still shows"));
    }
    for status in [
        reqwest::StatusCode::NO_CONTENT,
        reqwest::StatusCode::NOT_FOUND,
    ] {
        assert!(
            check_delete_status(status, "event-realm", "en", "timezones.name.Asia/Dubai").is_ok()
        );
    }
}

#[tokio::test]
async fn removal_http_errors_propagate_and_zone_keys_stay_one_path_segment() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    for (status, succeeds) in [(500, false), (404, true), (204, true)] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
            }
            let request = String::from_utf8(bytes).unwrap();
            assert!(request.starts_with("DELETE /admin/realms/event-realm/localization/en/timezones.name.Asia%2FDubai HTTP/1.1\r\n"), "{request}");
            write!(
                stream,
                "HTTP/1.1 {status} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        });
        let admin = PubKeycloakAdmin {
            url: format!("http://{address}"),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap(),
            token_supplier: serde_json::from_value(serde_json::json!({
                "access_token": "test-token", "expires_in": 60,
                "scope": "", "token_type": "Bearer"
            }))
            .unwrap(),
        };
        let result = delete_text(&admin, "event-realm", "en", "timezones.name.Asia/Dubai").await;
        server.join().unwrap();
        assert_eq!(result.is_ok(), succeeds, "HTTP {status}: {result:?}");
    }
}

#[test]
fn the_internal_fallback_cannot_be_replaced_by_event_localization() {
    let event = presentation(&[(
        "en",
        &[("global:timezones.defaultVoterDateTimeZone", Some("Hidden"))],
    )]);
    assert!(realm_texts(Some(&event)).is_empty());
}
