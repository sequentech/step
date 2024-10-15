// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context as ContextAnyhow, Result};
use chrono::{DateTime, Utc};
use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext,
    RenderError, RenderErrorReason,
};
use num_format::{Locale, ToFormattedString};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use tracing::instrument;

#[instrument(skip_all, err)]
pub fn render_template_text(
    template: &str,
    variables_map: Map<String, Value>,
) -> Result<String, RenderError> {
    let mut reg = Handlebars::new();

    reg.register_helper("sanitize_html", Box::new(sanitize_html));
    reg.register_helper("format_u64", Box::new(format_u64));
    reg.register_helper("format_percentage", Box::new(format_percentage));

    // render handlebars template
    reg.render_template(template, &json!(variables_map))
}

#[instrument(skip_all, err)]
pub fn render_template(
    template_name: &str,
    template_map: HashMap<String, String>,
    variables_map: Map<String, Value>,
) -> Result<String, RenderError> {
    let mut reg = Handlebars::new();

    reg.register_helper("sanitize_html", Box::new(sanitize_html));
    reg.register_helper("format_u64", Box::new(format_u64));
    reg.register_helper("format_percentage", Box::new(format_percentage));

    for (name, file) in template_map {
        reg.register_template_string(&name, &file)?;
    }

    // render handlebars template
    reg.render(template_name, &json!(variables_map))
}

pub fn sanitize_html(
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
        ["strong", "em", "b", "i", "br"].iter().cloned().collect();

    let mut builder = ammonia::Builder::default();
    let builder = builder.tags(tags);
    let cleaned = builder.clean(param).to_string();

    out.write(&cleaned)?;

    Ok(())
}

pub fn format_u64(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let unformatted_number: u64 = helper
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("format_u64", 0))?
        .value()
        .as_u64()
        .ok_or(RenderErrorReason::InvalidParamType("couldn't parse as u64"))?;

    let formatted_number = unformatted_number.to_formatted_string(&Locale::en);

    out.write(&formatted_number)?;

    Ok(())
}

pub fn format_percentage(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let val: f64 = helper
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex(
            "format_percentage",
            0,
        ))?
        .value()
        .as_f64()
        .ok_or(RenderErrorReason::InvalidParamType("couldn't parse as f64"))?;

    let formatted_number = format!("{:.2}", val);

    out.write(&formatted_number)?;

    Ok(())
}

/// Convert unix time to RFC2822 date and time format, like: Tue, 1 Jul 2003
/// 10:52:37 +0200.
pub fn timestamp_to_rfc2822(timestamp: i64) -> Result<String> {
    let dt = DateTime::<Utc>::from_timestamp(timestamp, 0)
        .with_context(|| "Error parsing timestamp")?;
    let statement_timestamp = std::panic::catch_unwind(|| dt.to_rfc2822())
        .map_err(|_| anyhow!("Error converting timestamp to RFC2822 format"))?;

    Ok(statement_timestamp)
}

/// Convert unix time to the given format
pub fn format_datetime(unix_time: i64, fmt: &str) -> Result<String> {
    let dt = DateTime::<Utc>::from_timestamp(unix_time, 0)
        .with_context(|| "Error parsing creation timestamp")?;
    let formatted_str = dt.format(fmt).to_string();
    Ok(formatted_str)
}
