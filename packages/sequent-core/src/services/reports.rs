// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext,
    RenderError, RenderErrorReason,
};
use num_format::{Locale, ToFormattedString};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use tracing::instrument;
use chrono::{DateTime, Local, TimeZone, ParseError};


#[instrument(skip_all, err)]
pub fn render_template_text(
    template: &str,
    variables_map: Map<String, Value>,
) -> Result<String, RenderError> {
    let mut reg = Handlebars::new();

    reg.register_helper("sanitize_html", Box::new(sanitize_html));
    reg.register_helper("format_u64", Box::new(format_u64));
    reg.register_helper("format_percentage", Box::new(format_percentage));
    reg.register_helper("format_date", Box::new(format_percentage));

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

pub fn format_date(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    // Extract the date string from the first parameter
    let date_str: &str = helper
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex(
            "format_date",
            0,
        ))?
        .value()
        .as_str()
        .ok_or(RenderErrorReason::InvalidParamType("couldn't parse as &str"))?;

    // Extract the dynamic format string from the second parameter
    let format_str: &str = helper
        .param(1)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex(
            "format_date",
            1,
        ))?
        .value()
        .as_str()
        .ok_or(RenderErrorReason::InvalidParamType("couldn't parse as &str"))?;

    // Detect the appropriate date parsing format dynamically
    let parsed_date = if date_str.contains(':') {
        // If the date string contains a time, assume "YYYY-MM-DD HH:MM:SS"
        DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|err| RenderError::new(format!("Date parsing error: {}", err)))?
            .with_timezone(&Local) // Convert to local timezone
    } else {
        // Otherwise, assume it's just a date "YYYY-MM-DD" and add a time placeholder
        DateTime::parse_from_str(&format!("{} 00:00:00", date_str), "%Y-%m-%d %H:%M:%S")
            .map_err(|err| RenderError::new(format!("Date parsing error: {}", err)))?
            .with_timezone(&Local) // Convert to local timezone
    };

    // Format the date using the provided format string
    let formatted_date = parsed_date.format(format_str).to_string();

    // Write the formatted date to the output
    out.write(&formatted_date)?;

    Ok(())
}
