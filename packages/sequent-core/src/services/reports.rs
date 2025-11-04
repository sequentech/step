// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context as ContextAnyhow, Result};
use chrono::{DateTime, Local, Utc};
use handlebars::{
    handlebars_helper, BlockParamHolder, Context, Handlebars, Helper,
    HelperDef, HelperResult, JsonValue, Output, RenderContext, RenderError,
    RenderErrorReason, Renderable, ScopedJson,
};
use handlebars_chrono::HandlebarsChronoDateTime;
use num_format::{Locale, ToFormattedString};
use serde_json::{json, to_string, Map, Value};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use tracing::{info, instrument, warn};

fn get_registry<'reg>() -> Handlebars<'reg> {
    let mut reg = Handlebars::new();
    reg.set_strict_mode(false);
    reg.register_helper(
        "sanitize_html",
        helper_wrapper_or(Box::new(sanitize_html), String::from("-")),
    );
    reg.register_helper(
        "format_u64",
        helper_wrapper_or(Box::new(format_u64), String::from("-")),
    );
    reg.register_helper(
        "format_percentage",
        helper_wrapper_or(Box::new(format_percentage), String::from("-")),
    );
    reg.register_helper(
        "format_date",
        helper_wrapper_or(Box::new(format_date), String::from("-")),
    );
    reg.register_helper(
        "let",
        helper_wrapper_or(Box::new(let_helper), String::from("-")),
    );
    reg.register_helper(
        "expr",
        helper_wrapper_or(Box::new(expr_helper), String::from("-")),
    );
    reg.register_helper(
        "datetime",
        helper_wrapper_or(
            Box::new(HandlebarsChronoDateTime),
            String::from("-"),
        ),
    );
    reg.register_helper(
        "inc",
        helper_wrapper_or(Box::new(inc), String::from("-")),
    );
    reg.register_helper("to_json", helper_wrapper(Box::new(to_json)));
    reg.register_helper(
        "parse_i64",
        helper_wrapper_or(Box::new(parse_i64), String::from("-")),
    );
    reg.register_helper(
        "divide",
        helper_wrapper_or(Box::new(divide), String::from("-")),
    );
    reg.register_helper(
        "multiply",
        helper_wrapper_or(Box::new(multiply), String::from("-")),
    );
    reg.register_helper(
        "sum",
        helper_wrapper_or(Box::new(sum), String::from("-")),
    );
    reg.register_helper(
        "modulo",
        helper_wrapper_or(Box::new(modulo), String::from("-")),
    );
    reg.register_helper("eq", Box::new(eq));
    reg
}

#[instrument(skip_all, err)]
pub fn render_template_text(
    template: &str,
    variables_map: Map<String, Value>,
) -> Result<String, RenderError> {
    let reg = get_registry();

    // render handlebars template
    reg.render_template(template, &json!(variables_map))
}

#[instrument(skip_all, err)]
pub fn render_template(
    template_name: &str,
    template_map: HashMap<String, String>,
    variables_map: Map<String, Value>,
) -> Result<String, RenderError> {
    let mut reg = get_registry();

    for (name, file) in template_map {
        reg.register_template_string(&name, &file)?;
    }

    // render handlebars template
    reg.render(template_name, &json!(variables_map))
}

