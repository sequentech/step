// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#[macro_use]
extern crate rocket;
#[path = "../src/routes/scanovate.rs"]
mod scanovate;
#[path = "../src/services/mod.rs"]
mod services;
#[path = "../src/types/mod.rs"]
mod types;

use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};

const BOUNDARY: &str = "mock-boundary";
const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0];
const WEBM: &[u8] = &[0x1A, 0x45, 0xDF, 0xA3];

async fn client() -> Client {
    Client::tracked(
        rocket::build()
            .manage(scanovate::MockSessions::default())
            .mount(
                "/",
                routes![
                    scanovate::flow_link,
                    scanovate::upload_media,
                    scanovate::session_token,
                    scanovate::results_with_image_names
                ],
            ),
    )
    .await
    .unwrap()
}

async fn create_session(client: &Client, process_id: &str, outcome: &str) {
    let link = client
        .post("/flow/v3/link")
        .json(&json!({
            "flow_id": 7,
            "identifier_id": process_id,
            "redirect_url": "http://keycloak/return",
            "params": { "mock_outcome": outcome }
        }))
        .dispatch()
        .await;
    assert_eq!(link.status(), Status::Ok);
}

fn multipart(parts: &[(&str, &str, &str, &[u8])]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, filename, content_type, content) in parts {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"; \
                 filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(content);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

fn full_capture() -> Vec<u8> {
    multipart(&[
        ("front_image", "front_image.jpg", "image/jpeg", JPEG),
        ("back_image", "back_image.jpg", "image/jpeg", JPEG),
        ("face_image", "face_image.jpg", "image/jpeg", JPEG),
        ("scan_video", "scan_video.webm", "video/webm", WEBM),
    ])
}

async fn upload(
    client: &Client,
    process_id: &str,
    body: Vec<u8>,
    authorization: Option<&str>,
) -> (Status, Option<Value>) {
    let mut request = client
        .post(format!("/api/v3/mobile_interaction/{process_id}/media"))
        .header(ContentType::new("multipart", "form-data").with_params(("boundary", BOUNDARY)))
        .body(body);
    if let Some(authorization) = authorization {
        request = request.header(Header::new("Authorization", authorization.to_string()));
    }
    let response = request.dispatch().await;
    let status = response.status();
    (status, response.into_json().await)
}

async fn results(client: &Client, process_id: &str) -> Value {
    client
        .get(format!(
            "/api/v3/mobile_interaction/v2/{process_id}/results_with_image_names"
        ))
        .dispatch()
        .await
        .into_json()
        .await
        .unwrap()
}

#[tokio::test]
async fn uploaded_media_completes_the_session_with_the_selected_outcome() {
    let client = client().await;
    create_session(&client, "media-ok", "success").await;
    create_session(&client, "media-fail", "document_authentication_failed").await;

    for process_id in ["media-ok", "media-fail"] {
        let (status, body) = upload(&client, process_id, full_capture(), Some("Bearer jwt")).await;
        assert_eq!(status, Status::Ok);
        assert_eq!(body, Some(json!({ "success": true, "errorCode": 0 })));
    }

    let ok = results(&client, "media-ok").await;
    assert_eq!(ok["data"]["success"], true);
    assert_eq!(
        ok["data"]["resultsList"][0]["backImage"],
        "media-ok/ocr/back_image.jpg"
    );
    let failed = results(&client, "media-fail").await;
    assert_eq!(failed["data"]["success"], false);
    assert_eq!(failed["data"]["errorCode"], 1026);
}

#[tokio::test]
async fn back_image_is_optional() {
    let client = client().await;
    create_session(&client, "front-only", "success").await;
    let body = multipart(&[
        ("front_image", "front_image.jpg", "image/jpeg", JPEG),
        ("face_image", "face_image.jpg", "image/jpeg", JPEG),
        ("scan_video", "scan_video.mp4", "video/mp4", WEBM),
    ]);

    let (status, _) = upload(&client, "front-only", body, Some("Bearer jwt")).await;

    assert_eq!(status, Status::Ok);
    let results = results(&client, "front-only").await;
    assert!(results["data"]["resultsList"][0].get("backImage").is_none());
}

#[tokio::test]
async fn upload_requires_a_bearer_token() {
    let client = client().await;
    create_session(&client, "no-token", "success").await;

    for authorization in [None, Some("Basic abc"), Some("Bearer  ")] {
        let (status, _) = upload(&client, "no-token", full_capture(), authorization).await;
        assert_eq!(status, Status::Unauthorized, "{authorization:?}");
    }
}

#[tokio::test]
async fn upload_requires_the_capture_parts() {
    let client = client().await;
    create_session(&client, "missing", "success").await;
    let body = multipart(&[
        ("front_image", "front_image.jpg", "image/jpeg", &[]),
        ("face_image", "face_image.jpg", "image/jpeg", JPEG),
    ]);

    let (status, body) = upload(&client, "missing", body, Some("Bearer jwt")).await;

    assert_eq!(status, Status::BadRequest);
    let body = body.unwrap();
    assert_eq!(body["success"], false);
    assert_eq!(body["data"], "missing parts: front_image");
}

#[tokio::test]
async fn document_only_capture_has_no_face_checks() {
    // With the liveness face capture, Keycloak checks the face on premise and
    // only sends the photos of the ID.
    let client = client().await;
    create_session(&client, "document-only", "success").await;
    let body = multipart(&[
        ("front_image", "front_image.jpg", "image/jpeg", JPEG),
        ("back_image", "back_image.jpg", "image/jpeg", JPEG),
    ]);

    let (status, _) = upload(&client, "document-only", body, Some("Bearer jwt")).await;

    assert_eq!(status, Status::Ok);
    let results = results(&client, "document-only").await;
    assert_eq!(results["data"]["success"], true);
    let processes: Vec<&str> = results["data"]["resultsList"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["process"].as_str().unwrap())
        .collect();
    assert_eq!(processes, ["ocr", "document_liveness_plus"]);
}

#[tokio::test]
async fn scan_video_is_optional() {
    let client = client().await;
    create_session(&client, "no-video", "success").await;
    let body = multipart(&[
        ("front_image", "front_image.jpg", "image/jpeg", JPEG),
        ("back_image", "back_image.jpg", "image/jpeg", JPEG),
        ("face_image", "face_image.jpg", "image/jpeg", JPEG),
    ]);

    let (status, _) = upload(&client, "no-video", body, Some("Bearer jwt")).await;

    assert_eq!(status, Status::Ok);
    assert_eq!(results(&client, "no-video").await["data"]["success"], true);
}

#[tokio::test]
async fn upload_to_an_unknown_session_is_not_found() {
    let client = client().await;

    let (status, _) = upload(&client, "unknown", full_capture(), Some("Bearer jwt")).await;

    assert_eq!(status, Status::NotFound);
}
