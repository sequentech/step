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
