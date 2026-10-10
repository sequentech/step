// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

#[test]
fn wire_input_requires_the_selected_variants_fields() {
    let raw: Input = serde_json::from_str(r#"{"raw":{"html":"<p>synthetic</p>"}}"#).unwrap();
    assert!(matches!(raw, Input::Raw { html, pdf_options: None } if html == "<p>synthetic</p>"));
    let stored: Input = serde_json::from_str(
        r#"{"s3":{"bucket":"synthetic","input_path":"in.html","output_path":"out.pdf"}}"#,
    )
    .unwrap();
    assert!(
        matches!(stored, Input::S3 { bucket, input_path, output_path, pdf_options: None } if bucket == "synthetic" && input_path == "in.html" && output_path == "out.pdf")
    );
    for invalid in [
        r#"{"raw":{}}"#,
        r#"{"raw":{"html":12}}"#,
        r#"{"s3":{"bucket":"synthetic","input_path":"in.html"}}"#,
        r#"{"unknown":{}}"#,
    ] {
        assert!(serde_json::from_str::<Input>(invalid).is_err(), "{invalid}");
    }
    assert_eq!(
        serde_json::to_value(Output {
            pdf_base64: Some("JVBERi0=".into())
        })
        .unwrap(),
        serde_json::json!({"pdf_base64":"JVBERi0="})
    );
}

#[tokio::test]
async fn raw_html_produces_a_real_pdf_and_s3_is_rejected_locally() {
    // Uses the real pinned Chromium renderer. The fixture has no remote assets.
    let reply = handle_render_impl(Input::Raw {
        html: "<html><body>synthetic PDF contract</body></html>".into(),
        pdf_options: None,
    })
    .await
    .unwrap()
    .into_response();
    assert_eq!(reply.status(), 200);
    let bytes = warp::hyper::body::to_bytes(reply.into_body())
        .await
        .unwrap();
    let output: Output = serde_json::from_slice(&bytes).unwrap();
    let pdf = BASE64_STANDARD.decode(output.pdf_base64.unwrap()).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.windows(5).any(|part| part == b"%%EOF"));
    let error = match handle_render_impl(Input::S3 {
        bucket: "synthetic".into(),
        input_path: "in.html".into(),
        output_path: "out.pdf".into(),
        pdf_options: None,
    })
    .await
    {
        Err(error) => error,
        Ok(_) => panic!("local renderer accepted S3 input"),
    };
    assert!(error
        .find::<CustomError>()
        .unwrap()
        .0
        .contains("S3 mechanism"));
}
