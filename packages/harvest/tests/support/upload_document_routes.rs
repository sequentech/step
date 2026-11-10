// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A document's name becomes part of its storage key and of the paths it is
//! uploaded and downloaded through. A name a user gives with a character
//! those paths cannot hold is refused before anything is stored, and a
//! generated name gets `_` in place of such a character.

use crate::route_services::{post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use windmill::services::documents::{
    document_name, is_document_name, DOCUMENT_NAME_REFUSAL,
};

const TENANT_ID: &str = "00000000-0000-4000-8000-00000000000a";
const EVENT_ID: &str = "00000000-0000-4000-8000-00000000000e";
const REFUSED: [&str; 9] = [
    "a;b.csv",
    "a#b.csv",
    "a\\b.csv",
    "a/b.csv",
    "../b.csv",
    "a%3Bb.csv",
    "a%2Fb.csv",
    "a?b.csv",
    "a\nb.csv",
];

fn refusal() -> Value {
    json!({
        "message": DOCUMENT_NAME_REFUSAL,
        "extensions": {"code": "invalid-document-name"}
    })
}

#[rocket::async_test]
async fn an_upload_named_with_a_character_its_paths_cannot_hold_is_refused() {
    // The refusal comes before the database, which these services refuse.
    let services = Services::without_database();
    let client = services.client().await;
    let uploads = REFUSED
        .iter()
        .map(|name| (Permissions::DOCUMENT_UPLOAD, *name, "text/csv"))
        // A tenant restore's archive too.
        .chain([(
            Permissions::TENANT_WRITE,
            "restore;1.zip",
            "application/zip",
        )]);
    for (permission, name, media_type) in uploads {
        let caller = Claims::new(TENANT_ID, "uploader").roles([permission]);
        let upload = json!({
            "name": name,
            "media_type": media_type,
            "size": 1,
            "is_public": false,
        });
        let (status, body) =
            text(post(&client, "/get-upload-url", &caller, &upload).await)
                .await;
        assert_eq!(status, Status::BadRequest, "{name:?}");
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap(),
            refusal(),
            "{name:?}"
        );
    }
}

#[rocket::async_test]
async fn a_report_named_with_a_character_its_paths_cannot_hold_is_refused() {
    let services = Services::without_database();
    let client = services.client().await;
    let caller = Claims::new(TENANT_ID, "reporter")
        .roles([Permissions::REPORT_GENERATE]);
    for name in REFUSED {
        let report = json!({
            "tenant_id": TENANT_ID,
            "election_event_id": EVENT_ID,
            "event_processor": "CREATE_REPORT",
            "cron_config": null,
            "event_payload": {
                "template": "<p>{{username}}</p>",
                "name": name,
                "variables": {},
                "format": "PDF",
            },
        });
        let (status, body) =
            text(post(&client, "/scheduled-event", &caller, &report).await)
                .await;
        assert_eq!(status, Status::BadRequest, "{name:?}");
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap(),
            refusal(),
            "{name:?}"
        );
    }
}

#[test]
fn a_generated_name_keeps_its_path_and_gets_an_underscore_for_any_other_such_character(
) {
    for name in [
        "census.csv",
        "Résultats 2026 (final).pdf",
        "tally_v2 [copy] & notes, +1 = @x 'y' ~z.pdf",
        "選挙.zip",
    ] {
        assert!(is_document_name(name), "{name}");
        assert_eq!(document_name(name), name);
    }
    for name in REFUSED {
        assert!(!is_document_name(name), "{name:?}");
    }
    assert_eq!(
        document_name("ballot-box-seals/election/area.json"),
        "ballot-box-seals/election/area.json"
    );
    assert_eq!(
        document_name("Mesa 1/2; sección #3 (50%)?\\\t.pdf"),
        "Mesa 1/2_ sección _3 (50_)___.pdf"
    );
}
