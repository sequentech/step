// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(all(feature = "reports", feature = "default_features"))]

//! Report templates are parsed and rendered recursively, so a template whose
//! partials include themselves, or that nests deeper than the limit, has to
//! be rejected with an error before anything recurses on it.

use sequent_core::services::reports::{render_template, render_template_text};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

const REPORT_BASE: &str = "<html>{{> report_content }}</html>";
const INCLUDES_ITSELF: &str = "includes itself";
const TOO_DEEP: &str = "levels deep";
const COMPUTED_PARTIAL_NAME: &str = "partial names must be literals";
const COMPUTED_INLINE_NAME: &str = "inline partial names must be string";
const UNCLOSED_BRACKET: &str = "closed by the first ']'";
const BEYOND_PARSER_DEPTH: usize = 10_000;
const NESTING_LIMIT: usize = 100;

/// Renders `report_content` as the partial that the base report template
/// includes, the way the report pipeline registers its templates.
fn render_report(report_content: &str) -> Result<String, String> {
    let template_map = HashMap::from([
        ("report_base_html".to_string(), REPORT_BASE.to_string()),
        ("report_content".to_string(), report_content.to_string()),
    ]);
    render_template("report_base_html", template_map, Map::new())
        .map_err(|error| error.to_string())
}

/// Renders a template text without variables.
fn render_text(template: &str) -> Result<String, String> {
    render_template_text(template, Map::new())
        .map_err(|error| error.to_string())
}

/// Checks that rendering failed with an error that mentions `reason`.
fn assert_rejected(result: Result<String, String>, reason: &str) {
    match result {
        Err(error) => assert!(error.contains(reason), "{error}"),
        Ok(rendered) => panic!("rendered {rendered:?}, expected: {reason}"),
    }
}

/// Builds `length` inline partials that each include the next one, and then
/// includes the first.
fn inline_partial_chain(length: usize) -> String {
    let mut template = String::new();
    for index in 0..length {
        let next = index + 1;
        template.push_str(&format!(
            "{{{{#*inline \"p{index}\"}}}}{{{{> p{next}}}}}{{{{/inline}}}}"
        ));
    }
    template.push_str(&format!(
        "{{{{#*inline \"p{length}\"}}}}x{{{{/inline}}}}{{{{> p0}}}}"
    ));
    template
}

/// Wraps `inner` in `depth` copies of `open` and the matching `close`.
fn nested(open: &str, inner: &str, close: &str, depth: usize) -> String {
    format!("{}{inner}{}", open.repeat(depth), close.repeat(depth))
}

/// Nests `depth` blocks around a single character.
fn nested_blocks(depth: usize) -> String {
    nested("{{#if true}}", "x", "{{/if}}", depth)
}

/// A block with `clauses` `else if` clauses, which nest one level deeper per
/// clause once parsed.
fn else_chain(keyword: &str, clauses: usize) -> String {
    format!(
        "{{{{#if false}}}}a{}{{{{/if}}}}",
        format!("{{{{{keyword} if false}}}}b").repeat(clauses)
    )
}

/// A content template that includes the base template that includes it is a
/// cycle through two registered templates.
#[test]
fn rejects_content_partial_that_includes_the_base_template() {
    assert_rejected(render_report("x{{> report_base_html}}"), INCLUDES_ITSELF);
}

/// Wrapping the include in a block helper does not hide the cycle.
#[test]
fn rejects_content_partial_that_includes_itself_inside_a_block() {
    assert_rejected(
        render_report("x{{#if true}}{{> report_content}}{{/if}}"),
        INCLUDES_ITSELF,
    );
}

/// A cycle that goes through three registered templates is found as well.
#[test]
fn rejects_registered_templates_that_include_each_other_in_a_ring() {
    let template_map = HashMap::from([
        ("a".to_string(), "{{> b}}".to_string()),
        ("b".to_string(), "{{#if true}}{{> c}}{{/if}}".to_string()),
        ("c".to_string(), "{{> a}}".to_string()),
    ]);

    let rendered = render_template("a", template_map, Map::new());

    assert_rejected(
        rendered.map_err(|error| error.to_string()),
        INCLUDES_ITSELF,
    );
}

