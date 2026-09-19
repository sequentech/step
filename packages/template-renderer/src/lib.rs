// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
pub mod assets;
pub mod bundle;
mod exchange;
pub mod helpers;
pub mod localization;
pub mod platform_csv;
pub mod prerender;
pub mod sample_data;
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
        "prerender_prepare" => match prepare_input(&input) {
            Ok(prepared) => serde_json::to_value(prepared).unwrap(),
            Err(e) => failure(e),
        },
        "render" => render(&input).unwrap_or_else(failure),
        "export" => exchange::export(&input).unwrap_or_else(failure),
        "import" => exchange::import(&input).unwrap_or_else(failure),
        "export_zip" => exchange::export_zip(&input).unwrap_or_else(failure),
        "import_zip" => exchange::import_zip(&input).unwrap_or_else(failure),
        "import_sample_data" => {
            use base64::Engine;
            match base64::engine::general_purpose::STANDARD
                .decode(input["base64"].as_str().unwrap_or(""))
            {
                Ok(bytes) => sample_data::import(&bytes).unwrap_or_else(failure),
                Err(e) => failure(e),
            }
        }
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
    let mut diagnostics = Vec::new();
    let translations =
        localization::merge_catalogs(&report["translations"], &input["translations"]);
    let channel = input["channel"].as_str().unwrap_or("document");
    if !matches!(channel, "document" | "email" | "sms") {
        return Err("Unsupported template channel".into());
    }
    let data = input["data"]
        .as_object()
        .ok_or("Scenario must be a JSON object")?;
    if channel == "document" && input["template"]["pre_render"]["enabled"] == true {
        let source = localization::language_chain(language, default)
            .iter()
            .find_map(|l| input["overrides"][l].as_str())
            .unwrap_or(input["source"].as_str().ok_or("Source must be text")?);
        let source =
            localization::compile(source, language, default, &translations, &mut diagnostics)?;
        let prepared = prerender::prepare(&source, &input["template"]["pre_render"]["known_data"])?;
        let options = &input["template"]["pdf_options"];
        let width = options["paper_width"].as_f64().unwrap_or(8.2677165354) * 72.0;
        let height = options["paper_height"].as_f64().unwrap_or(11.6929133858) * 72.0;
        let html = prerender::preview(&prepared, &input["data"], width, height)?;
        let assets = assets::from_value(&input["template"]["assets"])?;
        return Ok(
            json!({"html":assets::attach(&html, &assets)?,"diagnostics":diagnostics,"language":language,"direction":"ltr"}),
        );
    }
    schema::validate(&report["schema"], &input["data"], "/data", &mut diagnostics);
    if diagnostics.iter().any(|d| d["severity"] == "error") {
        return Ok(json!({"html":null,"diagnostics":diagnostics}));
    }
    helpers::take_diagnostics();
    let mut reg = helpers::get_registry();
    reg.set_dev_mode(false);
    let mut compile = |source: &str| {
        localization::compile(source, language, default, &translations, &mut diagnostics)
    };
    let html = if channel == "document" {
        let source = localization::language_chain(language, default)
            .iter()
            .find_map(|l| input["overrides"][l].as_str())
            .unwrap_or(input["source"].as_str().ok_or("Source must be text")?);
        let user = bounded_render(&reg, &compile(source)?, data)?;
        let mut system = data.clone();
        system.insert("rendered_user_template".into(), json!(user));
        let wrapper = input["wrapper"]
            .as_str()
            .unwrap_or(report["wrapper"].as_str().unwrap());
        bounded_render(&reg, &compile(wrapper)?, &system)?
    } else {
        let config = input["template"]
            .get(channel)
            .filter(|value| value.is_object())
            .unwrap_or(&report["configuration"][channel]);
        // Subject and SMS text must not become markup, including through runtime data.
        let mut plain = helpers::get_registry();
        plain.register_escape_fn(handlebars::no_escape);
        if channel == "email" {
            let subject = bounded_render(
                &plain,
                &compile(config["subject"].as_str().unwrap_or(""))?,
                data,
            )?;
            let body = if let Some(html) = config["html_body"].as_str().filter(|s| !s.is_empty()) {
                bounded_render(&reg, &compile(html)?, data)?
            } else {
                let text = bounded_render(
                    &plain,
                    &compile(config["plaintext_body"].as_str().unwrap_or(""))?,
                    data,
                )?;
                format!(
                    "<pre style=\"white-space:pre-wrap\">{}</pre>",
                    handlebars::html_escape(&text)
                )
            };
            format!(
                "<!doctype html><html><head><meta charset=\"utf-8\"><title>{0}</title></head><body><header style=\"font:600 16px sans-serif;padding:16px;border-bottom:1px solid #dbe1eb\">{0}</header><main>{body}</main></body></html>",
                handlebars::html_escape(&subject)
            )
        } else {
            let text = bounded_render(
                &plain,
                &compile(config["message"].as_str().unwrap_or(""))?,
                data,
            )?;
            format!(
                "<!doctype html><html><head><meta charset=\"utf-8\"></head><body><pre style=\"white-space:pre-wrap;font:16px/1.6 sans-serif;padding:24px\">{}</pre></body></html>",
                handlebars::html_escape(&text)
            )
        }
    };
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
        json!({"html":html,"diagnostics":diagnostics,"catalogVersion":1,"language":language,"direction":direction,"channel":channel}),
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

fn prepare_input(input: &Value) -> Result<prerender::Prepared, String> {
    let language = input["language"].as_str().unwrap_or("en");
    let default = input["defaultLanguage"].as_str().unwrap_or("en");
    let catalog: Value = serde_json::from_str(CATALOG).map_err(|e| e.to_string())?;
    let report = catalog["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == input["reportType"]);
    let translations = localization::merge_catalogs(
        &report
            .map(|r| r["translations"].clone())
            .unwrap_or(Value::Null),
        &input["translations"],
    );
    let source = localization::language_chain(language, default)
        .iter()
        .find_map(|l| input["overrides"][l].as_str())
        .unwrap_or(input["source"].as_str().unwrap_or(""));
    let mut diagnostics = Vec::new();
    let source = localization::compile(source, language, default, &translations, &mut diagnostics)?;
    let known = input
        .get("knownData")
        .unwrap_or(&input["template"]["pre_render"]["known_data"]);
    let mut prepared = prerender::prepare(&source, known)?;
    let files = assets::from_value(&input["template"]["assets"])?;
    prepared.html = assets::attach(&prepared.html, &files)?;
    Ok(prepared)
}
