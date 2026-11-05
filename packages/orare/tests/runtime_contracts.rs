// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#[cfg(all(feature = "openwhisk", not(feature = "aws_lambda")))]
#[test]
fn compiled_openwhisk_consumer_maps_input_output_and_rejects_invalid_arguments() {
    let binary = std::env::var("ORARE_TEST_OPENWHISK_BINARY")
        .expect("build --example contract-openwhisk and set ORARE_TEST_OPENWHISK_BINARY");
    let valid = std::process::Command::new(&binary)
        .arg(r#"{"amount":17}"#)
        .output()
        .unwrap();
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    assert_eq!(
        String::from_utf8(valid.stdout).unwrap().trim(),
        r#"{"doubled":34}"#
    );
    for input in [r#"{"amount":"17"}"#, r#"{}"#, "not JSON"] {
        let invalid = std::process::Command::new(&binary)
            .arg(input)
            .output()
            .unwrap();
        assert!(!invalid.status.success());
        assert!(String::from_utf8_lossy(&invalid.stderr).contains("Failed to deserialize input"));
    }
    let missing = std::process::Command::new(&binary).output().unwrap();
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("<INPUT>"));
}

#[cfg(feature = "aws_lambda")]
mod aws {
    #[derive(serde::Deserialize)]
    struct Input {
        amount: u32,
    }
    #[derive(serde::Serialize)]
    struct Output {
        doubled: u32,
    }
    #[orare::lambda_runtime]
    async fn calculate(input: Input) -> std::result::Result<Output, String> {
        if input.amount == 0 {
            return Err("synthetic handler rejection".into());
        }
        Ok(Output {
            doubled: input.amount * 2,
        })
    }
    fn event(body: Option<&str>) -> LambdaEvent<LambdaFunctionUrlRequest> {
        let request: LambdaFunctionUrlRequest = serde_json::from_value(serde_json::json!({
            "body": body, "isBase64Encoded": false,
            "requestContext": {"timeEpoch": 0, "http": {"method":"POST"}}
        }))
        .unwrap();
        LambdaEvent::new(request, Default::default())
    }
    #[tokio::test]
    async fn successful_handler_returns_the_literal_http_contract() {
        let response = func(event(Some(r#"{"amount":17}"#))).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.headers["content-type"], "text/plain");
        assert_eq!(response.body, Some(r#"{"doubled":34}"#.into()));
        assert!(!response.is_base64_encoded);
        assert!(response.cookies.is_empty());
    }
    #[tokio::test]
    async fn missing_or_malformed_body_returns_an_error_instead_of_panicking() {
        for (body, expected) in [
            (None, "missing lambda function body"),
            (Some("not JSON"), "error reading lambda function arguments"),
            (
                Some(r#"{"amount":"17"}"#),
                "error reading lambda function arguments",
            ),
        ] {
            let error = func(event(body)).await.unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }
    #[tokio::test]
    async fn handler_failure_keeps_its_diagnostic_and_is_not_a_success_response() {
        let error = func(event(Some(r#"{"amount":0}"#))).await.unwrap_err();
        assert!(
            error.to_string().contains("synthetic handler rejection"),
            "{error}"
        );
    }
}
