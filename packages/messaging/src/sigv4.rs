// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! AWS Signature Version 4 over a received request, for the fake provider
//! (development only): it takes an SNS or SES request only when it was
//! signed with the account's stored secret, as AWS does (review R7-19).
//! Also signs a request the same way, for tests that make one by hand.
//!
//! Only the header form (`Authorization: AWS4-HMAC-SHA256 ...`) is read;
//! presigned query strings are refused. Path segments are encoded the way
//! the SDK signs every service but S3 (each segment encoded again).

use chrono::{DateTime, NaiveDateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

const ALGORITHM: &str = "AWS4-HMAC-SHA256";
const DATE_HEADER: &str = "x-amz-date";
const DATE_FORMAT: &str = "%Y%m%dT%H%M%SZ";
/// How far the request's time may be from now, as AWS allows.
pub const MAX_SKEW_MINUTES: i64 = 15;

/// Why a request's signature isn't taken. Never carries a header value.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
pub enum SigV4Refusal {
    #[strum(serialize = "No AWS Signature Version 4 authorization")]
    NOT_SIGNED,
    #[strum(serialize = "The request's time is missing or too far from now")]
    STALE,
    #[strum(serialize = "The request signature doesn't match")]
    MISMATCH,
}

/// A received request, as it came on the wire.
pub struct ReceivedRequest<'a> {
    pub method: &'a str,
    /// The path, percent-encoded as received, without the query.
    pub path: &'a str,
    /// The raw query, without `?`; empty when none.
    pub query: &'a str,
    /// Every header as received. Repeated names are joined with `,`.
    pub headers: &'a [(String, String)],
    pub body: &'a [u8],
}

/// The parts of an `Authorization` header.
struct Authorization<'a> {
    access_key_id: &'a str,
    date: &'a str,
    region: &'a str,
    service: &'a str,
    signed_headers: Vec<&'a str>,
    signature: &'a str,
}

fn parse_authorization(value: &str) -> Option<Authorization<'_>> {
    let rest = value.trim().strip_prefix(ALGORITHM)?.trim();
    let mut credential = None;
    let mut signed_headers = None;
    let mut signature = None;
    for part in rest.split(',') {
        let (name, value) = part.trim().split_once('=')?;
        match name {
            "Credential" => credential = Some(value),
            "SignedHeaders" => signed_headers = Some(value),
            "Signature" => signature = Some(value),
            _ => {}
        }
    }
    let mut scope = credential?.split('/');
    let authorization = Authorization {
        access_key_id: scope.next()?,
        date: scope.next()?,
        region: scope.next()?,
        service: scope.next()?,
        signed_headers: signed_headers?.split(';').collect(),
        signature: signature?,
    };
    (scope.next()? == "aws4_request" && scope.next().is_none()).then_some(authorization)
}

