// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! What the SVG check keeps of a drawn chart, and what it takes out.

use super::{sanitize_svg, UnsafeSvg};

#[test]
fn a_plain_chart_is_kept_as_drawn() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="640" height="280" data-rendered-at="2026-09-29T10:00:00Z"><defs><linearGradient id="g"><stop offset="0" stop-color="#fff"/></linearGradient></defs><g class="bars"><rect x="1" y="2" width="3" height="4" fill="url(#g)"/><text x="5" y="6">53.2% &amp; more</text></g><use href="#g"/></svg>"##;
    assert_eq!(sanitize_svg(svg).unwrap(), svg);
}

#[test]
fn scripts_foreign_objects_and_unknown_elements_are_removed_with_their_content()
{
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script><foreignObject><div>x</div></foreignObject><image href="https://evil.invalid/x.png"/><a href="https://evil.invalid"><text>link</text></a><g><rect/></g></svg>"#;
    assert_eq!(
        sanitize_svg(svg).unwrap(),
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g><rect/></g></svg>"#
    );
}

#[test]
fn event_handlers_and_links_that_leave_the_document_are_removed() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" onload="alert(1)"><rect ONCLICK="x()" fill="red"/><use xlink:href="https://evil.invalid/s.svg#a"/><use href="#ok"/><rect fill="url(https://evil.invalid/p)" stroke="url( '#ok' )"/><rect style="fill: url(http://evil.invalid)"/><rect style="fill: red"/><rect title="javascript:alert(1)"/></svg>"##;
    assert_eq!(
        sanitize_svg(svg).unwrap(),
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><rect fill="red"/><use/><use href="#ok"/><rect stroke="url( '#ok' )"/><rect/><rect style="fill: red"/><rect/></svg>"##
    );
}

#[test]
fn a_style_sheet_that_imports_or_fetches_is_removed() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>@import url(https://evil.invalid/a.css);</style><style>.a{background:url("//evil.invalid/x")}</style><style>.b{fill:red}</style></svg>"#;
    assert_eq!(
        sanitize_svg(svg).unwrap(),
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.b{fill:red}</style></svg>"#
    );
}

#[test]
fn declarations_doctypes_comments_and_instructions_are_dropped() {
    let svg = r#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY x "y">]><!-- c --><svg xmlns="http://www.w3.org/2000/svg"><?pi x?><g/></svg>"#;
    assert_eq!(
        sanitize_svg(svg).unwrap(),
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g/></svg>"#
    );
}

#[test]
fn a_document_that_is_not_an_svg_is_refused() {
    assert_eq!(sanitize_svg("<html><body/></html>"), Err(UnsafeSvg::NotSvg));
    assert_eq!(sanitize_svg("not markup"), Err(UnsafeSvg::NotSvg));
    assert_eq!(
        sanitize_svg(r#"<svg xmlns="http://www.w3.org/2000/svg"><g></svg>"#),
        Err(UnsafeSvg::Malformed)
    );
}
