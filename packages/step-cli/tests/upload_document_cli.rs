// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#[path = "support/http_cli.rs"]
mod http_cli;
use http_cli::command;
use serde_json::{json, Value};

#[test]
fn upload_uses_the_public_endpoint_unless_local_is_explicit() {
    for local in [false, true] {
        let mut args = vec!["upload-document", "--file-path", "payload.json"];
        if local {
            args.push("--is-local");
        }
        let (output, _, requests) = command(&args, None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(requests.len(), 2);
        let query: Value = serde_json::from_slice(&requests[0].1).unwrap();
        assert_eq!(query["operationName"], "GetUploadUrl");
        assert_eq!(
            query["variables"],
            json!({"name":"payload.json", "media_type":"application/json",
            "size":19,"is_public":false,"is_local":local,"election_event_id":null})
        );
        assert_eq!(requests[1].0, "PUT /blob?signature=literal HTTP/1.1\r\n");
        assert_eq!(requests[1].1, b"{\"synthetic\":true}\n");
        assert!(String::from_utf8_lossy(&output.stdout).contains("Uploaded document. ID: document"));
    }
}