pub fn helper_wrapper_or<'a>(
    func: Box<dyn HelperDef + Send + Sync + 'a>,
    or_val: String,
) -> Box<dyn HelperDef + Send + Sync + 'a> {
    struct WrapperHelper<'a> {
        func: Box<dyn HelperDef + Send + Sync + 'a>,
        or_val: String,
    }

    impl<'a> HelperDef for WrapperHelper<'a> {
        fn call<'reg: 'rc, 'rc>(
            &self,
            helper: &Helper<'rc>,
            handlebars: &'reg Handlebars<'reg>,
            context: &'rc Context,
            render_context: &mut RenderContext<'reg, 'rc>,
            out: &mut dyn Output,
        ) -> HelperResult {
            match self.func.call(
                helper,
                handlebars,
                context,
                render_context,
                out,
            ) {
                Ok(val) => Ok(val),
                Err(err) => {
                    warn!(
                        "Error calling helper name={name:?} with params={params:?}, hash={hash:?}, returning or_val={or_val:?}: {err:?}. Ignoring it..",
                        name=helper.name(),
                        params=helper.params(),
                        hash=helper.hash(),
                        or_val=self.or_val,
                    );
                    out.write(&self.or_val)?;
                    Ok(())
                }
            }
        }
    }

    Box::new(WrapperHelper { func, or_val })
}

pub fn helper_wrapper<'a>(
    func: Box<dyn HelperDef + Send + Sync + 'a>,
) -> Box<dyn HelperDef + Send + Sync + 'a> {
    struct WrapperHelper<'a> {
        func: Box<dyn HelperDef + Send + Sync + 'a>,
    }

    impl<'a> HelperDef for WrapperHelper<'a> {
        fn call<'reg: 'rc, 'rc>(
            &self,
            helper: &Helper<'rc>,
            handlebars: &'reg Handlebars<'reg>,
            context: &'rc Context,
            render_context: &mut RenderContext<'reg, 'rc>,
            out: &mut dyn Output,
        ) -> HelperResult {
            match self.func.call(
                helper,
                handlebars,
                context,
                render_context,
                out,
            ) {
                Ok(val) => Ok(val),
                Err(err) => {
                    warn!(
                        "Error calling helper name={name:?} with params={params:?}, hash={hash:?}: {err:?}",
                        name=helper.name(),
                        params=helper.params(),
                        hash=helper.hash()
                    );
                    Err(err)
                }
            }
        }
    }

    Box::new(WrapperHelper { func })
}

pub fn expr_helper<'reg, 'rc>(
    h: &Helper<'rc>,
    _r: &'reg Handlebars<'reg>,
    _ctx: &'rc Context,
    rc: &mut RenderContext<'reg, 'rc>,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let value = h
        .param(0)
        .as_ref()
        .map(|v| v.value().to_owned())
        .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("expr", 0))?;

    let str_val = match value {
        Value::String(content) => content,
        Value::Null => String::from("-"),
        _ => value.to_string(),
    };

    // Write the value to the output
    out.write(&str_val)?;

    Ok(())
}

pub fn let_helper<'reg, 'rc>(
    h: &Helper<'rc>,
    _r: &'reg Handlebars<'reg>,
    _ctx: &'rc Context,
    rc: &mut RenderContext<'reg, 'rc>,
    _out: &mut dyn Output,
) -> Result<(), RenderError> {
    let name_param = h
        .param(0)
        .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("let", 0))?;

    let Some(Value::String(name_constant)) =
        name_param.try_get_constant_value()
    else {
        return Err(RenderErrorReason::ParamTypeMismatchForName(
            "let",
            "0".to_string(),
            "constant string".to_string(),
        )
        .into());
    };

    let value = h
        .param(1)
        .as_ref()
        .map(|v| v.value().to_owned())
        .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("let", 2))?;

    let block = rc.block_mut().unwrap();

    block.set_block_param(name_constant, BlockParamHolder::Value(value));

    Ok(())
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

fn parse_u64_value(value: &JsonValue) -> Result<u64, RenderError> {
    match value {
        JsonValue::Number(n) => n.as_u64().ok_or_else(|| {
            RenderError::new(format!(
                "Expected u64 but got invalid number: {n}"
            ))
        }),
        JsonValue::String(s) => s.parse::<u64>().map_err(|_| {
            RenderError::new(format!("Failed to parse '{}' as u64", s))
        }),
        _ => Err(RenderError::new(
            "Expected u64 or a string representing an u64",
        )),
    }
}

