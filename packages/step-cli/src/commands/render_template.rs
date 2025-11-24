// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use clap::Args;
use sequent_core::services::reports::render_template_text;
use sequent_core::types::to_map::ToMap;
use serde_json::{Map, Value};
use std::fs;
use windmill::services::reports::manual_verification;

#[derive(clap::ValueEnum, Clone)]
pub enum TemplateType {
    Custom,
    ManualVerification,
}

/// Render a handlebars-rs template with variables
#[derive(Args)]
#[command(about)]
pub struct RenderTemplate {
    /// Path to the user handlebars-rs template
    #[arg(short = 't', long, value_name = "USER_TEMPLATE")]
    user_template: String,

    /// Path to the user file variables to use in JSON format
    #[arg(short = 'v', long, value_name = "USER_VARS")]
    user_vars: String,

    /// Path to the system template
    #[arg(short = 'T', long, value_name = "SYSTEM_TEMPLATE")]
    system_template: Option<String>,

    /// Path to the system file variables to use in JSON format
    #[arg(short = 'V', long, value_name = "SYSTEM_VARS")]
    system_vars: Option<String>,

    /// Type of the user template
    #[arg(
        short = 'y',
        long,
        value_name = "TEMPLATE_TYPE",
        default_value = "custom"
    )]
    template_type: TemplateType,

    /// Path to the output HTML file
    #[arg(short = 'o', long, value_name = "OUTPUT")]
    output: String,
}

impl RenderTemplate {
    /// Execute the rendering process
    pub fn run(&self) {
        match self.generate_report() {
            Ok(_) => println!("Successfully generated the report"),
            Err(err) => eprintln!("Error! Failed to generate the report: {err:?}"),
        }
    }

    /// Generate the report by reading templates, parsing variables, and
    /// rendering the output
    fn generate_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let user_vars_content = fs::read_to_string(&self.user_vars)
            .map_err(|e| format!("Could not read user variables file: {e:?}"))?;

        // Parse the user or system template variables based on the template type
        let user_vars: Map<String, Value> = match self.template_type {
            TemplateType::Custom => {
                let vars_json: Map<String, Value> = serde_json::from_str(&user_vars_content)
                    .map_err(|e| format!("Could not parse user template variables: {e:?}"))?;
                vars_json
            }
            TemplateType::ManualVerification => {
                let system_vars_content = fs::read_to_string(&self.user_vars)
                    .map_err(|e| format!("Could not read system variables file: {e:?}"))?;
                let system_template_data: manual_verification::UserData =
                    serde_json::from_str(&system_vars_content)
                        .map_err(|e| format!("Could not parse system template variables: {e:?}"))?;
                system_template_data.to_map()?
            }
        };

        // Load the appropriate user or manual verification template based on
        // the template type
        let user_template = match self.template_type {
            TemplateType::Custom => fs::read_to_string(&self.user_template)
                .map_err(|e| format!("Could not read user template file: {e:?}"))?,
            TemplateType::ManualVerification => {
                let custom_template = self.user_template.clone();
                fs::read_to_string(custom_template).map_err(|e| {
                    format!("Could not read manual verification template file: {e:?}")
                })?
            }
        };

        let rendered_output = self.render_template_with_vars(user_template, user_vars)?;

        fs::write(&self.output, rendered_output)
            .map_err(|e| format!("Failed to write the output file: {e:?}"))?;

        Ok(())
    }

    /// Render the template with the provided variables and base template
    fn render_template_with_vars(
        &self,
        user_template: String,
        user_vars: Map<String, Value>,
    ) -> Result<String, String> {
        let rendered_user_template: String = render_template_text(&user_template, user_vars)
            .map_err(|e| format!("User template rendering error: {e:?}"))?;

        // Determine the system template content based on the template type
        let system_template = match &self.template_type {
            TemplateType::Custom => "{{{rendered_user_template}}}".to_string(),
            TemplateType::ManualVerification => {
                if let Some(system_template) = &self.system_template {
                    fs::read_to_string(system_template).map_err(|e| {
                        format!(
                            "Could not read manual verification system template file {system_template:?}: {e:?}",
                        )
                    })?
                } else {
                    include_str!("../../../../.devcontainer/minio/public-assets/manual_verification_system.hbs").to_string()
                }
            }
        };

        let mut system_vars: Map<String, Value> = match self.template_type {
            TemplateType::Custom => Default::default(),
            TemplateType::ManualVerification => {
                let vars_path: String = self
                    .system_vars
                    .clone()
                    .ok_or(format!("System vars not provided"))?;
                let system_vars_content = fs::read_to_string(&vars_path)
                    .map_err(|e| format!("Could not read system variables file: {e:?}"))?;
                let system_template_data: manual_verification::SystemData =
                    serde_json::from_str(&system_vars_content)
                        .map_err(|e| format!("Could not parse system template variables: {e:?}"))?;
                system_template_data
                    .to_map()
                    .map_err(|e| format!("Error converting into map: {e:?}"))?
            }
        };
        system_vars.insert(
            "rendered_user_template".to_string(),
            Value::String(rendered_user_template),
        );

        render_template_text(&system_template, system_vars)
            .map_err(|e| format!("System template rendering error: {e:?}"))
    }
}
