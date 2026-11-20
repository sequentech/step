// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A presigned upload URL lets anyone upload to its key until it expires, so
//! it never reaches the logs.

#![cfg(all(
    feature = "s3",
    feature = "keycloak",
    feature = "default_features"
))]

#[path = "support/logs.rs"]
mod logs;

use logs::with_captured_logs;
use sequent_core::services::s3::get_upload_url;

/// The only test of this binary, so the environment it sets is its own.
#[test]
fn an_upload_url_stays_out_of_the_logs() {
    for (name, value) in [
        ("AWS_REGION", "us-east-1"),
        ("AWS_S3_PRIVATE_URI", "http://127.0.0.1:9000"),
        ("AWS_S3_BUCKET", "documents"),
        ("AWS_S3_UPLOAD_EXPIRATION_SECS", "60"),
        ("AWS_S3_ACCESS_KEY", "access-key"),
        ("AWS_S3_ACCESS_SECRET", "access-secret"),
    ] {
        std::env::set_var(name, value);
    }
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");

    let (url, logs) = runtime.block_on(with_captured_logs(get_upload_url(
        "tenant-t/document-d/upload.zip".to_string(),
        false,
        true,
    )));

    let url = url.expect("upload url");
    let signature = url.split("X-Amz-Signature=").nth(1).expect("a signed URL");
    assert!(logs.contains("get_upload_url"), "{logs}");
    assert!(!logs.contains(&url), "upload URL in logs: {logs}");
    assert!(!logs.contains(signature), "signature in logs: {logs}");
}
