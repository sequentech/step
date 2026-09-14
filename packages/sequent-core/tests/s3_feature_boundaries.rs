// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(all(
    feature = "s3",
    feature = "keycloak",
    feature = "default_features"
))]
#[path = "support/http.rs"]
mod http;
use http::{Exchange, HttpServer};
use serde_json::json;
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn downloads_reject_truncated_streams_instead_of_returning_partial_files() {
    if let (Ok(url), Ok(marker)) = (
        std::env::var("CORE_S3_TEST_URL"),
        std::env::var("CORE_S3_TEST_MARKER"),
    ) {
        assert_eq!(
            std::fs::read_to_string(marker).unwrap(),
            url,
            "owned child fixture"
        );
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            use sequent_core::services::s3::{
                get_file_from_s3, get_object_into_temp_file,
            };
            let file = get_object_into_temp_file(
                "bucket",
                "file-ok",
                "download-",
                ".txt",
            )
            .await
            .unwrap();
            assert_eq!(
                std::fs::read(file.path()).unwrap(),
                b"complete payload"
            );
            let bytes = get_file_from_s3("bucket".into(), "bytes-ok".into())
                .await
                .unwrap();
            assert_eq!(bytes, b"complete payload");
            assert!(get_file_from_s3("bucket".into(), "bytes-short".into())
                .await
                .unwrap_err()
                .to_string()
                .contains("Failed to read from S3 download stream"));
            assert!(
                get_object_into_temp_file(
                    "bucket",
                    "file-short",
                    "download-",
                    ".txt"
                )
                .await
                .is_err(),
                "a broken stream must not become a successful partial download"
            );
        });
        return;
    }
    let peer = HttpServer::start(vec![
        Exchange::json("GET", "/bucket/file-ok", 200, json!(null))
            .body("complete payload"),
        Exchange::json("GET", "/bucket/bytes-ok", 200, json!(null))
            .body("complete payload"),
        Exchange::json("GET", "/bucket/bytes-short", 200, json!(null))
            .body("complete payload")
            .truncate_at(4),
        Exchange::json("GET", "/bucket/file-short", 200, json!(null))
            .body("complete payload")
            .truncate_at(4),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("owned-fixture");
    std::fs::write(&marker, &peer.url).unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--exact","downloads_reject_truncated_streams_instead_of_returning_partial_files","--nocapture"])
        .env_clear().env("PATH","/usr/bin:/bin").env("HOME",dir.path())
        .env("CORE_S3_TEST_URL",&peer.url).env("CORE_S3_TEST_MARKER",&marker)
        .env("AWS_REGION","us-east-1").env("AWS_EC2_METADATA_DISABLED","true").env("AWS_MAX_ATTEMPTS","1")
        .env("AWS_S3_ACCESS_KEY","synthetic-access").env("AWS_S3_ACCESS_SECRET","synthetic-secret")
        .env("AWS_ACCESS_KEY_ID","synthetic-access").env("AWS_SECRET_ACCESS_KEY","synthetic-secret")
        .env("AWS_S3_PRIVATE_URI",&peer.url).env("AWS_S3_PUBLIC_URI",&peer.url)
        .stdout(Stdio::inherit()).stderr(Stdio::inherit());
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("owned S3 fixture exceeded its deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(status.success(), "isolated S3 download regression failed");
    let requests = peer.finish();
    assert_eq!(requests.len(), 4);
    assert!(requests
        .iter()
        .all(|r| r.headers["authorization"].contains("synthetic-access")));
}
