// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use strum_macros::{Display, EnumString};

/// The template variable through which an election event's number format
/// reaches the report helpers.
pub const NUMBER_FORMAT_POLICY_VARIABLE: &str = "number_format_policy";

const NO_BREAK_SPACE: &str = "\u{a0}";
const RIGHT_SINGLE_QUOTATION_MARK: &str = "\u{2019}";

/// How an election event writes numbers such as vote counts and
/// percentages: its thousands separator, then its decimal separator.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    EnumString,
    Display,
)]
pub enum NumberFormatPolicy {
    /// 1,234,567.89
    #[default]
    #[strum(serialize = "comma-period")]
    #[serde(rename = "comma-period")]
    CommaPeriod,
    /// 1.234.567,89
    #[strum(serialize = "period-comma")]
    #[serde(rename = "period-comma")]
    PeriodComma,
    /// 1 234 567,89, grouped with no-break spaces.
    #[strum(serialize = "space-comma")]
    #[serde(rename = "space-comma")]
    SpaceComma,
    /// 1 234 567.89, grouped with no-break spaces.
    #[strum(serialize = "space-period")]
    #[serde(rename = "space-period")]
    SpacePeriod,
    /// 1’234’567.89
    #[strum(serialize = "apostrophe-period")]
    #[serde(rename = "apostrophe-period")]
    ApostrophePeriod,
}

impl NumberFormatPolicy {
    pub fn group_separator(self) -> &'static str {
        match self {
            Self::CommaPeriod => ",",
            Self::PeriodComma => ".",
            Self::SpaceComma | Self::SpacePeriod => NO_BREAK_SPACE,
            Self::ApostrophePeriod => RIGHT_SINGLE_QUOTATION_MARK,
        }
    }

    pub fn decimal_separator(self) -> &'static str {
        match self {
            Self::PeriodComma | Self::SpaceComma => ",",
            Self::CommaPeriod | Self::SpacePeriod | Self::ApostrophePeriod => {
                "."
            }
        }
    }

    /// `value` with its digits grouped in thousands.
    pub fn format_integer(self, value: impl Into<i128>) -> String {
        let value: i128 = value.into();
        let grouped = group_digits(
            &value.unsigned_abs().to_string(),
            self.group_separator(),
        );
        if value < 0 {
            format!("-{grouped}")
        } else {
            grouped
        }
    }

    /// `value` rounded to `decimals` places, its integer part grouped in
    /// thousands. Infinities and NaN are written as Rust writes them.
    pub fn format_decimal(self, value: f64, decimals: usize) -> String {
        if !value.is_finite() {
            return value.to_string();
        }
        let fixed = format!("{:.*}", decimals, value.abs());
        let (integer, fraction) =
            fixed.split_once('.').unwrap_or((fixed.as_str(), ""));
        let is_zero = fixed.chars().all(|c| c == '0' || c == '.');
        let mut formatted = String::new();
        if value.is_sign_negative() && !is_zero {
            formatted.push('-');
        }
        formatted.push_str(&group_digits(integer, self.group_separator()));
        if !fraction.is_empty() {
            formatted.push_str(self.decimal_separator());
            formatted.push_str(fraction);
        }
        formatted
    }
}

/// Reads an optional policy, taking a code this version does not know as no
/// policy, so that one unknown value cannot make a whole election event
/// presentation unreadable.
pub fn deserialize_lenient_number_format_policy<'de, D>(
    deserializer: D,
) -> Result<Option<NumberFormatPolicy>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| serde_json::from_value(value).ok()))
}