/// The access key ID the request was signed with, unchecked.
pub fn access_key_id(authorization: &str) -> Option<&str> {
    parse_authorization(authorization)
        .map(|parsed| parsed.access_key_id)
        .filter(|id| !id.is_empty())
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<String> {
    let values: Vec<&'a str> = headers
        .iter()
        .filter(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
        .collect();
    (!values.is_empty()).then(|| values.join(","))
}

/// AWS's URI encoding: everything but unreserved characters, `/` kept
/// when `path` is set.
fn uri_encode(value: &str, path: bool) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            b'/' if path => encoded.push('/'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn percent_decode(value: &str) -> String {
    urlencoding::decode(&value.replace('+', "%20"))
        .map(|decoded| decoded.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

fn canonical_query(query: &str) -> String {
    let mut pairs: Vec<(String, String)> = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (
                uri_encode(&percent_decode(name), false),
                uri_encode(&percent_decode(value), false),
            )
        })
        .collect();
    pairs.sort();
    pairs
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn canonical_value(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn hmac(key: &[u8], data: &str) -> Vec<u8> {
    // HMAC accepts keys of any length.
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("any key length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// The hex signature of `request` for these credentials and scope, over
/// `signed_headers` (lowercase), with `amz_date` as its time.
#[allow(clippy::too_many_arguments)]
fn signature(
    request: &ReceivedRequest<'_>,
    signed_headers: &[&str],
    amz_date: &str,
    date: &str,
    region: &str,
    service: &str,
    secret: &str,
) -> Option<String> {
    let mut names: Vec<String> = signed_headers
        .iter()
        .map(|n| n.to_ascii_lowercase())
        .collect();
    names.sort();
    let mut canonical_headers = String::new();
    for name in &names {
        let value = header(request.headers, name)?;
        canonical_headers.push_str(&format!("{name}:{}\n", canonical_value(&value)));
    }
    let path = if request.path.is_empty() {
        "/"
    } else {
        request.path
    };
    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        request.method.to_ascii_uppercase(),
        uri_encode(path, true),
        canonical_query(request.query),
        canonical_headers,
        names.join(";"),
        hex::encode(Sha256::digest(request.body)),
    );
    let string_to_sign = format!(
        "{ALGORITHM}\n{amz_date}\n{date}/{region}/{service}/aws4_request\n{}",
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );
    let key = [date, region, service, "aws4_request"]
        .iter()
        .fold(format!("AWS4{secret}").into_bytes(), |key, part| {
            hmac(&key, part)
        });
    Some(hex::encode(hmac(&key, &string_to_sign)))
}

/// Whether `request` was signed with `secret`, at a time within
/// [`MAX_SKEW_MINUTES`] of `now`. The signature is compared in constant
/// time.
pub fn verify(
    request: &ReceivedRequest<'_>,
    secret: &str,
    now: DateTime<Utc>,
) -> Result<(), SigV4Refusal> {
    let authorization = header(request.headers, "authorization").ok_or(SigV4Refusal::NOT_SIGNED)?;
    let parsed = parse_authorization(&authorization).ok_or(SigV4Refusal::NOT_SIGNED)?;
    let amz_date = header(request.headers, DATE_HEADER).ok_or(SigV4Refusal::STALE)?;
    let at = NaiveDateTime::parse_from_str(amz_date.trim(), DATE_FORMAT)
        .map_err(|_| SigV4Refusal::STALE)?
        .and_utc();
    if (now - at).num_minutes().abs() > MAX_SKEW_MINUTES
        || !amz_date.trim().starts_with(parsed.date)
    {
        return Err(SigV4Refusal::STALE);
    }
    // The host and the time must be signed, as AWS requires.
    if !["host", DATE_HEADER]
        .iter()
        .all(|required| parsed.signed_headers.contains(required))
    {
        return Err(SigV4Refusal::MISMATCH);
    }
    let expected = signature(
        request,
        &parsed.signed_headers,
        amz_date.trim(),
        parsed.date,
        parsed.region,
        parsed.service,
        secret,
    )
    .ok_or(SigV4Refusal::MISMATCH)?;
    let (expected, presented) = (expected.as_bytes(), parsed.signature.as_bytes());
    let same = expected.len() == presented.len()
        && expected
            .iter()
            .zip(presented)
            .fold(0u8, |difference, (a, b)| difference | (a ^ b))
            == 0;
    same.then_some(()).ok_or(SigV4Refusal::MISMATCH)
}

/// The `Authorization` value that signs `request` (which must carry the
/// `host` and `x-amz-date` headers) over every header it has.
pub fn authorization(
    request: &ReceivedRequest<'_>,
    access_key_id: &str,
    secret: &str,
    region: &str,
    service: &str,
) -> Option<String> {
    let amz_date = header(request.headers, DATE_HEADER)?;
    let date = amz_date.get(..8)?.to_string();
    let mut names: Vec<String> = request
        .headers
        .iter()
        .map(|(name, _)| name.to_ascii_lowercase())
        .filter(|name| name != "authorization")
        .collect();
    names.sort();
    names.dedup();
    let signed: Vec<&str> = names.iter().map(String::as_str).collect();
    let signature = signature(request, &signed, &amz_date, &date, region, service, secret)?;
    Some(format!(
        "{ALGORITHM} Credential={access_key_id}/{date}/{region}/{service}/aws4_request, \
         SignedHeaders={}, Signature={signature}",
        names.join(";")
    ))
}

/// `x-amz-date` for `at`.
pub fn amz_date(at: DateTime<Utc>) -> String {
    at.format(DATE_FORMAT).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    /// AWS's published `get-vanilla` case of the Signature Version 4 test
    /// suite.
    #[test]
    fn the_published_test_vector_verifies() {
        let headers = headers(&[
            ("Host", "example.amazonaws.com"),
            ("X-Amz-Date", "20150830T123600Z"),
            (
                "Authorization",
                "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
                 SignedHeaders=host;x-amz-date, \
                 Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31",
            ),
        ]);
        let request = ReceivedRequest {
            method: "GET",
            path: "/",
            query: "",
            headers: &headers,
            body: b"",
        };
        let at = NaiveDateTime::parse_from_str("20150830T123600Z", DATE_FORMAT)
            .unwrap()
            .and_utc();
        assert_eq!(
            verify(&request, "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY", at),
            Ok(())
        );
        assert_eq!(
            verify(&request, "another secret", at),
            Err(SigV4Refusal::MISMATCH)
        );
        assert_eq!(
            verify(
                &request,
                "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
                at + chrono::Duration::minutes(16)
            ),
            Err(SigV4Refusal::STALE)
        );
    }

    #[test]
    fn a_signed_request_verifies_only_unchanged_and_with_its_secret() {
        let now = Utc::now();
        let unsigned = headers(&[
            ("Host", "harvest:8400"),
            ("Content-Type", "application/x-www-form-urlencoded"),
            ("X-Amz-Date", &amz_date(now)),
        ]);
        let request = |headers, body| ReceivedRequest {
            method: "POST",
            path: "/fake-provider/aws/",
            query: "",
            headers,
            body,
        };
        let value = authorization(
            &request(&unsigned, b"Action=Publish"),
            "AKIDEXAMPLE",
            "secret",
            "eu-west-1",
            "sns",
        )
        .unwrap();
        assert_eq!(access_key_id(&value), Some("AKIDEXAMPLE"));
        let mut signed = unsigned.clone();
        signed.push(("Authorization".to_string(), value));
        assert_eq!(
            verify(&request(&signed, b"Action=Publish"), "secret", now),
            Ok(())
        );
        assert_eq!(
            verify(&request(&signed, b"Action=Publish&x=1"), "secret", now),
            Err(SigV4Refusal::MISMATCH)
        );
        assert_eq!(
            verify(&request(&signed, b"Action=Publish"), "other", now),
            Err(SigV4Refusal::MISMATCH)
        );
        assert_eq!(
            verify(&request(&unsigned, b"Action=Publish"), "secret", now),
            Err(SigV4Refusal::NOT_SIGNED)
        );
    }
}
