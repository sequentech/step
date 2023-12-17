// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use handlebars::{Handlebars, RenderError};
use serde_json::{json, Map, Value};
use tracing::instrument;

#[instrument(skip_all, err)]
pub fn render_template_text(
    template: &str,
    variables_map: Map<String, Value>,
) -> Result<String, RenderError> {
    // render handlebars template
    let reg = Handlebars::new();
    reg.render_template(template, &json!(variables_map))
}