/// An inline partial that includes itself recurses without end.
#[test]
fn rejects_inline_partial_that_includes_itself() {
    assert_rejected(
        render_text(r#"{{#*inline "a"}}x{{> a}}{{/inline}}{{> a}}"#),
        INCLUDES_ITSELF,
    );
}

/// Inline partials that include each other form a cycle, whether the
/// template is rendered on its own or as a registered one.
#[test]
fn rejects_inline_partials_that_include_each_other() {
    let template = concat!(
        r#"{{#*inline "a"}}{{> b}}{{/inline}}"#,
        r#"{{#*inline "b"}}{{#if true}}{{> a}}{{/if}}{{/inline}}"#,
        "{{> a}}",
    );
    assert_rejected(render_text(template), INCLUDES_ITSELF);
    assert_rejected(render_report(template), INCLUDES_ITSELF);
}

/// The content of a partial block is rendered by its partial, so it cannot
/// include that partial.
#[test]
fn rejects_partial_block_content_that_includes_its_partial() {
    let template = concat!(
        r#"{{#*inline "layout"}}<b>{{> @partial-block}}</b>{{/inline}}"#,
        "{{#> layout}}{{> layout}}{{/layout}}",
    );
    assert_rejected(render_text(template), INCLUDES_ITSELF);
}

/// A partial name computed by a helper cannot be checked for cycles.
#[test]
fn rejects_computed_partial_name() {
    assert_rejected(
        render_text(r#"{{#*inline "a"}}x{{> (concat "a")}}{{/inline}}{{> a}}"#),
        COMPUTED_PARTIAL_NAME,
    );
    assert_rejected(
        render_report(r#"x{{> (concat "report_" "content")}}"#),
        COMPUTED_PARTIAL_NAME,
    );
}

/// An inline partial defined under a name that comes from the data cannot be
/// matched to the partials that include it.
#[test]
fn rejects_inline_partial_with_computed_name() {
    let mut variables = Map::new();
    variables.insert("name".to_string(), Value::from("a"));
    let rendered = render_template_text(
        "{{#*inline name}}x{{> a}}{{/inline}}{{> a}}",
        variables,
    );
    assert_rejected(
        rendered.map_err(|error| error.to_string()),
        COMPUTED_INLINE_NAME,
    );
}

/// A long chain of partials has no cycle but renders as deep as it is long.
#[test]
fn rejects_long_inline_partial_chain() {
    assert_rejected(render_text(&inline_partial_chain(1_000)), TOO_DEEP);
}

/// Blocks nested far past the limit are rejected before they are parsed,
/// whether the template is rendered on its own or as a registered one.
#[test]
fn rejects_deeply_nested_blocks() {
    assert_rejected(render_text(&nested_blocks(1_000)), TOO_DEEP);
    assert_rejected(render_text(&nested_blocks(BEYOND_PARSER_DEPTH)), TOO_DEEP);
    assert_rejected(
        render_report(&nested_blocks(BEYOND_PARSER_DEPTH)),
        TOO_DEEP,
    );
}

/// The limit is on the number of levels, so one level more is rejected and
/// the limit itself still renders.
#[test]
fn rejects_nesting_one_level_past_the_limit() {
    assert_rejected(render_text(&nested_blocks(NESTING_LIMIT + 1)), TOO_DEEP);
    assert_eq!(
        render_text(&nested_blocks(NESTING_LIMIT)),
        Ok("x".to_string())
    );
}

/// Every `else if` clause nests inside the previous one, so a long chain is
/// as deep as the same number of nested blocks.
#[test]
fn rejects_long_else_chains() {
    assert_rejected(render_text(&else_chain("else", 1_000)), TOO_DEEP);
    assert_rejected(render_text(&else_chain("^", 1_000)), TOO_DEEP);
    assert_rejected(
        render_text(&format!(
            "{}{}{}",
            "{{#each items}}".repeat(60),
            else_chain("else", 60),
            "{{/each}}".repeat(60),
        )),
        TOO_DEEP,
    );
}

/// Subexpressions nest in the parser as blocks do.
#[test]
fn rejects_deeply_nested_subexpressions() {
    let template = nested(
        "{{concat ",
        &nested("(concat ", "\"a\"", ")", BEYOND_PARSER_DEPTH),
        "}}",
        1,
    );
    assert_rejected(render_text(&template), TOO_DEEP);
}

/// Array and object literals nest in the parser as blocks do.
#[test]
fn rejects_deeply_nested_literals() {
    let arrays = nested("[", "1", "]", BEYOND_PARSER_DEPTH);
    let objects = nested("{\"a\":", "1", "}", BEYOND_PARSER_DEPTH);
    assert_rejected(render_text(&format!("{{{{concat {arrays}}}}}")), TOO_DEEP);
    assert_rejected(
        render_text(&format!("{{{{concat {objects}}}}}")),
        TOO_DEEP,
    );
}

/// Closing tags inside comments, strings, escapes and raw blocks do not
/// close a block, so they cannot be used to hide how deep the blocks nest.
#[test]
fn counts_blocks_whose_closing_tags_sit_in_comments_strings_or_raw_text() {
    for open in [
        "{{#if true}}{{!-- {{/if}} --}}",
        "{{#if true}}{{! {{/if}}",
        r#"{{#if "{{/if}}"}}"#,
        r"{{#if true}}\{{/if}}",
        "{{#if true}}{{{{raw}}}}{{/if}}{{{{/raw}}}}",
        "{{#if true}}{{> 'a{{/if}}'}}",
        "{{#if true}}{{x.[{{/if}}]}}",
    ] {
        let template = nested(open, "x", "{{/if}}", BEYOND_PARSER_DEPTH);
        assert_rejected(render_text(&template), TOO_DEEP);
    }
}

/// A call to a helper named `else` is an ordinary tag, so the blocks after it
/// still count towards the limit.
#[test]
fn counts_blocks_after_a_helper_named_else() {
    for call in [
        r#"{{else "text"}}"#,
        "{{else answer=1}}",
        "{{ else [first name] }}",
    ] {
        let template = format!("{call}{}", nested_blocks(BEYOND_PARSER_DEPTH));
        assert_rejected(render_text(&template), TOO_DEEP);
    }
}

/// A bracket is an array literal or a path segment that ends at the first
/// closing bracket. A bracket that would need more than that is rejected,
/// because the end of its tag could not be told.
#[test]
fn rejects_bracket_not_closed_by_the_next_closing_bracket() {
    assert_rejected(render_text("{{concat [[1]]}}"), UNCLOSED_BRACKET);
    assert_rejected(render_text(r#"{{concat ["]"]}}"#), UNCLOSED_BRACKET);
}

/// Blocks nested up to the limit still render.
#[test]
fn renders_nested_blocks_within_the_limit() {
    assert_eq!(render_text(&nested_blocks(50)), Ok("x".to_string()));
}

/// A chain of `else if` clauses within the limit selects the first clause
/// that holds, and the final `else` applies when none does.
#[test]
fn renders_else_chains_within_the_limit() {
    let template = concat!(
        "{{#if a}}A{{else if b}}B{{else if c}}C{{else}}D{{/if}}",
        "{{#each items}}{{#if this}}+{{else}}-{{/if}}{{/each}}",
    );
    let render = |variables: Value| {
        let Value::Object(variables) = variables else {
            panic!("variables must be an object");
        };
        render_template_text(template, variables)
            .expect("template should render")
    };

    assert_eq!(render(json!({"c": true, "items": [1, 0]})), "C+-");
    assert_eq!(render(json!({"b": true, "c": true, "items": []})), "B");
    assert_eq!(render(json!({"items": [0]})), "D-");
    assert_eq!(
        render_text(&else_chain("else", NESTING_LIMIT - 2)),
        Ok(String::new())
    );
}

/// Comments, escapes, raw blocks, triple braces, array literals and strings
/// that look like tags are read as the template grammar reads them.
#[test]
fn renders_comments_escapes_raw_blocks_and_literals() {
    let template = concat!(
        "{{!-- {{#if}} --}}{{! {{/if}}",
        r"\{{a}}",
        "{{{{raw}}}}{{#b}}{{{{/raw}}}}",
        "{{{ value }}}",
        "{{#each [1, 2]}}{{this}}{{/each}}",
        r#"{{#if "}}"}}a{{/if}}{{#if "("}}b{{/if}}{{#if ")"}}c{{/if}}"#,
    );
    let mut variables = Map::new();
    variables.insert("value".to_string(), Value::from("<v>"));

    let rendered = render_template_text(template, variables)
        .expect("template should render");

    assert_eq!(rendered, "{{a}}{{#b}}<v>12abc");
}

/// Path segments in brackets, which may hold spaces, and string arrays still
/// render.
#[test]
fn renders_path_segments_and_array_literals() {
    let mut variables = Map::new();
    variables.insert("voter".to_string(), json!({"first name": "Ana"}));

    let rendered = render_template_text(
        r#"{{voter.[first name]}}{{#each ["x", "y"]}}{{this}}{{/each}}"#,
        variables,
    )
    .expect("template should render");

    assert_eq!(rendered, "Anaxy");
}

/// The report templates that ship with the report pipeline render.
#[test]
fn renders_the_velvet_report_templates() {
    for base in ["report_base_html", "report_base_pdf"] {
        let template_map = HashMap::from([
            (
                "report_base_html".to_string(),
                include_str!("../../velvet/src/resources/report_base_html.hbs")
                    .to_string(),
            ),
            (
                "report_base_pdf".to_string(),
                include_str!("../../velvet/src/resources/report_base_pdf.hbs")
                    .to_string(),
            ),
            (
                "report_content".to_string(),
                include_str!("../../velvet/src/resources/report_content.hbs")
                    .to_string(),
            ),
        ]);

        let rendered = render_template(base, template_map, Map::new());

        assert!(rendered.is_ok(), "{base}: {rendered:?}");
    }
}

/// A short chain of inline partials renders.
#[test]
fn renders_short_inline_partial_chain() {
    assert_eq!(render_text(&inline_partial_chain(10)), Ok("x".to_string()));
}

/// The base report template includes the content template once.
#[test]
fn renders_report_content_inside_the_base_template() {
    let mut variables = Map::new();
    variables.insert("items".to_string(), json!(["a", "b"]));
    let template_map = HashMap::from([
        ("report_base_html".to_string(), REPORT_BASE.to_string()),
        (
            "report_content".to_string(),
            "{{#each items}}{{#unless @first}},{{/unless}}{{this}}{{/each}}"
                .to_string(),
        ),
    ]);

    let rendered = render_template("report_base_html", template_map, variables)
        .expect("template should render");

    assert_eq!(rendered, "<html>a,b</html>");
}

/// A partial that several others include is not a cycle.
#[test]
fn renders_partial_included_from_several_places() {
    let template = concat!(
        r#"{{#*inline "leaf"}}x{{/inline}}"#,
        r#"{{#*inline "left"}}{{> leaf}}{{/inline}}"#,
        r#"{{#*inline "right"}}{{#if true}}{{> leaf}}{{/if}}{{/inline}}"#,
        "{{> left}}{{> right}}{{> leaf}}",
    );
    assert_eq!(render_text(template), Ok("xxx".to_string()));
}

/// Registered templates that include one another in a line, one of them
/// from two places, render.
#[test]
fn renders_registered_templates_that_include_one_another_in_a_line() {
    let template_map = HashMap::from([
        ("page".to_string(), "{{> header}}|{{> body}}".to_string()),
        ("header".to_string(), "H".to_string()),
        ("body".to_string(), "{{> header}}B".to_string()),
    ]);

    let rendered = render_template("page", template_map, Map::new())
        .expect("template should render");

    assert_eq!(rendered, "H|HB");
}

/// A partial block hands its content to the partial that it calls.
#[test]
fn renders_layout_partial_block() {
    let template = concat!(
        r#"{{#*inline "layout"}}<b>{{> @partial-block}}</b>{{/inline}}"#,
        "{{#> layout}}{{value}}{{/layout}}",
    );
    let mut variables = Map::new();
    variables.insert("value".to_string(), Value::from("x"));

    let rendered = render_template_text(template, variables)
        .expect("template should render");

    assert_eq!(rendered, "<b>x</b>");
}
