use std::collections::HashSet;

// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext,
    RenderError,
};
use serde_json::{json, Map, Value};
use tracing::instrument;

#[instrument(skip_all, err)]
pub fn render_template_text(
    template: &str,
    variables_map: Map<String, Value>,
) -> Result<String, RenderError> {
    let mut reg = Handlebars::new();

    reg.register_helper("sanitize_html", Box::new(sanitize_html));

    // render handlebars template
    reg.render_template(template, &json!(variables_map))
}

fn sanitize_html(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = helper
        .param(0)
        .and_then(|v| v.value().as_str())
        .unwrap_or("");

    let tags: HashSet<&str> =
        ["strong", "em", "b", "i"].iter().cloned().collect();

    let mut builder = ammonia::Builder::default();
    let builder = builder.tags(tags);
    let cleaned = builder.clean(param).to_string();

    out.write(&cleaned)?;

    Ok(())
}
