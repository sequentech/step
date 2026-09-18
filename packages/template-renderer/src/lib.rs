// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
pub mod assets;
pub mod bundle;
mod exchange;
pub mod helpers;
pub mod localization;
pub mod platform_csv;
mod schema;
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

const MAX_INPUT: usize = 32_000_000;
const CATALOG: &str = include_str!("../catalog/v1/catalog.json");
fn failure(message: impl ToString) -> Value {
    json!({"html":null,"diagnostics":[{"severity":"error","code":"render","message":message.to_string()}]})
}
#[wasm_bindgen]
pub fn studio_execute(input: &str) -> String {
    let result = if input.len() > MAX_INPUT {
        failure("Input exceeds 32 MB")
    } else {
        match serde_json::from_str(input) {
            Ok(value) => execute(value),
            Err(e) => failure(e),
        }
    };
    result.to_string()
}
pub fn execute(input: Value) -> Value {
    if input.to_string().len() > MAX_INPUT {
        return failure("Input exceeds 32 MB");
    }
    match input["op"].as_str().unwrap_or("render") {
        "catalog" => serde_json::from_str(CATALOG).expect("embedded catalog"),
        "render" => render(&input).unwrap_or_else(failure),
        "export" => exchange::export(&input).unwrap_or_else(failure),
        "import" => exchange::import(&input).unwrap_or_else(failure),
        "export_zip" => exchange::export_zip(&input).unwrap_or_else(failure),
        "import_zip" => exchange::import_zip(&input).unwrap_or_else(failure),
        "validate_assets" => match assets::from_value(&input["assets"]) {
            Ok(_) => json!({"diagnostics": []}),
            Err(e) => failure(e),
        },
        _ => failure("Unknown operation"),
    }
}
fn render(input: &Value) -> Result<Value, String> {
    let catalog: Value = serde_json::from_str(CATALOG).map_err(|e| e.to_string())?;
    let report = catalog["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == input["reportType"])
        .ok_or("Unsupported report type")?;
    let language = input["language"].as_str().unwrap_or("en");
    let default = input["defaultLanguage"].as_str().unwrap_or("en");
    let source = localization::language_chain(language, default)
        .iter()
        .find_map(|l| input["overrides"][l].as_str())
        .unwrap_or(input["source"].as_str().ok_or("Source must be text")?);
    let mut diagnostics = Vec::new();
    let source = localization::compile(
        source,
        language,
        default,
        &input["translations"],
        &mut diagnostics,
    )?;
    let data = input["data"]
        .as_object()
        .ok_or("Scenario must be a JSON object")?;
    schema::validate(&report["schema"], &input["data"], "/data", &mut diagnostics);
    if diagnostics.iter().any(|d| d["severity"] == "error") {
        return Ok(json!({"html":null,"diagnostics":diagnostics}));
    }
    helpers::take_diagnostics();
    let mut reg = helpers::get_registry();
    reg.set_dev_mode(false);
    let user = bounded_render(&reg, &source, data).map_err(|e| e.to_string())?;
    let mut system = data.clone();
    system.insert("rendered_user_template".into(), json!(user));
    let html = bounded_render(
        &reg,
        input["wrapper"]
            .as_str()
            .unwrap_or(report["wrapper"].as_str().unwrap()),
        &system,
    )
    .map_err(|e| e.to_string())?;
    if html.len() > 8_000_000 {
        return Err("Output exceeds 8 MB".into());
    }
    diagnostics.extend(helpers::take_diagnostics());
    let direction = if matches!(
        language.split('-').next().unwrap_or(language),
        "ar" | "he" | "fa" | "ur"
    ) {
        "rtl"
    } else {
        "ltr"
    };
    let root = regex::Regex::new(r"<html[^>]*>").map_err(|e| e.to_string())?;
    let html = root
        .replace(
            &html,
            format!(
                "<html lang=\"{}\" dir=\"{direction}\">",
                handlebars::html_escape(language)
            ),
        )
        .into_owned();
    let files = assets::from_value(&input["template"]["assets"])?;
    let html = assets::attach(&html, &files)?;
    Ok(
        json!({"html":html,"diagnostics":diagnostics,"catalogVersion":1,"language":language,"direction":direction}),
    )
}

fn bounded_render(
    reg: &handlebars::Handlebars<'_>,
    source: &str,
    data: &serde_json::Map<String, Value>,
) -> Result<String, String> {
    struct Output(Vec<u8>);
    impl std::io::Write for Output {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0.len() + bytes.len() > 8_000_000 {
                return Err(std::io::Error::other("Output exceeds 8 MB"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Output(Vec::new());
    reg.render_template_to_write(source, data, &mut output)
        .map_err(|e| e.to_string())?;
    String::from_utf8(output.0).map_err(|e| e.to_string())
}
