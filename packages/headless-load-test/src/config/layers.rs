// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

/// The `layers.yaml` config: tenants to provision, each with the election
/// events and vote load to run against them.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Layers {
    pub tenants: Vec<TenantLayer>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct TenantLayer {
    pub slug: String,
    pub election_events: Vec<ElectionEventLayer>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ElectionEventLayer {
    pub voters: u32,
    #[serde(deserialize_with = "deserialize_positive_f64")]
    pub votes_per_second: f64,
    #[serde(deserialize_with = "deserialize_duration")]
    pub duration: Duration,
}

pub fn load_layers(path: &Path) -> Result<Layers> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read layers file at {}", path.display()))?;
    parse_layers_str(&contents)
        .with_context(|| format!("failed to parse layers file at {}", path.display()))
}

fn parse_layers_str(contents: &str) -> Result<Layers> {
    Ok(serde_yaml::from_str(contents)?)
}

fn deserialize_positive_f64<'de, D>(deserializer: D) -> std::result::Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = f64::deserialize(deserializer)?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "votes_per_second must be a positive number, got {value}"
        )))
    }
}

/// Accepts a bare integer (seconds) or a number suffixed with `s`, `m`, or
/// `h`, e.g. `30s`, `5m`, `1h`.
fn deserialize_duration<'de, D>(deserializer: D) -> std::result::Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_duration(&raw).map_err(serde::de::Error::custom)
}

fn parse_duration(raw: &str) -> std::result::Result<Duration, String> {
    let trimmed = raw.trim();
    let (number, unit) = match trimmed.chars().last() {
        Some(c) if c.is_ascii_alphabetic() => (&trimmed[..trimmed.len() - c.len_utf8()], c),
        _ => (trimmed, 's'),
    };
    let amount: u64 = number.parse().map_err(|_| {
        format!(
            "invalid duration `{trimmed}`: expected a number optionally \
             followed by s, m, or h"
        )
    })?;
    let seconds = match unit {
        's' => amount,
        'm' => amount * 60,
        'h' => amount * 3600,
        other => {
            return Err(format!(
                "invalid duration `{trimmed}`: unknown unit `{other}`, \
                 expected s, m, or h"
            ))
        }
    };
    Ok(Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
tenants:
  - slug: acme
    election_events:
      - voters: 100
        votes_per_second: 5.0
        duration: 10m
  - slug: initech
    election_events:
      - voters: 50
        votes_per_second: 2.5
        duration: 30s
"#;

    #[test]
    fn a_valid_layers_file_parses() {
        let layers = parse_layers_str(VALID).unwrap();
        assert_eq!(layers.tenants.len(), 2);
        assert_eq!(layers.tenants[0].slug, "acme");
        assert_eq!(layers.tenants[0].election_events[0].voters, 100);
        assert_eq!(layers.tenants[0].election_events[0].votes_per_second, 5.0);
        assert_eq!(
            layers.tenants[0].election_events[0].duration,
            Duration::from_secs(600)
        );
        assert_eq!(
            layers.tenants[1].election_events[0].duration,
            Duration::from_secs(30)
        );
    }

    #[test]
    fn a_missing_required_field_is_rejected() {
        let missing_slug = r#"
tenants:
  - election_events:
      - voters: 100
        votes_per_second: 5.0
        duration: 10m
"#;
        assert!(parse_layers_str(missing_slug).is_err());
    }

    #[test]
    fn a_negative_votes_per_second_is_rejected() {
        let negative = VALID.replace("votes_per_second: 5.0", "votes_per_second: -1.0");
        let err = parse_layers_str(&negative).unwrap_err();
        assert!(
            err.to_string().contains("votes_per_second"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn a_zero_votes_per_second_is_rejected() {
        let zero = VALID.replace("votes_per_second: 5.0", "votes_per_second: 0");
        assert!(parse_layers_str(&zero).is_err());
    }

    #[test]
    fn an_invalid_duration_unit_is_rejected() {
        let bad = VALID.replace("duration: 10m", "duration: 10x");
        let err = parse_layers_str(&bad).unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("duration"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn a_non_numeric_duration_is_rejected() {
        let bad = VALID.replace("duration: 10m", "duration: soon");
        assert!(parse_layers_str(&bad).is_err());
    }

    #[test]
    fn load_layers_reports_a_missing_file_clearly() {
        let err = load_layers(Path::new("/nonexistent/path/layers.yaml")).unwrap_err();
        assert!(
            err.to_string().contains("layers file"),
            "unexpected error: {err}"
        );
    }
}