fn group_digits(digits: &str, separator: &str) -> String {
    let length = digits.len();
    let mut grouped = String::with_capacity(length + length / 3 * 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (length - index) % 3 == 0 {
            grouped.push_str(separator);
        }
        grouped.push(digit);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const ALL: [NumberFormatPolicy; 5] = [
        NumberFormatPolicy::CommaPeriod,
        NumberFormatPolicy::PeriodComma,
        NumberFormatPolicy::SpaceComma,
        NumberFormatPolicy::SpacePeriod,
        NumberFormatPolicy::ApostrophePeriod,
    ];

    #[test]
    fn every_policy_writes_its_sample() {
        let samples: Vec<String> = ALL
            .iter()
            .map(|policy| {
                format!(
                    "{}{}89",
                    policy.format_integer(1_234_567u64),
                    policy.decimal_separator()
                )
            })
            .collect();
        assert_eq!(
            samples,
            vec![
                "1,234,567.89",
                "1.234.567,89",
                "1\u{a0}234\u{a0}567,89",
                "1\u{a0}234\u{a0}567.89",
                "1\u{2019}234\u{2019}567.89",
            ]
        );
    }

    #[test]
    fn the_default_is_the_comma_grouping_the_reports_always_used() {
        assert_eq!(
            NumberFormatPolicy::default(),
            NumberFormatPolicy::CommaPeriod
        );
    }

    #[test]
    fn integers_are_grouped_in_thousands() {
        let policy = NumberFormatPolicy::CommaPeriod;
        assert_eq!(policy.format_integer(0u64), "0");
        assert_eq!(policy.format_integer(999u64), "999");
        assert_eq!(policy.format_integer(1_000u64), "1,000");
        assert_eq!(policy.format_integer(12_000_000u64), "12,000,000");
        assert_eq!(policy.format_integer(8_589_934_591u64), "8,589,934,591");
        assert_eq!(
            policy.format_integer(u64::MAX),
            "18,446,744,073,709,551,615"
        );
        assert_eq!(policy.format_integer(-1_234i64), "-1,234");
        assert_eq!(
            policy.format_integer(i64::MIN),
            "-9,223,372,036,854,775,808"
        );
    }

    #[test]
    fn decimals_round_and_use_the_policy_separators() {
        let period_comma = NumberFormatPolicy::PeriodComma;
        assert_eq!(period_comma.format_decimal(1234.5, 2), "1.234,50");
        assert_eq!(period_comma.format_decimal(99.999, 2), "100,00");
        assert_eq!(period_comma.format_decimal(0.125, 1), "0,1");
        assert_eq!(period_comma.format_decimal(1234.6, 0), "1.235");
        assert_eq!(
            NumberFormatPolicy::CommaPeriod.format_decimal(-1234.5, 1),
            "-1,234.5"
        );
        assert_eq!(
            NumberFormatPolicy::CommaPeriod.format_decimal(-0.001, 2),
            "0.00"
        );
        assert_eq!(
            NumberFormatPolicy::CommaPeriod.format_decimal(f64::NAN, 2),
            "NaN"
        );
    }

    #[cfg(feature = "default_features")]
    #[test]
    fn a_presentation_with_an_unknown_policy_still_reads() {
        use crate::ballot::ElectionEventPresentation;
        let read = |value: serde_json::Value| {
            serde_json::from_value::<ElectionEventPresentation>(value)
                .expect("presentation reads")
                .number_format_policy
        };
        assert_eq!(read(serde_json::json!({})), None);
        assert_eq!(
            read(serde_json::json!({"number_format_policy": null})),
            None
        );
        assert_eq!(
            read(serde_json::json!({"number_format_policy": "period-comma"})),
            Some(NumberFormatPolicy::PeriodComma)
        );
        assert_eq!(
            read(serde_json::json!({"number_format_policy": "dot-space"})),
            None
        );
        assert_eq!(read(serde_json::json!({"number_format_policy": 3})), None);
    }

    #[test]
    fn code_names_round_trip_through_serde_and_strum() {
        for policy in ALL {
            let code = serde_json::to_value(policy).unwrap();
            assert_eq!(code, serde_json::json!(policy.to_string()));
            assert_eq!(
                serde_json::from_value::<NumberFormatPolicy>(code).unwrap(),
                policy
            );
            assert_eq!(
                NumberFormatPolicy::from_str(&policy.to_string()).unwrap(),
                policy
            );
        }
        assert_eq!(
            serde_json::to_value(NumberFormatPolicy::ApostrophePeriod).unwrap(),
            serde_json::json!("apostrophe-period")
        );
    }
}