pub fn format_u64(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let unformatted_val = helper
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("format_u64", 0))?
        .value();
    let unformatted_number: u64 = parse_u64_value(unformatted_val)?;

    let formatted_number = unformatted_number.to_formatted_string(&Locale::en);
    out.write(&formatted_number)?;

    Ok(())
}

fn parse_f64_value(value: &JsonValue) -> Result<f64, RenderError> {
    match value {
        JsonValue::Number(n) => n.as_f64().ok_or_else(|| {
            RenderError::new(format!(
                "Expected f64 but got invalid number: {n}"
            ))
        }),
        JsonValue::String(s) => s.parse::<f64>().map_err(|_| {
            RenderError::new(format!("Failed to parse '{}' as f64", s))
        }),
        _ => Err(RenderError::new(
            "Expected f64 or a string representing an f64",
        )),
    }
}

#[allow(non_camel_case_types)]
pub struct divide;

impl HelperDef for divide {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        _handlebars: &'reg Handlebars<'reg>,
        _context: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        // Get the first parameter (dividend)
        let dividend_value = helper
            .param(0)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("divide", 0))?
            .value();
        let dividend = parse_f64_value(dividend_value)?;

        // Get the second parameter (divisor)
        let divisor_value = helper
            .param(1)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("divide", 1))?
            .value();
        let divisor = parse_f64_value(divisor_value)?;

        if divisor == 0.0 {
            return Err(RenderError::new("Division by zero"));
        }

        let result = dividend / divisor;
        Ok(ScopedJson::Derived(JsonValue::from(result)))
    }
}

#[allow(non_camel_case_types)]
pub struct sum;

impl HelperDef for sum {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        _handlebars: &'reg Handlebars<'reg>,
        _context: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        // Iterates through all parameters of the helper.
        // Parses each parameter as a u64 value.
        // Accumulates the parsed values into a sum.
        let result: u64 = helper
            .params()
            .iter()
            .map(|p| {
                p.value()
                    .as_u64()
                    .ok_or(RenderErrorReason::InvalidParamType(
                        "couldn't parse as u64",
                    ))
            })
            .sum::<Result<u64, RenderErrorReason>>()?;

        // Returns the sum as a ScopedJson.
        Ok(ScopedJson::Derived(JsonValue::from(result)))
    }
}

#[allow(non_camel_case_types)]
pub struct multiply;

impl HelperDef for multiply {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        _handlebars: &'reg Handlebars<'reg>,
        _context: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        // Get the first parameter
        let first_value = helper
            .param(0)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("multiply", 0))?
            .value();
        let first_num = parse_f64_value(first_value)?;

        // Get the second parameter
        let second_value = helper
            .param(1)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("multiply", 1))?
            .value();
        let second_num = parse_f64_value(second_value)?;

        let result = first_num * second_num;
        Ok(ScopedJson::Derived(JsonValue::from(result)))
    }
}

#[allow(non_camel_case_types)]
pub struct modulo;

impl HelperDef for modulo {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        _handlebars: &'reg Handlebars<'reg>,
        _context: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        // Get the first parameter (dividend)
        let dividend_value = helper
            .param(0)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("modulo", 0))?
            .value();
        let dividend = parse_u64_value(dividend_value)?;

        // Get the second parameter (divisor)
        let divisor_value = helper
            .param(1)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("modulo", 1))?
            .value();
        let divisor = parse_u64_value(divisor_value)?;

        if divisor == 0 {
            return Err(RenderError::new("Modulo by zero"));
        }

        let result = dividend % divisor;
        Ok(ScopedJson::Derived(JsonValue::from(result)))
    }
}

#[allow(non_camel_case_types)]
pub struct parse_i64;

