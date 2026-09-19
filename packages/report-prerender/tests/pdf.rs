// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use lopdf::{dictionary, Document, Object, Stream};
use sequent_report_prerender::*;
use serde_json::json;
use std::io::{Cursor, Seek};
fn background() -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let content = doc.add_object(Stream::new(
        lopdf::Dictionary::new(),
        b"0.9 g 0 0 300 300 re f".to_vec(),
    ));
    let page = doc.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"Contents"=>content});
    // Deliberately inherited MediaBox/Resources tests real Chromium page trees.
    doc.objects.insert(pages,Object::Dictionary(dictionary!{"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1,"MediaBox"=>vec![0.into(),0.into(),300.into(),300.into()],"Resources"=>lopdf::Dictionary::new()}));
    let root = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    doc.trailer.set("Root", root);
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    out
}
fn template() -> Prepared {
    prepare(r#"{{pdf_text "/name" x=10 y=10 width=200 height=25}}{{pdf_qr "/url" x=10 y=60 width=90 height=90}}{{pdf_mark "/selected" x=200 y=60 width=20 height=10}}"#,&json!({})).unwrap()
}
#[test]
fn runtime_fields_and_thousand_ballots_stream_to_disk() {
    let mut file = tempfile::tempfile().unwrap();
    fill_many(&background(),&template(),DEFAULT_FONT,(0..1000).map(|i| Ok(json!({"name":format!("José #{i}"),"url":format!("https://example.test/receipt/{i}"),"selected":i%2==0}))),&mut file).unwrap();
    file.rewind().unwrap();
    let doc = Document::load_from(file).unwrap();
    assert_eq!(doc.get_pages().len(), 1000);
    // All pages share a single immutable background form.
    assert_eq!(
        doc.objects
            .values()
            .filter(|o| o
                .as_stream()
                .ok()
                .and_then(|s| s.dict.get(b"Subtype").ok())
                .and_then(|v| v.as_name().ok())
                == Some(b"Form"))
            .count(),
        1
    );
    let first = doc
        .get_page_content(*doc.get_pages().get(&1).unwrap())
        .unwrap();
    let last = doc
        .get_page_content(*doc.get_pages().get(&1000).unwrap())
        .unwrap();
    assert_ne!(first, last);
    assert!(String::from_utf8(first)
        .unwrap()
        .contains("/ActualText <FEFF004A006F007300E9"));
}
#[test]
fn absent_or_invalid_values_are_errors_not_blank_receipts() {
    for data in [
        json!({}),
        json!({"name":"a","url":"u","selected":"false"}),
        json!({"name":"x".repeat(1000),"url":"u","selected":false}),
    ] {
        assert!(fill_many(
            &background(),
            &template(),
            DEFAULT_FONT,
            [Ok(data)],
            Cursor::new(Vec::new())
        )
        .is_err());
    }
    let mut prepared = template();
    prepared.fields[0].page = 2;
    assert!(fill_many(
        &background(),
        &prepared,
        DEFAULT_FONT,
        [Ok(json!({}))],
        Cursor::new(Vec::new())
    )
    .is_err());
    prepared.fields[0].page = 1;
    prepared.fields[0].x = 290.0;
    assert!(fill_many(
        &background(),
        &prepared,
        DEFAULT_FONT,
        [Ok(json!({}))],
        Cursor::new(Vec::new())
    )
    .is_err());
}
#[test]
fn fingerprints_track_layout_and_election_changes_not_object_order() {
    let a = json!({"document":"x","pdf_options":{"scale":1}});
    let b = json!({"candidates":[{"id":"a","name":"Amy"}]});
    assert_eq!(
        fingerprint(&a, &b),
        fingerprint(&json!({"pdf_options":{"scale":1},"document":"x"}), &b)
    );
    assert_ne!(
        fingerprint(&a, &b),
        fingerprint(&a, &json!({"candidates":[{"id":"a","name":"Ana"}]}))
    );
    assert_ne!(
        fingerprint(&a, &b),
        fingerprint(&json!({"document":"y"}), &b)
    );
}

#[test]
fn conditional_pages_stitch_only_the_ballots_contests() {
    let mut doc = Document::load_mem(&background()).unwrap();
    let first = *doc.get_pages().get(&1).unwrap();
    let page = doc.get_object(first).unwrap().clone();
    let second = doc.add_object(page);
    let parent = doc
        .get_dictionary(first)
        .unwrap()
        .get(b"Parent")
        .unwrap()
        .as_reference()
        .unwrap();
    doc.get_object_mut(parent)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Kids", vec![first.into(), second.into()]);
    doc.get_object_mut(parent)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Count", 2);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    let manifest=prepare(r#"<section data-pdf-page="1"></section><section data-pdf-page="2"></section>{{pdf_page "/contests/c1" page=1}}{{pdf_page "/contests/c2" page=2}}{{pdf_mark "/selections/a" page=1 x=10 y=20 width=20 height=10}}{{pdf_mark "/selections/b" page=2 x=10 y=20 width=20 height=10}}"#,&json!({})).unwrap();
    let cached = CachedPdf {
        background: bytes,
        manifest,
    };
    let input = json!({"data":{"ballot_data":[{"id":"ballot-1","contest_choices":[{"contest":{"id":"c2","candidates":[{"id":"b"}]},"decoded_choices":[{"choice":{"id":"b","selected":0}}]}]}]}});
    let out = cached.bytes(&input).unwrap();
    let output = Document::load_mem(&out).unwrap();
    assert_eq!(output.get_pages().len(), 1);
    let content = String::from_utf8(
        output
            .get_page_content(*output.get_pages().get(&1).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(
        content.contains(" c f"),
        "selected candidate is a filled ellipse"
    );
}

#[test]
fn image_field_embeds_pixels_and_complex_text_has_actual_text() {
    use base64::Engine;
    let mut image_bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        2,
        2,
        image::Rgba([20, 60, 100, 128]),
    ))
    .write_to(&mut image_bytes, image::ImageFormat::Png)
    .unwrap();
    let manifest=prepare(r#"{{pdf_image "/image" x=10 y=10 width=50 height=50}}{{pdf_text "/name" x=10 y=80 width=250 height=25}}"#,&json!({})).unwrap();
    let data = json!({"image":base64::engine::general_purpose::STANDARD.encode(image_bytes.into_inner()),"name":"María مرحبا"});
    let output = fill_many(
        &background(),
        &manifest,
        DEFAULT_FONT,
        [Ok(data)],
        Cursor::new(Vec::new()),
    )
    .unwrap()
    .into_inner();
    let doc = Document::load_mem(&output).unwrap();
    assert!(doc.objects.values().any(|o| o
        .as_stream()
        .ok()
        .and_then(|s| s.dict.get(b"Subtype").ok())
        .and_then(|v| v.as_name().ok())
        == Some(b"Image")));
    assert!(String::from_utf8(
        doc.get_page_content(*doc.get_pages().get(&1).unwrap())
            .unwrap()
    )
    .unwrap()
    .contains("/ActualText <FEFF"));
}

#[test]
fn result_subsets_skip_unrelated_contests_and_keep_channel_totals_distinct() {
    let mut doc = Document::load_mem(&background()).unwrap();
    let first = *doc.get_pages().get(&1).unwrap();
    let second = doc.add_object(doc.get_object(first).unwrap().clone());
    let parent = doc
        .get_dictionary(first)
        .unwrap()
        .get(b"Parent")
        .unwrap()
        .as_reference()
        .unwrap();
    doc.get_object_mut(parent)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Kids", vec![first.into(), second.into()]);
    doc.get_object_mut(parent)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Count", 2);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    let manifest=prepare(r#"<section data-pdf-page="1"></section><section data-pdf-page="2"></section>{{pdf_page "/contests/c1" page=1}}{{pdf_page "/contests/c2" page=2}}{{pdf_text "/results/a/votes" page=1 x=10 y=10 width=100 height=25}}{{pdf_text "/results/b/votes" page=2 x=10 y=10 width=100 height=25}}{{pdf_text "/result_scopes/area1:c2:ONLINE/b/votes" page=2 x=10 y=40 width=100 height=25}}"#,&json!({})).unwrap();
    let cached = CachedPdf {
        background: bytes,
        manifest,
    };
    let input = json!({"reports":[
        {"area":{"id":"area1"},"contest":{"id":"c2"},"channel_type":null,"candidate_result":[{"candidate":{"id":"b"},"total_count":123}]},
        {"area":{"id":"area1"},"contest":{"id":"c2"},"channel_type":"ONLINE","candidate_result":[{"candidate":{"id":"b"},"total_count":45}]}
    ]});
    let output = Document::load_mem(&cached.bytes(&input).unwrap()).unwrap();
    assert_eq!(
        output.get_pages().len(),
        1,
        "only the contest present in this report is rendered"
    );
    let content = String::from_utf8(
        output
            .get_page_content(*output.get_pages().get(&1).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(
        content.contains("/ActualText <FEFF003100320033"),
        "main result keeps total 123"
    );
    assert!(
        content.contains("/ActualText <FEFF00340035"),
        "channel scope keeps breakdown 45"
    );
}

#[test]
fn result_scope_ambiguity_only_rejects_referenced_derived_fields() {
    let report = |area: Option<&str>, count: u32| json!({"area":area.map(|id|json!({"id":id})),"contest":{"id":"c1"},"channel_type":null,"candidate_result":[{"candidate":{"id":"a"},"total_count":count}]});
    let cached = |source: &str| CachedPdf {
        background: background(),
        manifest: prepare(source, &json!({})).unwrap(),
    };
    let duplicates = json!({"reports":[report(Some("area1"),10),report(Some("area1"),11)],"title":"Valid raw report"});
    assert!(
        cached(r#"{{pdf_text "/title" x=10 y=10 width=250 height=25}}"#)
            .bytes(&duplicates)
            .is_ok()
    );
    assert!(cached(
        r#"{{pdf_text "/result_scopes/area1:c1/a/votes" x=10 y=10 width=250 height=25}}"#
    )
    .bytes(&duplicates)
    .unwrap_err()
    .to_string()
    .contains("Missing runtime field"));
    assert!(
        cached(r#"{{pdf_text "/results/a/votes" x=10 y=10 width=250 height=25}}"#)
            .bytes(&duplicates)
            .is_err()
    );
    let consolidated =
        json!({"reports":[report(Some("area1"),10),report(Some("area2"),20),report(None,30)]});
    let output = Document::load_mem(
        &cached(r#"{{pdf_text "/results/a/votes" x=10 y=10 width=250 height=25}}"#)
            .bytes(&consolidated)
            .unwrap(),
    )
    .unwrap();
    let content = String::from_utf8(
        output
            .get_page_content(*output.get_pages().get(&1).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(
        content.contains("/ActualText <FEFF00330030"),
        "aggregate total is selected for consolidated reports"
    );
}

#[test]
fn background_validation_checks_manifest_without_runtime_records() {
    let pdf = background();
    let valid = template();
    validate_background(&pdf, &valid).unwrap();
    let mut invalid = valid.clone();
    invalid.version += 1;
    assert!(validate_background(&pdf, &invalid).is_err());
    invalid = valid.clone();
    invalid.fields[0].page = 2;
    assert!(validate_background(&pdf, &invalid).is_err());
    invalid = valid.clone();
    invalid.fields[0].x = 290.0;
    assert!(validate_background(&pdf, &invalid).is_err());
    invalid = valid.clone();
    invalid.fields[0].width = f32::NAN;
    assert!(validate_background(&pdf, &invalid).is_err());
    invalid = valid.clone();
    invalid.fields[0].kind = "html".into();
    assert!(validate_background(&pdf, &invalid).is_err());
    for path in ["name", "/broken~", "/broken~2escape"] {
        invalid = valid.clone();
        invalid.fields[0].path = path.into();
        assert!(validate_background(&pdf, &invalid).is_err());
    }
    invalid = valid.clone();
    invalid.fields[0].kind = "page".into();
    invalid.fields.push(invalid.fields[0].clone());
    assert!(validate_background(&pdf, &invalid).is_err());
    let mut escaped = valid;
    escaped.fields[0].path = "/name~1alias/~0key".into();
    validate_background(&pdf, &escaped).unwrap();
    assert!(validate_background(b"not a PDF", &escaped).is_err());
}

#[test]
fn validate_command_needs_no_runtime_file_and_rejects_invalid_geometry() {
    let directory = tempfile::tempdir().unwrap();
    let pdf = directory.path().join("background.pdf");
    let manifest = directory.path().join("manifest.json");
    std::fs::write(&pdf, background()).unwrap();
    let mut prepared = template();
    std::fs::write(&manifest, serde_json::to_vec(&prepared).unwrap()).unwrap();
    let validate = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_sequent-report-prerender"))
            .arg("validate")
            .arg(&pdf)
            .arg(&manifest)
            .output()
            .unwrap()
    };
    assert!(validate().status.success());
    prepared.fields[0].x = 1000.0;
    std::fs::write(&manifest, serde_json::to_vec(&prepared).unwrap()).unwrap();
    let result = validate();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("outside its page"));
}
