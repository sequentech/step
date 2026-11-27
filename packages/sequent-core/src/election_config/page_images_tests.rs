// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::election_config::paths::Cell;
use serde_json::json;

const SEAL: &str = "https://images.example.org/seal%281%29.svg";
// A synthetic image: no client file is in the repository.
const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2 2"><circle cx="1" cy="1" r="1"/></svg>"#;

fn data() -> String {
    data_address(SVG).unwrap()
}

/// A realm's sign-in CSS as Keycloak keeps it: MessageFormat, braces quoted.
fn login_css(address: &str) -> String {
    format!(
        ".logo '{{' background: url(\"{address}\") left / 42px no-repeat; '}}'\n\
         body::after '{{' background: #fff url(\"{address}\") center / 150px no-repeat; '}}'"
    )
}

/// A Parameters sheet like a delivered workbook's: the tenant package and
/// the event's presentation are JSON text in the `value` column.
fn workbook(address: &str) -> Workbook {
    let tenant = json!({
        "format": "sequent.tenant-package/1",
        "tenantSettings": {"localizationTexts": {"en": {"loginCustomCss": login_css(address)}}},
    });
    let presentation =
        json!({"logo_url": address, "css": ".a { color: red; }"});
    let grid = vec![
        vec![Cell::text("type"), Cell::text("key"), Cell::text("value")],
        vec![
            Cell::text("event"),
            Cell::text("tenant_package"),
            Cell::text(tenant.to_string()),
        ],
        vec![
            Cell::text("event"),
            Cell::text("election_event.presentation"),
            Cell::text(presentation.to_string()),
        ],
    ];
    Workbook::new(vec![Sheet::from_grid("Parameters", &grid).unwrap()]).unwrap()
}

fn value_of(workbook: &Workbook, key: &str) -> Value {
    let row = workbook
        .rows("parameters")
        .iter()
        .find(|row| row.text("key") == Some(key))
        .unwrap();
    match row.get("value").unwrap() {
        Value::String(text) => serde_json::from_str(text).unwrap(),
        parsed => parsed.clone(),
    }
}

#[test]
fn every_way_a_page_fetches_an_address_is_found() {
    let text = "a { background: url(https://a.example/1.png); } \
                b { background: url( 'https://a.example/2.png' ); } \
                c { background: url(''https://a.example/3.png''); } \
                @import \"https://fonts.example/css2?family=A\"; \
                @import url(//cdn.example/b.css); \
                <img src=\"https://a.example/4.png\" alt=\"\">";
    assert_eq!(
        internet_addresses(text),
        [
            "https://a.example/1.png",
            "https://a.example/2.png",
            "https://a.example/3.png",
            "//cdn.example/b.css",
            "https://fonts.example/css2?family=A",
            "https://a.example/4.png",
        ]
    );
}

#[test]
fn embedded_relative_and_unfetched_addresses_are_not_internet_images() {
    let text = "a { background: url(data:image/png;base64,AAAA); } \
                b { background: url(/resources/x.png); } \
                <a href=\"https://a.example/\">x</a> \
                <img data-src=\"https://a.example/lazy.png\">";
    assert!(internet_addresses(text).is_empty());
}

#[test]
fn the_check_names_each_cell_and_address_once() {
    let problems = check(&workbook(SEAL));
    let named: Vec<(String, String)> = problems
        .iter()
        .map(|p| (p.path.clone(), p.details["address"].to_string()))
        .collect();
    assert_eq!(named.len(), 2, "{named:?}");
    assert!(named.iter().all(|(_, address)| address == SEAL));
    assert!(problems
        .iter()
        .all(|p| p.severity == super::super::problem::Severity::Warning));
    assert!(problems
        .iter()
        .all(|p| p.id.as_deref() == Some("page.internet-image")));
    assert!(
        problems[0].message.contains("loginCustomCss"),
        "{}",
        problems[0].message
    );
    assert!(
        problems[1].message.contains("logo_url"),
        "{}",
        problems[1].message
    );
}

#[test]
fn a_logo_url_column_is_checked_too() {
    let grid = vec![
        vec![Cell::text("name"), Cell::text("presentation.logo_url")],
        vec![Cell::text("Event"), Cell::text(SEAL)],
    ];
    let workbook =
        Workbook::new(vec![Sheet::from_grid("ElectionEvent", &grid).unwrap()])
            .unwrap();
    assert_eq!(check(&workbook).len(), 1);
}

