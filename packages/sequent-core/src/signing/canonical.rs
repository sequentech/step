// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The canonical payload: the exact bytes every signer of a request signs.

use super::code::signing_code;
use super::types::SigningAction;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::fmt;
use uuid::Uuid;

/// The first field of every payload, so a signature over it can't be
/// mistaken for a signature over anything else.
pub const SIGNING_DOMAIN: &str = "step-signing/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalJsonError {
    /// Floats have no single text form, so a payload must not hold one.
    Float(String),
    /// An integer beyond ±(2^53 − 1), which a JSON reader holding numbers as
    /// doubles (a browser) can't round-trip exactly.
    UnsafeInteger(String),
    /// Keys are ASCII, so their byte order is also RFC 8785's order.
    NonAsciiKey(String),
}

/// The largest integer every JSON reader holds exactly: 2^53 − 1.
pub const MAX_SAFE_INTEGER: i64 = (1 << 53) - 1;

impl fmt::Display for CanonicalJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CanonicalJsonError::Float(number) => {
                write!(f, "a canonical payload can't hold the float {number}")
            }
            CanonicalJsonError::UnsafeInteger(number) => write!(
                f,
                "a canonical payload can't hold the integer {number} exactly"
            ),
            CanonicalJsonError::NonAsciiKey(key) => {
                write!(f, "a canonical payload key must be ASCII: {key}")
            }
        }
    }
}

impl std::error::Error for CanonicalJsonError {}

/// UTF-8 JSON in the form of RFC 8785 (JCS), restricted so the simple
/// rules below are the whole of it: no whitespace; object keys ASCII only
/// and sorted by their bytes (for ASCII this is JCS's UTF-16 order); numbers
/// only integers within ±(2^53 − 1); strings escaped as serde_json does
/// (`\"`, `\\`, `\b \f \n \r \t`, other control characters as
/// lowercase `\u00xx`, everything else literal). The keys are sorted here
/// rather than taken from the map, because serde_json keeps insertion order
/// when its `preserve_order` feature is on.
pub fn canonical_json(value: &Value) -> Result<String, CanonicalJsonError> {
    let mut out = String::new();
    write_canonical(value, &mut out)?;
    Ok(out)
}

fn write_canonical(
    value: &Value,
    out: &mut String,
) -> Result<(), CanonicalJsonError> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => {
            out.push_str(&value.to_string())
        }
        Value::Number(number) => {
            if number.is_f64() {
                return Err(CanonicalJsonError::Float(number.to_string()));
            }
            let safe = number.as_i64().is_some_and(|integer| {
                (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&integer)
            });
            if !safe {
                return Err(CanonicalJsonError::UnsafeInteger(
                    number.to_string(),
                ));
            }
            out.push_str(&number.to_string());
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            if let Some(key) = map.keys().find(|key| !key.is_ascii()) {
                return Err(CanonicalJsonError::NonAsciiKey(key.clone()));
            }
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by_key(|(key, _)| *key);
            out.push('{');
            for (index, (key, item)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                write_canonical(item, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// A signed time: RFC 3339, UTC, whole seconds.
pub fn format_signing_time(time: &DateTime<Utc>) -> String {
    time.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// What a request's payload is built from. `subject` is the action's part
/// (see the `*Subject` types).
#[derive(Debug, Clone, PartialEq)]
pub struct SigningPayloadFields {
    pub tenant_id: String,
    pub election_event_id: String,
    pub request_id: Uuid,
    pub action: SigningAction,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub subject: Value,
    pub config_revision: Option<String>,
    pub rule_revision: i64,
    pub requested_by: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// A request's payload with its signing code, which is derived from the
/// request id and the subject, so it can't disagree with them.
#[derive(Debug, Clone, PartialEq)]
pub struct SigningPayload {
    fields: SigningPayloadFields,
    code: String,
}

impl SigningPayload {
    /// Fails when the subject has no canonical form.
    pub fn new(
        fields: SigningPayloadFields,
    ) -> Result<Self, CanonicalJsonError> {
        let code = signing_code(fields.request_id, &fields.subject)?;
        Ok(SigningPayload { fields, code })
    }

    pub fn fields(&self) -> &SigningPayloadFields {
        &self.fields
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn to_value(&self) -> Value {
        let fields = &self.fields;
        json!({
            "domain": SIGNING_DOMAIN,
            "tenant_id": fields.tenant_id,
            "election_event_id": fields.election_event_id,
            "request_id": fields.request_id.hyphenated().to_string(),
            "action": fields.action.to_string(),
            "election_id": fields.election_id,
            "area_id": fields.area_id,
            "subject": fields.subject,
            "config_revision": fields.config_revision,
            "rule_revision": fields.rule_revision,
            "requested_by": fields.requested_by,
            "created_at": format_signing_time(&fields.created_at),
            "expires_at": fields.expires_at.as_ref().map(format_signing_time),
            "code": self.code,
        })
    }

    /// The bytes every signer signs (`signing_request.canonical_payload`).
    pub fn canonical(&self) -> Result<String, CanonicalJsonError> {
        canonical_json(&self.to_value())
    }
}

#[cfg(test)]
#[path = "canonical_tests.rs"]
mod canonical_tests;
