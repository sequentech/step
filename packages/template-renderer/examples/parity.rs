// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_template_renderer::execute;
use serde_json::{json, Value};
fn main() {
    let catalog = execute(json!({"op":"catalog"}));
    let mut cases = Vec::new();
    for report in catalog["reports"].as_array().unwrap() {
        for scenario in report["scenarios"].as_array().unwrap() {
            let input = json!({"op":"render","reportType":report["id"],"source":report["source"],"data":scenario["data"]});
            cases.push(json!({"name":format!("{}/{}",report["id"],scenario["name"]),"expected":execute(input.clone()),"input":input}));
        }
    }
    for language in ["en", "es-MX", "fr", "ar", "he"] {
        let input = json!({"op":"render","reportType":"credentials","source":"{{t \"title\"}} {{t \"votes\" count}} {{number count}} {{date date}} {{username}}","data":{"count":3,"date":"2026-09-18T12:00:00Z","username":"<script>"},"language":language,"defaultLanguage":"en","translations":{"en":{"title":"Result","votes":{"one":"{count} vote","other":"{count} votes"}},"ar":{"votes":{"few":"{count} أصوات","other":"{count}"}}}});
        cases.push(json!({"name":language,"expected":execute(input.clone()),"input":input}));
    }
    let invalid = json!({"op":"render","reportType":"credentials","source":"{{#if}}","data":{}});
    cases.push(json!({"name":"invalid","expected":execute(invalid.clone()),"input":invalid}));
    println!("{}", Value::Array(cases));
}