#[test]
fn embedding_leaves_nothing_to_warn_about() {
    let (embedded, replaced) = embed_image(&workbook(SEAL), SEAL, SVG).unwrap();
    assert_eq!(replaced, 3);
    assert!(check(&embedded).is_empty());
    let presentation = value_of(&embedded, "election_event.presentation");
    assert_eq!(presentation["logo_url"], data());
    assert_eq!(presentation["css"], ".a { color: red; }");
}

#[test]
fn a_stylesheet_carries_the_file_once() {
    let (embedded, _) = embed_image(&workbook(SEAL), SEAL, SVG).unwrap();
    let tenant = value_of(&embedded, "tenant_package");
    let css = tenant["tenantSettings"]["localizationTexts"]["en"]
        ["loginCustomCss"]
        .as_str()
        .unwrap();
    assert_eq!(css.matches(&data()).count(), 1);
    let name = property_name(SEAL);
    // Written the way the rest of a realm text is: MessageFormat braces.
    assert!(
        css.starts_with(&format!(
            ":root '{{' {name}: url(\"{}\"); '}}'\n",
            data()
        )),
        "{css}"
    );
    assert_eq!(css.matches(&format!("var({name})")).count(), 2);
    assert!(css
        .contains(&format!(".logo '{{' background: var({name}) left / 42px")));
}

#[test]
fn the_files_rule_comes_after_the_imports() {
    let css = format!(
        "@charset \"utf-8\";\n@import url(https://fonts.example/a.css);\n\
         a {{ background: url({SEAL}); }} b {{ background: url('{SEAL}'); }}"
    );
    let (out, n) = embed_in_text(&css, SEAL, &data());
    assert_eq!(n, 2);
    let import_end = out.find("a.css);").unwrap();
    let rule = out.find(":root {").unwrap();
    assert!(import_end < rule, "{out}");
}

#[test]
fn a_single_use_is_replaced_where_it_is() {
    let (out, n) =
        embed_in_text(&format!("<img src=\"{SEAL}\">"), SEAL, &data());
    assert_eq!(n, 1);
    assert_eq!(out, format!("<img src=\"{}\">", data()));
}

#[test]
fn only_an_image_is_embedded() {
    for (bytes, reason) in [
        (&b"<html><body>hi</body></html>"[..], "not a PNG"),
        (&br#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#[..], "script"),
        (&vec![0x89; MAX_IMAGE_BYTES + 1][..], "at most"),
    ] {
        let problem = embed_image(&workbook(SEAL), SEAL, bytes).unwrap_err();
        assert!(problem.message.contains(reason), "{}", problem.message);
    }
    assert!(data_address(b"\x89PNG\r\n\x1a\n....")
        .unwrap()
        .starts_with("data:image/png;base64,"));
    assert!(data_address(&[0xFF, 0xD8, 0xFF, 0xE0])
        .unwrap()
        .starts_with("data:image/jpeg;base64,"));
}

#[test]
fn an_address_the_workbook_does_not_name_is_refused() {
    let problem =
        embed_image(&workbook(SEAL), "https://other.example/x.png", SVG)
            .unwrap_err();
    assert!(problem.message.contains("nothing in the workbook names"));
    assert!(embed_image(&workbook(SEAL), "seal.svg", SVG).is_err());
}

/// A file larger than a cell holds is continued in `value+` cells when the
/// workbook is written, and read back whole.
#[cfg(feature = "election_config_xlsx")]
#[test]
fn a_large_file_survives_writing_and_reading_the_workbook() {
    use crate::election_config::{xlsx::read_xlsx, xlsx_write::write_xlsx};
    // Comments pad a valid SVG to about four cells' worth once in base64.
    let padding = "<!-- padding -->".repeat(6_000);
    let big =
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\">{padding}</svg>");
    let (embedded, _) =
        embed_image(&workbook(SEAL), SEAL, big.as_bytes()).unwrap();
    let reread = read_xlsx(&write_xlsx(&embedded).unwrap()).unwrap();
    let headers = &reread.sheet("parameters").unwrap().headers;
    assert!(headers.len() > 4, "{headers:?}");
    for key in ["tenant_package", "election_event.presentation"] {
        assert_eq!(value_of(&reread, key), value_of(&embedded, key), "{key}");
    }
    let expected = data_address(big.as_bytes()).unwrap();
    assert!(expected.len() > 3 * 32_767);
    assert_eq!(
        value_of(&reread, "election_event.presentation")["logo_url"],
        expected
    );
    assert!(check(&reread).is_empty());
}
