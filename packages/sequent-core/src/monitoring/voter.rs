// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the snapshot job keeps of one voter in one election: the facts the
//! settings say where to read, cleaned the way they say. Only derived
//! values are kept — a voter's age band, never their date of birth.

use super::config::{DimensionMapping, DimensionOrigin, Settings};
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;

/// Where a voter's facts are read from: their account's attributes, the
/// annotations of the election (the Post) and of their area.
#[derive(Debug, Clone, Copy)]
pub struct VoterSources<'a> {
    pub attributes: &'a BTreeMap<String, Vec<String>>,
    pub election_annotations: &'a BTreeMap<String, String>,
    pub area_annotations: &'a BTreeMap<String, String>,
}

/// One voter's derived facts. A fact that is missing, or that the settings
/// cannot read, is `None` (or absent from `dims`): it counts as Unknown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VoterFacts {
    pub region: Option<String>,
    pub country: Option<String>,
    /// The value of each configured voter dimension, keyed by its name.
    pub dims: BTreeMap<String, String>,
    pub pre_enrolled: bool,
}

/// The facts `settings` say to keep of a voter, with ages as of `on`, the
/// event's first day.
pub fn voter_facts(
    settings: &Settings,
    sources: VoterSources<'_>,
    on: NaiveDate,
) -> VoterFacts {
    let read = |mapping: &DimensionMapping| {
        dimension_value(mapping, raw_value(mapping, sources), on)
    };
    let pre_enrolled =
        first(sources.attributes, &settings.pre_enrolled.voter_attribute)
            .is_some_and(|value| value == settings.pre_enrolled.equals);
    VoterFacts {
        region: read(&settings.scope.region),
        country: read(&settings.scope.country),
        dims: settings
            .dimensions
            .iter()
            .filter_map(|(name, mapping)| {
                read(mapping).map(|value| (name.clone(), value))
            })
            .collect(),
        pre_enrolled,
    }
}

fn first<'a>(
    attributes: &'a BTreeMap<String, Vec<String>>,
    name: &str,
) -> Option<&'a str> {
    attributes
        .get(name)?
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
}

fn raw_value<'a>(
    mapping: &DimensionMapping,
    sources: VoterSources<'a>,
) -> Option<&'a str> {
    match mapping.origin()? {
        DimensionOrigin::VoterAttribute(name) => {
            first(sources.attributes, name)
        }
        DimensionOrigin::ElectionAnnotation(key) => {
            sources.election_annotations.get(key).map(String::as_str)
        }
        DimensionOrigin::AreaAnnotation(key) => {
            sources.area_annotations.get(key).map(String::as_str)
        }
    }
}

/// `raw` cleaned as `mapping` says: one part of a compound value, or the
/// age band of a date of birth as of `on`. `None` for a missing, empty or
/// unreadable value. Display labels are not applied: the value is the key
/// they are looked up by.
pub fn dimension_value(
    mapping: &DimensionMapping,
    raw: Option<&str>,
    on: NaiveDate,
) -> Option<String> {
    let mut value = raw?.trim();
    if let Some(split) = &mapping.split {
        value = value
            .split(split.separator.as_str())
            .nth(split.part)?
            .trim();
    }
    if value.is_empty() {
        return None;
    }
    if mapping.age_bands.is_empty() {
        return Some(value.to_string());
    }
    let age = age_on(parse_date(value)?, on)?;
    mapping
        .age_bands
        .iter()
        .find(|band| band.to.map_or(true, |to| age <= to))
        .map(|band| band.label.clone())
}

/// A date of birth as accounts hold it: `YYYY-MM-DD`, alone or starting a
/// timestamp, or `DD/MM/YYYY`.
fn parse_date(value: &str) -> Option<NaiveDate> {
    let head = value.get(..10).unwrap_or(value);
    NaiveDate::parse_from_str(head, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(head, "%d/%m/%Y"))
        .ok()
}

/// Whole years from `born` to `on`; `None` for a birth after `on`.
fn age_on(born: NaiveDate, on: NaiveDate) -> Option<u32> {
    if born > on {
        return None;
    }
    let mut age = on.year() - born.year();
    if (on.month(), on.day()) < (born.month(), born.day()) {
        age -= 1;
    }
    u32::try_from(age).ok()
}

#[cfg(test)]
#[path = "voter_tests.rs"]
mod voter_tests;
