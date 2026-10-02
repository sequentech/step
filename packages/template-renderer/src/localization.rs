// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use chrono::{DateTime, NaiveDate};
use handlebars::{Context, Handlebars, Helper, HelperDef, RenderContext, RenderError, ScopedJson};
use intl_pluralrules::{PluralRuleType, PluralRules};
use num_format::{Locale, ToFormattedString};
use regex::Regex;
use serde_json::{json, Value};
use std::str::FromStr;

/// Override individual words without copying a report's complete language catalog.
pub fn merge_catalogs(defaults: &Value, overrides: &Value) -> Value {
    let mut result = defaults.as_object().cloned().unwrap_or_default();
    if let Some(languages) = overrides.as_object() {
        for (language, values) in languages {
            if let Some(values) = values.as_object() {
                let catalog = result.entry(language.clone()).or_insert_with(|| json!({}));
                if let Some(catalog) = catalog.as_object_mut() {
                    catalog.extend(values.clone());
                }
            }
        }
    }
    Value::Object(result)
}

pub fn language_chain(language: &str, default: &str) -> Vec<String> {
    let mut result = Vec::new();
    for item in [
        language,
        language.split('-').next().unwrap_or(language),
        default,
    ] {
        if !result.iter().any(|s| s == item) {
            result.push(item.to_string());
        }
    }
    result
}
pub fn number(language: &str, value: &Value) -> Result<String, String> {
    let raw = if let Some(s) = value.as_str() {
        s.to_owned()
    } else {
        value.to_string()
    };
    let n: f64 = raw.parse().map_err(|_| "Expected a number")?;
    if !n.is_finite() {
        return Err("Expected a finite number".into());
    }
    let base = language.split('-').next().unwrap_or(language);
    let locale = Locale::from_name(language)
        .or_else(|_| Locale::from_name(base))
        .map_err(|_| format!("Unsupported number locale: {language}"))?;
    let parts: Vec<_> = raw.split('.').collect();
    let integer: i64 = parts[0]
        .parse()
        .map_err(|_| "Number must have a decimal representation within i64 range")?;
    let mut formatted = integer.to_formatted_string(&locale);
    if let Some(fraction) = parts.get(1) {
        formatted.push_str(locale.decimal());
        formatted.push_str(fraction);
    }
    if base == "ar" {
        formatted = formatted
            .chars()
            .map(|c| match c {
                '0'..='9' => char::from_u32('٠' as u32 + c as u32 - '0' as u32).unwrap_or(c),
                _ => c,
            })
            .collect();
    }
    Ok(formatted)
}
fn date(language: &str, value: &Value) -> Result<String, String> {
    let text = value.as_str().ok_or("Date must be ISO 8601 text")?;
    let parsed = DateTime::parse_from_rfc3339(text)
        .map(|d| d.date_naive())
        .or_else(|_| NaiveDate::parse_from_str(text, "%Y-%m-%d"))
        .map_err(|_| "Date must be YYYY-MM-DD or RFC3339")?;
    let exact = language.replace('-', "_");
    let base = language.split('-').next().unwrap_or(language);
    let fallback = match base {
        "en" => "en_US",
        "es" => "es_ES",
        "fr" => "fr_FR",
        "de" => "de_DE",
        "ar" => "ar_SA",
        "he" => "he_IL",
        "pt" => "pt_PT",
        "it" => "it_IT",
        "ja" => "ja_JP",
        _ => &exact,
    };
    let locale = chrono::Locale::from_str(&exact)
        .or_else(|_| chrono::Locale::from_str(fallback))
        .map_err(|_| format!("Unsupported date locale: {language}"))?;
    Ok(parsed.format_localized("%x", locale).to_string())
}
struct Localized(&'static str);
impl HelperDef for Localized {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        let parameter = |i| {
            h.param(i)
                .map(|p| p.value())
                .ok_or_else(|| RenderError::new("Missing localization argument"))
        };
        let language = parameter(0)?
            .as_str()
            .ok_or_else(|| RenderError::new("Invalid locale"))?;
        let value = parameter(1)?;
        let result = match self.0 {
            "number" => number(language, value),
            "date" => date(language, value),
            _ => {
                let translated: Value = serde_json::from_str(value.as_str().unwrap_or("null"))
                    .map_err(|e| RenderError::new(e.to_string()))?;
                if let Some(s) = translated.as_str() {
                    Ok(s.to_string())
                } else {
                    let count = parameter(2)?;
                    let raw = count
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| count.to_string());
                    let base = language.split('-').next().unwrap_or(language);
                    let locale = base
                        .parse::<unic_langid::LanguageIdentifier>()
                        .map_err(|e| RenderError::new(e.to_string()))?;
                    let rules = PluralRules::create(locale, PluralRuleType::CARDINAL)
                        .map_err(RenderError::new)?;
                    let category = format!(
                        "{:?}",
                        rules.select(raw.as_str()).map_err(RenderError::new)?
                    )
                    .to_lowercase();
                    let text = translated
                        .get(&category)
                        .or_else(|| translated.get("other"))
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            RenderError::new("Plural translation requires an 'other' form")
                        })?;
                    Ok(text.replace(
                        "{count}",
                        &number(language, count).map_err(RenderError::new)?,
                    ))
                }
            }
        }
        .map_err(RenderError::new)?;
        Ok(ScopedJson::Derived(json!(result)))
    }
}
pub fn register(reg: &mut Handlebars<'_>) {
    reg.register_helper("studio_translate", Box::new(Localized("translate")));
    reg.register_helper("studio_number", Box::new(Localized("number")));
    reg.register_helper("studio_date", Box::new(Localized("date")));
}
/// Resolve authoring helpers while retaining all data expressions for Step runtime.
pub fn compile(
    source: &str,
    language: &str,
    default: &str,
    catalogs: &Value,
    diagnostics: &mut Vec<Value>,
) -> Result<String, String> {
    handlebars::Template::compile(source).map_err(|e| e.to_string())?;
    let tags = Regex::new(r#"\{\{\s*(t|number|date)\s+((?:"(?:\\.|[^"\\])*"|[^}])*)\}\}"#)
        .map_err(|e| e.to_string())?;
    let translation =
        Regex::new(r#"^"((?:\\.|[^"\\])*)"(?:\s+(.+?))?\s*$"#).map_err(|e| e.to_string())?;
    let mut error = None;
    let output=tags.replace_all(source,|capture:&regex::Captures<'_>|{
      let helper=&capture[1];let arguments=capture[2].trim();
      if helper!="t" {return format!("{{{{studio_{helper} {} {arguments}}}}}",json!(language));}
      let Some(parts)=translation.captures(arguments) else {error=Some("Use {{t \"key\"}} or {{t \"key\" count}}".into());return capture[0].to_string()};
      let key:String=serde_json::from_str(&format!("\"{}\"",&parts[1])).unwrap_or_default();
      let chain=language_chain(language,default);
      let resolved=chain.iter().find_map(|lang|catalogs.get(lang).and_then(|c|c.get(&key)).map(|v|(lang,v)));
      let (locale,value)=if let Some((locale,value))=resolved {
       if locale!=language {diagnostics.push(json!({"severity":"warning","code":"language-fallback","message":format!("Translation '{key}' uses {locale}")}));}
       (locale.as_str(),value.clone())
      }else{
       diagnostics.push(json!({"severity":"warning","code":"missing-translation","message":format!("Missing translation '{key}' for {language}")}));
       (language,json!(format!("[{key}]")))
      };
      if !value.is_string() && value.get("other").and_then(Value::as_str).is_none(){error=Some(format!("Translation '{key}' must be text or plural forms with 'other'"));}
      format!("{{{{studio_translate {} {} {}}}}}",json!(locale),json!(value.to_string()),parts.get(2).map(|p|p.as_str()).unwrap_or("null"))
    }).into_owned();
    if let Some(e) = error {
        Err(e)
    } else {
        Ok(output)
    }
}

/// Compile authoring translations using only helpers available in STEP 10.0.
/// Translation values are quoted native `with` arguments, so their braces cannot
/// become executable Handlebars and runtime voter data is never materialized.
pub fn compile_v10(
    source: &str,
    language: &str,
    default: &str,
    catalogs: &Value,
    diagnostics: &mut Vec<Value>,
    html: bool,
) -> Result<String, String> {
    use handlebars::template::{HelperTemplate, Parameter, TemplateElement};

    const HELPERS: &[&str] = &[
        "if",
        "unless",
        "each",
        "with",
        "lookup",
        "raw",
        "log",
        "eq",
        "ne",
        "gt",
        "gte",
        "lt",
        "lte",
        "and",
        "or",
        "not",
        "len",
        "sanitize_html",
        "format_u64",
        "format_percentage",
        "format_percentage_one",
        "format_dec_percentage",
        "format_date",
        "let",
        "expr",
        "datetime",
        "inc",
        "inc2",
        "to_json",
        "url_encode",
        "parse_i64",
        "divide",
        "multiply",
        "sum",
        "modulo",
        "next",
        "is_some",
    ];

    fn unsupported(name: &str) -> String {
        match name {
            "t" => "STEP 10.0 can export scalar {{t \"key\"}} translations only. Use a standalone translation with a literal key; keep dynamic values as native Handlebars fields.".into(),
            "number" | "studio_number" => "STEP 10.0 has no localized number helper. Use native {{format_u64 value}} for English integer formatting, or supply a preformatted runtime field.".into(),
            "date" | "studio_date" => "STEP 10.0 has no Studio date helper. Supply a preformatted runtime date (such as issue_date), or use the native datetime helper with its supported input format.".into(),
            _ => format!("Helper '{name}' is unavailable in STEP 10.0. Replace it with a native STEP helper or a runtime data field before deployment."),
        }
    }
    fn check_parameter(parameter: &Parameter) -> Result<(), String> {
        if let Parameter::Subexpression(expression) = parameter {
            match expression.as_element() {
                TemplateElement::Expression(helper) | TemplateElement::HtmlExpression(helper) => {
                    check_helper(helper, false)?;
                }
                _ => return Err("Unsupported STEP 10.0 subexpression".into()),
            }
        }
        Ok(())
    }
    fn check_helper(helper: &HelperTemplate, translation: bool) -> Result<(), String> {
        let name = helper
            .name
            .as_name()
            .ok_or("Dynamic helper names are not supported by STEP 10.0 export")?;
        if name == "t" && translation {
            return Ok(());
        }
        if matches!(name, "t" | "number" | "date") || name.starts_with("studio_") {
            return Err(unsupported(name));
        }
        if (!helper.params.is_empty() || !helper.hash.is_empty()) && !HELPERS.contains(&name) {
            return Err(unsupported(name));
        }
        for parameter in helper.params.iter().chain(helper.hash.values()) {
            check_parameter(parameter)?;
        }
        Ok(())
    }

    struct Compiler<'a> {
        source: &'a str,
        line_offsets: Vec<usize>,
        language: &'a str,
        default: &'a str,
        catalogs: &'a Value,
        diagnostics: &'a mut Vec<Value>,
        html: bool,
        replacements: Vec<(usize, usize, String)>,
    }
    impl Compiler<'_> {
        fn walk(&mut self, template: &handlebars::Template) -> Result<(), String> {
            for (index, element) in template.elements.iter().enumerate() {
                match element {
                    TemplateElement::Expression(helper)
                    | TemplateElement::HtmlExpression(helper)
                        if helper.name.as_name() == Some("t") =>
                    {
                        let Some(Parameter::Literal(Value::String(key))) = helper.params.first()
                        else {
                            return Err(unsupported("t"));
                        };
                        if helper.params.len() > 2 || !helper.hash.is_empty() {
                            return Err(unsupported("t"));
                        }
                        let chain = language_chain(self.language, self.default);
                        let resolved = chain.iter().find_map(|locale| {
                            self.catalogs
                                .get(locale)
                                .and_then(|catalog| catalog.get(key))
                                .map(|value| (locale, value))
                        });
                        let text = match resolved {
                            Some((locale, Value::String(text))) => {
                                if locale != self.language {
                                    self.diagnostics.push(json!({"severity":"warning","code":"language-fallback","message":format!("Translation '{key}' uses {locale}")}));
                                }
                                text.clone()
                            }
                            Some(_) => return Err(format!("Translation '{key}' uses plural forms, which STEP 10.0 cannot evaluate. Choose a scalar translation and use a native count field, or provide preformatted runtime text.")),
                            None => {
                                self.diagnostics.push(json!({"severity":"warning","code":"missing-translation","message":format!("Missing translation '{key}' for {}", self.language)}));
                                format!("[{key}]")
                            }
                        };
                        let mapping = template
                            .mapping
                            .get(index)
                            .ok_or("Missing Handlebars source location")?;
                        let line_start = *self
                            .line_offsets
                            .get(mapping.0.saturating_sub(1))
                            .ok_or("Invalid Handlebars source line")?;
                        let column = self.source[line_start..]
                            .char_indices()
                            .nth(mapping.1.saturating_sub(1))
                            .map(|(offset, _)| offset)
                            .unwrap_or(0);
                        let start = line_start + column;
                        let end = tag_end(self.source, start)?;
                        let tag = &self.source[start..end];
                        let leading_trim = if tag.starts_with("{{~") || tag.starts_with("{{{~") {
                            "~"
                        } else {
                            ""
                        };
                        let trailing_trim = if tag.ends_with("~}}") || tag.ends_with("~}}}") {
                            "~"
                        } else {
                            ""
                        };
                        let value = serde_json::to_string(&text).map_err(|e| e.to_string())?;
                        let field =
                            if self.html && matches!(element, TemplateElement::Expression(_)) {
                                "{{this}}"
                            } else {
                                "{{{this}}}"
                            };
                        self.replacements.push((start, end, format!("{{{{{leading_trim}#with {value}}}}}{field}{{{{/with{trailing_trim}}}}}")));
                    }
                    TemplateElement::Expression(helper)
                    | TemplateElement::HtmlExpression(helper)
                    | TemplateElement::HelperBlock(helper) => {
                        check_helper(helper, false)?;
                        if let Some(body) = &helper.template {
                            self.walk(body)?;
                        }
                        if let Some(body) = &helper.inverse {
                            self.walk(body)?;
                        }
                    }
                    TemplateElement::DecoratorExpression(decorator)
                    | TemplateElement::DecoratorBlock(decorator) => {
                        if decorator.name.as_name() != Some("inline") {
                            return Err(
                                "STEP 10.0 only supports the native inline decorator".into()
                            );
                        }
                        for parameter in &decorator.params {
                            check_parameter(parameter)?;
                        }
                        if let Some(body) = &decorator.template {
                            self.walk(body)?;
                        }
                    }
                    TemplateElement::PartialExpression(partial)
                    | TemplateElement::PartialBlock(partial) => {
                        check_parameter(&partial.name)?;
                        for parameter in partial.params.iter().chain(partial.hash.values()) {
                            check_parameter(parameter)?;
                        }
                        if let Some(body) = &partial.template {
                            self.walk(body)?;
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        }
    }
    fn tag_end(source: &str, start: usize) -> Result<usize, String> {
        let text = source
            .get(start..)
            .ok_or("Invalid Handlebars source position")?;
        let braces = if text.starts_with("{{{") {
            3
        } else if text.starts_with("{{") {
            2
        } else {
            return Err("Invalid Handlebars translation position".into());
        };
        let mut quote = None;
        let mut escaped = false;
        for (offset, character) in text[braces..].char_indices() {
            let offset = braces + offset;
            if let Some(delimiter) = quote {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == delimiter {
                    quote = None;
                }
            } else if matches!(character, '\'' | '"') {
                quote = Some(character);
            } else if text[offset..].starts_with(&"}".repeat(braces)) {
                return Ok(start + offset + braces);
            }
        }
        Err("Unterminated translation expression".into())
    }

    let template = handlebars::Template::compile(source).map_err(|e| e.to_string())?;
    let mut line_offsets = vec![0];
    line_offsets.extend(source.match_indices('\n').map(|(index, _)| index + 1));
    let mut compiler = Compiler {
        source,
        line_offsets,
        language,
        default,
        catalogs,
        diagnostics,
        html,
        replacements: Vec::new(),
    };
    compiler.walk(&template)?;
    compiler.replacements.sort_by_key(|item| item.0);
    let mut output = source.to_string();
    for (start, end, replacement) in compiler.replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    handlebars::Template::compile(&output).map_err(|e| e.to_string())?;
    Ok(output)
}