impl HelperDef for parse_i64 {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        _r: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let num_str: &str = helper
            .param(0)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("parse_i64", 0))?
            .value()
            .as_str()
            .ok_or(RenderErrorReason::InvalidParamType(
                "couldn't parse as str",
            ))?;
        let num_i64: i64 = num_str.parse::<i64>().map_err(|_| {
            RenderErrorReason::InvalidParamType("couldn't parse as i64")
        })?;

        Ok(ScopedJson::Derived(json!(num_i64)))
    }
}

pub fn format_percentage(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let val_json = helper
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex(
            "format_percentage",
            0,
        ))?
        .value();

    let val = parse_f64_value(val_json)?;

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
    let date_json: &Value = helper
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("format_date", 0))?
        .value();

    let date_str: &str = date_json.as_str().ok_or_else(|| {
        warn!("couldn't parse as &str: date_json={date_json:?}");

        RenderErrorReason::InvalidParamType("couldn't parse as &str")
    })?;

    // Extract the dynamic format string from the second parameter
    let format_json: &Value = helper
        .param(1)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("format_date", 1))?
        .value();

    let format_str: &str = format_json.as_str().ok_or_else(|| {
        warn!("couldn't parse as &str: format_json={format_json:?}");

        RenderErrorReason::InvalidParamType("couldn't parse as &str")
    })?;

    // Detect the appropriate date parsing format dynamically
    let parsed_date = if date_str.contains(':') {
        // If the date string contains a time, assume "YYYY-MM-DD HH:MM:SS"
        DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|err| {
                RenderError::new(format!(
                    "Date parsing error: {err:?}, date_json={date_json:?}"
                ))
            })?
            .with_timezone(&Local) // Convert to local timezone
    } else {
        // Otherwise, assume it's just a date "YYYY-MM-DD" and add a time
        // placeholder
        DateTime::parse_from_str(
            &format!("{} 00:00:00", date_str),
            "%Y-%m-%d %H:%M:%S",
        )
        .map_err(|err| {
            RenderError::new(format!(
                "Date parsing error: {err:?}, date_json={date_json:?}"
            ))
        })?
        .with_timezone(&Local) // Convert to local timezone
    };

    // Format the date using the provided format string
    let formatted_date = parsed_date.format(format_str).to_string();

    // Write the formatted date to the output
    out.write(&formatted_date)?;

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

pub fn inc(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let index: u64 = helper
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("inc", 0))?
        .value()
        .as_u64()
        .ok_or(RenderErrorReason::InvalidParamType("couldn't parse as u64"))?;

    let inc_index = index + 1;

    out.write(&inc_index.to_string())?;

    Ok(())
}

pub fn to_json(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    // Get the first parameter (expected to be the data to serialize)
    if let Some(param) = h.param(0) {
        // Serialize the parameter to JSON
        let json =
            to_string(param.value()).unwrap_or_else(|_| "null".to_string());
        // Write the JSON to the template output
        out.write(&json)?;
    }
    Ok(())
}

#[allow(non_camel_case_types)]
pub struct eq;

impl HelperDef for eq {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        r: &'reg Handlebars<'reg>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let param0 = h
            .param(0)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("eq", 0))?
            .value();

        let param1 = h
            .param(1)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("eq", 1))?
            .value();

        let is_equal = match (param0, param1) {
            // Compare strings
            (Value::String(a), Value::String(b)) => a == b,
            // Compare numbers
            (Value::Number(a), Value::Number(b)) => a == b,
            // Compare booleans
            (Value::Bool(a), Value::Bool(b)) => a == b,
            // Compare null
            (Value::Null, Value::Null) => true,
            // For mixed types or other cases, compare as JSON values
            _ => param0 == param1,
        };

        if is_equal {
            // Render the block if the values are equal
            if let Some(template) = h.template() {
                template.render(r, ctx, rc, out)?;
            }
        } else {
            // Render the else block if the values are not equal
            if let Some(template) = h.inverse() {
                template.render(r, ctx, rc, out)?;
            }
        }

        Ok(())
    }
}
