// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use clap::Args;
use sequent_core::services::reports::render_template;
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Args)]
#[command(about = "Render a template with variables into a report format")]
pub struct RenderTemplate {
    #[arg(short, long, value_name = "TEMPLATE")]
    template: String,

    #[arg(short, long, value_name = "VARS")]
    vars: String,

    #[arg(short, long, value_name = "OUTPUT")]
    output: String,
}

impl RenderTemplate {
    pub fn run(&self) {
        match self.generate_report() {
            Ok(_) => println!("Successfully generated the report"),
            Err(err) => eprintln!("Error! Failed to generate the report: {}", err),
        }
    }

    fn generate_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let vars_content = fs::read_to_string(&self.vars)
            .map_err(|e| format!("Could not read variables file: {}", e))?;
        let vars_json: Map<String, Value> = serde_json::from_str(&vars_content)
            .map_err(|e| format!("Could not parse variables JSON: {}", e))?;

        let template_content = fs::read_to_string(&self.template)
            .map_err(|e| format!("Could not read template file: {}", e))?;

        let rendered_output = render_template_with_vars(template_content, vars_json)?;

        fs::write(&self.output, rendered_output)
            .map_err(|e| format!("Failed to write the output file: {}", e))?;

        Ok(())
    }
}

fn render_template_with_vars(
    template_content: String,
    vars: Map<String, Value>,
) -> Result<Vec<u8>, String> {
    let base_template_name = "report_base_html";
    let base_html_template_field_path =
        "/workspaces/step/packages/velvet/src/resources/report_base_html.hbs";

    let base_template_content = fs::read_to_string(&base_html_template_field_path).map_err(|e| {
        format!(
            "Could not read template file {}: {}",
            base_html_template_field_path, e
        )
    })?;

    let mut template_map = HashMap::new();
    template_map.insert("report_content".to_string(), template_content);
    template_map.insert(base_template_name.to_string(), base_template_content);

    render_template(base_template_name, template_map, vars)
        .map(|rendered| rendered.into_bytes())
        .map_err(|e| format!("Template rendering error: {}", e))
}
