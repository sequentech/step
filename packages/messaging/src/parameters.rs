// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Template parameters. Providers take them by position or by name; a
//! parameter written `@name=value` is named, any other is positional.

/// One parameter of an approved template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateParameter {
    Positional(String),
    Named { name: String, value: String },
}

impl TemplateParameter {
    pub fn parse(raw: &str) -> TemplateParameter {
        if let Some((name, value)) = raw.strip_prefix('@').and_then(|rest| rest.split_once('=')) {
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return TemplateParameter::Named {
                    name: name.to_string(),
                    value: value.to_string(),
                };
            }
        }
        TemplateParameter::Positional(raw.to_string())
    }

    pub fn value(&self) -> &str {
        match self {
            TemplateParameter::Positional(value) | TemplateParameter::Named { value, .. } => value,
        }
    }
}

pub fn parse_all(raw: &[String]) -> Vec<TemplateParameter> {
    raw.iter().map(|p| TemplateParameter::parse(p)).collect()
}

/// A Meta template `parameters` entry (WhatsApp and Messenger).
pub fn meta_text_parameter(parameter: &TemplateParameter) -> serde_json::Value {
    match parameter {
        TemplateParameter::Positional(value) => serde_json::json!({"type": "text", "text": value}),
        TemplateParameter::Named { name, value } => {
            serde_json::json!({"type": "text", "parameter_name": name, "text": value})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_parameters_need_the_at_prefix_and_an_identifier() {
        assert_eq!(
            TemplateParameter::parse("@first_name=Ana"),
            TemplateParameter::Named {
                name: "first_name".to_string(),
                value: "Ana".to_string()
            }
        );
        assert_eq!(
            TemplateParameter::parse("@note=a=b"),
            TemplateParameter::Named {
                name: "note".to_string(),
                value: "a=b".to_string()
            }
        );
        for positional in ["Ana", "@handle", "a=b", "@two words=x", "@=x"] {
            assert_eq!(
                TemplateParameter::parse(positional),
                TemplateParameter::Positional(positional.to_string())
            );
        }
    }

    #[test]
    fn meta_parameters_carry_the_name_when_named() {
        assert_eq!(
            meta_text_parameter(&TemplateParameter::parse("@order=5")),
            serde_json::json!({"type": "text", "parameter_name": "order", "text": "5"})
        );
        assert_eq!(
            meta_text_parameter(&TemplateParameter::parse("5")),
            serde_json::json!({"type": "text", "text": "5"})
        );
    }
}
