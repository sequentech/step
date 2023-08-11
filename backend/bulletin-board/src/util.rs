// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt::Debug;
use std::str::FromStr;
use strand::signature::{
    StrandSignaturePk as PublicKey, StrandSignatureSk as SecretKey,
};
use strand::util::StrandError;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::reload;
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

use crate::Error;

pub type BoardUuid = String;

lazy_static! {
    static ref IDENTIFIER_RE: Regex = Regex::new(r"^[\da-zA-Z-_]+$").unwrap();
}

pub fn is_valid_identifier(data: &str) -> bool {
    (*IDENTIFIER_RE).is_match(data)
}

pub fn init_log() -> Result<(), String> {
    let tracing_level_str =
        env::var("TRACING_LEVEL").unwrap_or_else(|_| String::from("info"));

    let hierarchical_layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(3)
        .with_thread_names(false)
        .with_thread_ids(false)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(false);

    let level_filter = LevelFilter::from_str(&tracing_level_str)
        .map_err(|_| "couldn't parse level filter")?;
    let (filter, _) = reload::Layer::new(level_filter);
    let subscriber = Registry::default().with(filter).with(hierarchical_layer);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|_| "tracing::subscriber::set_global_default() call failed")?;
    tracing_log::LogTracer::init()
        .map_err(|_| "tracing_log::LogTracer::init() call failed")?;
    info!(tracing_level_str);

    Ok(())
}

pub type Timestamp = u64;

pub trait Now {
    fn now() -> Timestamp;
}

impl Now for Timestamp {
    fn now() -> Timestamp {
        instant::now() as Timestamp
    }
}

pub trait Validate {
    fn validate(&self) -> Result<(), Error>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyPairConfig {
    // base64 encoding of a SecretKey serialization
    pub secret_key: String,

    // base64 encoding of a PublicKey serialization
    pub public_key: String,
}

impl TryFrom<SecretKey> for KeyPairConfig {
    type Error = StrandError;

    fn try_from(secret_key: SecretKey) -> Result<Self, Self::Error> {
        Ok(KeyPairConfig {
            public_key: PublicKey::from(&secret_key).try_into()?,
            secret_key: secret_key.try_into()?,
        })
    }
}

#[cfg(test)]
#[derive(Debug, Default, Serialize, Deserialize)]
struct TestSuite {
    tests: Vec<TestCase>,
}
#[cfg(test)]
#[derive(Debug, Default, Serialize, Deserialize)]
struct TestCase {
    description: String,
    data: String,
    parse_error: Option<String>,
    validate_error: Option<String>,
}

// Generic toml+json-based test suite validator
#[cfg(test)]
pub fn validate_test_suite<T, F, U>(test_suite_str: &str, validate_fn: F)
where
    for<'a> T: Deserialize<'a> + Debug,
    U: Debug,
    F: Fn(T) -> Result<(), U>,
{
    let test_suite: TestSuite = toml::from_str(test_suite_str).unwrap();

    for test_case in test_suite.tests {
        let test_description = test_case.description;
        let deserialize_result: Result<T, _> =
            serde_json::from_str(&test_case.data);

        // check parse errors
        if test_case.parse_error.is_none() {
            assert!(
                deserialize_result.is_ok(),
                "test case with description `{test_description}`: \
                {deserialize_result:?}"
            );
        } else {
            let expected_error = test_case.parse_error.unwrap();
            assert!(
                deserialize_result.is_err(),
                "test case with description `{test_description}`: Expected \
                parse error `{expected_error}` but got \
                `{deserialize_result:?}` instead"
            );
            assert_eq!(
                format!("{deserialize_result:?}"),
                expected_error,
                "test case with description `{test_description}`"
            );
            continue;
        }

        // check validate errors
        let validate_result: Result<(), U> =
            validate_fn(deserialize_result.unwrap());
        if test_case.validate_error.is_none() {
            assert!(
                validate_result.is_ok(),
                "test case with description `{test_description}`: \
                {validate_result:?}"
            );
        } else {
            let expected_error = test_case.validate_error.unwrap();
            assert!(
                validate_result.is_err(),
                "test case with description `{test_description}`: Expected \
                validation error `{expected_error}` but got \
                `{validate_result:?}` instead"
            );
            assert_eq!(
                format!("{validate_result:?}"),
                expected_error,
                "test case with description `{test_description}`"
            );
        }
    }
}
