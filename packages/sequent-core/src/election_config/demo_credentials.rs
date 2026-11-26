// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Ordinary workbook and draft paths must not retain account credentials.

use super::problem::{Code, Problem, Report};
use super::sheet::{Sheet, SHEET_ADMIN_USERS, SHEET_TRUSTEES};
use serde_json::Value;

#[cfg(feature = "election_config_xlsx")]
fn normalised(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// AWS static key material under any common spelling: case, separators and
/// prefixes are ignored (`AWS_SECRET_ACCESS_KEY`, `awsAccessKeyId`,
/// `secret-access-key`, `access_key`...). Deliberately broad: a false match
/// only routes a workbook to protected intake.
#[cfg(feature = "election_config_xlsx")]
fn aws_key_field(name: &str) -> bool {
    let name = normalised(name);
    name.contains("accesskeyid")
        || name.contains("secretaccesskey")
        || name.contains("sessiontoken")
        || name.ends_with("accesskey")
        || name.ends_with("secretkey")
}

#[cfg(feature = "election_config_xlsx")]
fn non_empty(value: &Value) -> bool {
    !value.is_null()
        && !matches!(value, Value::String(text) if text.trim().is_empty())
}

/// Nested JSON text is decoded too (bounded), as Blueprints may encode a
/// member as a string. Nested text that doesn't parse is ordinary text (a
/// template such as `{{name}}`, say): nothing downstream reads it as a key.
/// Too deep a nesting fails closed. Never returns or records a value.
#[cfg(feature = "election_config_xlsx")]
fn carries_aws_key(value: &Value, depth: usize) -> bool {
    if depth > 8 {
        return true;
    }
    match value {
        Value::Object(fields) => fields.iter().any(|(field, value)| {
            (aws_key_field(field) && non_empty(value))
                || carries_aws_key(value, depth + 1)
        }),
        Value::Array(items) => {
            items.iter().any(|value| carries_aws_key(value, depth + 1))
        }
        Value::String(text) => {
            let text = text.trim();
            (text.starts_with('{') || text.starts_with('['))
                && serde_json::from_str::<Value>(text)
                    .is_ok_and(|decoded| carries_aws_key(&decoded, depth + 1))
        }
        _ => false,
    }
}

/// Whether the Parameters sheet carries AWS key material: in the
/// `tenant_package` value (typed object or JSON text, as
/// `portable_messaging::validate_workbook_sheets` reads it), or in any row or
/// column named like an AWS key. An unreadable `tenant_package` text fails
/// closed. Value-free: the answer is a boolean only.
#[cfg(feature = "election_config_xlsx")]
fn parameters_carry_aws_keys(sheets: &[Sheet]) -> bool {
    sheets
        .iter()
        .filter(|sheet| {
            use super::sheet::{normalise_sheet_name, SHEET_PARAMETERS};
            sheet.key == SHEET_PARAMETERS
                || normalise_sheet_name(&sheet.name) == SHEET_PARAMETERS
        })
        .flat_map(|sheet| &sheet.rows)
        .any(|row| {
            let key = row.text("key").map(normalised).unwrap_or_default();
            let value = row.get("value");
            if aws_key_field(&key) && value.is_some_and(non_empty) {
                return true;
            }
            if row
                .cells
                .iter()
                .any(|(name, value)| aws_key_field(name) && non_empty(value))
            {
                return true;
            }
            match (key.as_str(), value) {
                ("tenantpackage", Some(Value::String(text)))
                    if !text.trim().is_empty() =>
                {
                    match serde_json::from_str::<Value>(text) {
                        Ok(decoded) => carries_aws_key(&decoded, 0),
                        Err(_) => true,
                    }
                }
                ("tenantpackage", Some(value)) => carries_aws_key(value, 0),
                _ => false,
            }
        })
}

pub(super) fn credential_column(header: &str) -> bool {
    let name: String = header
        .rsplit('.')
        .next()
        .unwrap_or(header)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    matches!(
        name.as_str(),
        "password"
            | "passwordsalt"
            | "hashedpassword"
            | "numofiterations"
            | "credentials"
            | "credentialdata"
            | "secretdata"
    )
}

fn contains_credential_value(name: &str, value: &Value) -> bool {
    if credential_column(name) {
        return !value.is_null()
            && !matches!(value, Value::String(text) if text.is_empty());
    }
    match value {
        Value::Object(fields) => fields
            .iter()
            .any(|(field, value)| contains_credential_value(field, value)),
        Value::Array(items) => items
            .iter()
            .any(|value| contains_credential_value("", value)),
        _ => false,
    }
}

/// The typed guard applies before a workbook becomes a draft or ordinary
/// bundle. No field value, and no authored column name, enters diagnostics.
pub fn reject_ordinary_credentials(sheets: &[Sheet]) -> Result<(), Report> {
    let mut report = Report::default();
    for sheet in sheets {
        if !matches!(sheet.key.as_str(), SHEET_ADMIN_USERS | SHEET_TRUSTEES)
            && !matches!(
                super::sheet::normalise_sheet_name(&sheet.name).as_str(),
                SHEET_ADMIN_USERS | SHEET_TRUSTEES
            )
        {
            continue;
        }
        for row in &sheet.rows {
            if row
                .cells
                .iter()
                .any(|(name, value)| contains_credential_value(name, value))
            {
                report.push(
                    Problem::error(
                        Code::NotApproved,
                        "demo account bootstrap",
                        "Admin or trustee passwords require an explicitly enabled demo package; no draft or delivery was created",
                    )
                    .at(&row.origin(Some("Password"))),
                );
            }
        }
    }
    if report.has_errors() {
        Err(report)
    } else {
        Ok(())
    }
}

/// A local, value-free probe before choosing between ordinary conversion and
/// protected intake. Unknown, oversized or unparseable inputs are not ordinary;
/// nor are admin/trustee passwords, or AWS key material in Parameters.
#[cfg(feature = "election_config_xlsx")]
pub fn is_ordinary_workbook(bytes: &[u8]) -> bool {
    use std::io::Cursor;
    if bytes.is_empty() || bytes.len() > 12 * 1024 * 1024 {
        return false;
    }
    let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(bytes)) else {
        return false;
    };
    if zip.len() > 4096 {
        return false;
    }
    let mut total = 0u64;
    for index in 0..zip.len() {
        let Ok(member) = zip.by_index(index) else {
            return false;
        };
        let Some(next) = total
            .checked_add(member.size())
            .filter(|n| *n <= 64 * 1024 * 1024)
        else {
            return false;
        };
        total = next;
    }
    drop(zip);
    super::xlsx::read_xlsx(bytes).is_ok_and(|w| {
        reject_ordinary_credentials(w.sheets()).is_ok()
            && !parameters_carry_aws_keys(w.sheets())
    })
}

#[cfg(all(test, feature = "election_config_xlsx"))]
#[test]
fn workbook_probe_is_value_free_and_does_not_require_an_election_plan() {
    use super::{paths::Cell, sheet::Workbook, xlsx_write::write_xlsx};
    for name in ["AdminUsers", "Trustees"] {
        let build = |value: &str| {
            let sheet = Sheet::from_grid(
                name,
                &[
                    vec![Cell::text("username"), Cell::text("Pass-word")],
                    vec![Cell::text("fixture-user"), Cell::text(value)],
                ],
            )
            .unwrap();
            write_xlsx(&Workbook::new(vec![sheet]).unwrap()).unwrap()
        };
        assert!(is_ordinary_workbook(&build("")));
        assert!(!is_ordinary_workbook(&build("synthetic-secret-canary")));
    }
    assert!(!is_ordinary_workbook(b"this is not an XLSX"));
    assert!(!is_ordinary_workbook(&vec![0u8; 12 * 1024 * 1024 + 1]));
}

#[cfg(all(test, feature = "election_config_xlsx"))]
#[test]
fn workbook_probe_routes_aws_key_material_in_parameters_to_protected_intake() {
    use super::{paths::Cell, sheet::Workbook, xlsx_write::write_xlsx};
    use serde_json::json;
    // Synthetic, format-valid, not AWS credentials.
    const KEY_ID: &str = "AKIASYNTHETICEXAMPLE";
    const SECRET: &str = "SyntheticSecretValueForTestsOnly00000000";
    let parameters = |rows: &[(&str, String)]| {
        let mut grid = vec![vec![Cell::text("key"), Cell::text("value")]];
        for (key, value) in rows {
            grid.push(vec![Cell::text(*key), Cell::text(value.as_str())]);
        }
        let sheet = Sheet::from_grid("Parameters", &grid).unwrap();
        write_xlsx(&Workbook::new(vec![sheet]).unwrap()).unwrap()
    };
    let package = |account: serde_json::Value| {
        json!({"format":"sequent.tenant-package/1","messagingAccounts":{
            "format":"sequent.tenant-messaging-accounts/1","accounts":[account]}})
        .to_string()
    };
    let ses = |extra: serde_json::Value| {
        let mut account = json!({"reference":"email","name":"Synthetic Email",
            "sender":{"provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"}});
        for (k, v) in extra.as_object().unwrap() {
            account[k] = v.clone();
        }
        account
    };
    let sns = json!({"reference":"sms","name":"Synthetic SMS",
        "sender":{"provider":"AWS_SNS","sender_id":"Demo","region":"us-east-1"},
        "aws_access_key_id": KEY_ID});
    let ordinary =
        |rows: &[(&str, String)]| is_ordinary_workbook(&parameters(rows));

    // AWS-free profiles, and empty key fields, stay ordinary.
    assert!(ordinary(&[("tenant_package", package(ses(json!({}))))]));
    assert!(ordinary(&[(
        "tenant_package",
        package(ses(json!({"credential_commitment": "ab".repeat(32)})))
    )]));
    assert!(ordinary(&[(
        "tenant_package",
        package(ses(
            json!({"aws_access_key_id": "", "aws_secret_access_key": null})
        ))
    )]));
    // A template-like nested text is plain text, not JSON to fail on.
    assert!(ordinary(&[(
        "tenant_package",
        json!({"format":"sequent.tenant-package/1",
            "tenantSettings":{"greeting":"{{voter.name}}"}})
        .to_string()
    )]));
    assert!(ordinary(&[("tenant_package", String::new())]));
    assert!(ordinary(&[("election_event_name", "Synthetic".into())]));

    // SES and SNS key material, complete or partial, any spelling or nesting.
    for protected in [
        vec![(
            "tenant_package",
            package(ses(
                json!({"aws_access_key_id": KEY_ID, "aws_secret_access_key": SECRET}),
            )),
        )],
        vec![(
            "tenant_package",
            package(ses(json!({"aws_secret_access_key": SECRET}))),
        )],
        vec![("tenant_package", package(sns.clone()))],
        vec![(
            "tenant_package",
            package(ses(json!({"credentials": {"AWS-Access-Key-Id": KEY_ID}}))),
        )],
        vec![(
            "tenant_package",
            package(ses(json!({"secretAccessKey": SECRET}))),
        )],
        // The accounts member itself encoded as JSON text.
        vec![(
            "tenant_package",
            json!({"format":"sequent.tenant-package/1",
                "messagingAccounts": package(sns.clone())})
            .to_string(),
        )],
        // An unreadable tenant_package text fails closed.
        vec![("tenant_package", "{\"messagingAccounts\": ".into())],
        // A Parameters row named like a key.
        vec![("aws_secret_access_key", SECRET.into())],
        vec![("Tenant Package", package(sns.clone()))],
    ] {
        assert!(!ordinary(&protected));
    }
    // Admin/trustee password behaviour is unchanged (see the test above).
}

/// A verifier is sensitive account bootstrap data. It is never a review,
/// Blueprint or workbook member. It may exist only in a protected service
/// sidecar and the final signed, encrypted delivery.
pub const DEMO_LOGIN_VERIFIERS_MEMBER: &str = "demo_login_verifiers.json";
pub const DEMO_LOGIN_VERIFIERS_FORMAT: &str = "sequent.demo-login-verifiers/1";
pub const DEMO_LOGIN_ITERATIONS: u32 = 600_000;
const MAX_ACCOUNTS: usize = 32;
const SALT_BYTES: usize = 16;
const VERIFIER_BYTES: usize = 32;

#[derive(Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DemoLoginPrincipal {
    pub username: String,
    pub email: Option<String>,
    pub group_name: String,
    pub trustee: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DemoLoginVerifier {
    pub username: String,
    pub email: Option<String>,
    pub group_name: String,
    pub trustee: Option<String>,
    pub iterations: u32,
    pub salt: String,
    pub verifier: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DemoLoginVerifiers {
    pub format: String,
    pub tenant_id: String,
    pub accounts: Vec<DemoLoginVerifier>,
}

fn refuse(message: &'static str) -> Report {
    Report::from_problem(Problem::error(
        Code::NotApproved,
        "demo account bootstrap",
        message,
    ))
}

fn synthetic(principal: &DemoLoginPrincipal) -> bool {
    let user = &principal.username;
    let role = &principal.group_name;
    !user.is_empty()
        && user.len() <= 64
        && user != "admin"
        && !user.starts_with("service-account-")
        && user
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && matches!(role.as_str(), "sbei" | "trustee")
        && principal.email.as_ref().is_none_or(|email| {
            let Some((name, domain)) = email.split_once('@') else {
                return false;
            };
            !name.is_empty()
                && !name.contains('@')
                && matches!(domain, "example.invalid" | "demo.example.invalid")
        })
        && match principal.trustee.as_deref() {
            Some(trustee) => trustee == user && role == "trustee",
            None => role != "trustee",
        }
}

/// Built-in, bounded eligible login roster. A configured installation
/// policy takes precedence. These aliases are not derived from an upload.
pub fn default_demo_login_roster() -> Vec<DemoLoginPrincipal> {
    let mut roster = Vec::with_capacity(18);
    for id in 1..=6 {
        let username = format!("trustee{id}");
        roster.push(DemoLoginPrincipal {
            email: Some(format!("{username}@demo.example.invalid")),
            group_name: "trustee".into(),
            trustee: Some(username.clone()),
            username,
        });
    }
    for post in ["bangkok", "dhaka", "tokyo", "abudhabi"] {
        for id in 1..=3 {
            roster.push(DemoLoginPrincipal {
                username: format!("sbei-{post}-{id}"),
                email: None,
                group_name: "sbei".into(),
                trustee: None,
            });
        }
    }
    roster
}

fn checked_roster(approved: &[DemoLoginPrincipal]) -> bool {
    !approved.is_empty()
        && approved.len() <= MAX_ACCOUNTS
        && approved.iter().enumerate().all(|(i, principal)| {
            synthetic(principal)
                && !approved[..i]
                    .iter()
                    .any(|other| other.username == principal.username)
        })
}

/// Validate the installation-owned fixture roster on both authoring and
/// receiving sides. No rejected identity or user-supplied value is reported.
pub fn verify_demo_login_roster(
    approved: &[DemoLoginPrincipal],
) -> Result<(), Report> {
    if checked_roster(approved) {
        Ok(())
    } else {
        Err(refuse("the installed demo login roster is invalid"))
    }
}

pub fn verify_demo_login_verifiers(
    value: &DemoLoginVerifiers,
    tenant_id: &str,
    approved: &[DemoLoginPrincipal],
) -> Result<(), Report> {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    let canonical = |text: &str, length: usize| {
        text.len() == length.div_ceil(3) * 4
            && B64.decode(text).is_ok_and(|bytes| {
                bytes.len() == length && B64.encode(bytes) == text
            })
    };
    if value.format != DEMO_LOGIN_VERIFIERS_FORMAT
        || value.tenant_id != tenant_id
        || !uuid::Uuid::parse_str(tenant_id)
            .is_ok_and(|id| id.to_string() == tenant_id)
        || !checked_roster(approved)
        || value.accounts.is_empty()
        || value.accounts.len() > approved.len()
        || value.accounts.iter().enumerate().any(|(index, account)| {
            value.accounts[..index]
                .iter()
                .any(|other| other.username == account.username)
                || !approved.iter().any(|principal| {
                    principal.username == account.username
                        && principal.email == account.email
                        && principal.group_name == account.group_name
                        && principal.trustee == account.trustee
                })
                || account.iterations != DEMO_LOGIN_ITERATIONS
                || !canonical(&account.salt, SALT_BYTES)
                || !canonical(&account.verifier, VERIFIER_BYTES)
        })
    {
        return Err(refuse(
            "invalid or unauthorized demo account verifier set",
        ));
    }
    Ok(())
}

#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_signing",
    not(target_arch = "wasm32")
))]
pub struct DemoWorkbookExtract {
    pub workbook: Vec<u8>,
    pub manifest: Option<DemoLoginVerifiers>,
}

/// For a native authorized build service only. `approved` is an installation
/// policy, never part of an upload or draft. Rebuild the redacted workbook
/// from typed cells rather than copying its ZIP/shared-strings data.
#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_signing",
    not(target_arch = "wasm32")
))]
pub fn extract_demo_workbook(
    bytes: &[u8],
    tenant_id: &str,
    approved: &[DemoLoginPrincipal],
) -> Result<DemoWorkbookExtract, Report> {
    use super::sheet::{Row, Workbook};
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    use ring::{
        pbkdf2,
        rand::{SecureRandom, SystemRandom},
    };
    use std::collections::BTreeMap;

    let workbook = super::xlsx::read_xlsx(bytes)
        .map_err(|_| refuse("could not read the demo workbook"))?;
    let is_demo_sheet = |sheet: &Sheet| {
        matches!(sheet.key.as_str(), SHEET_ADMIN_USERS | SHEET_TRUSTEES)
    };
    let password = |row: &Row| -> Result<Option<String>, Report> {
        let mut value = None;
        for (header, cell) in &row.cells {
            if !credential_column(header) {
                continue;
            }
            if matches!(cell, Value::Null)
                || matches!(cell, Value::String(v) if v.is_empty())
            {
                continue;
            }
            if !header.eq_ignore_ascii_case("password") || value.is_some() {
                return Err(refuse(
                    "ambiguous or unsupported demo password column",
                ));
            }
            let candidate = cell
                .as_str()
                .ok_or_else(|| refuse("demo password must be text"))?;
            if !(12..=1024).contains(&candidate.len()) {
                return Err(refuse(
                    "demo password must be between 12 and 1024 UTF-8 bytes",
                ));
            }
            value = Some(candidate.to_owned());
        }
        Ok(value)
    };
    let mut scheduled = BTreeMap::<String, String>::new();
    let mut admins = BTreeMap::new();
    let mut relevant_admin = |row: &Row,
                              secret: String|
     -> Result<(), Report> {
        let username = row.text("username").ok_or_else(|| {
            refuse("demo password requires an approved login")
        })?;
        if ["username", "email", "group_name", "trustee"]
            .into_iter()
            .any(|field| {
                row.get(field)
                    .is_some_and(|value| !value.is_null() && !value.is_string())
            })
        {
            return Err(refuse("demo account identity fields must be text"));
        }
        let candidate = DemoLoginPrincipal {
            username: username.to_owned(),
            email: row
                .text("email")
                .filter(|text| !text.is_empty())
                .map(str::to_owned),
            group_name: row.text("group_name").unwrap_or_default().to_owned(),
            trustee: row
                .text("trustee")
                .filter(|text| !text.is_empty())
                .map(str::to_owned),
        };
        if !checked_roster(approved) || !approved.contains(&candidate) {
            return Err(refuse(
                "demo password does not match an approved login",
            ));
        }
        match scheduled.entry(username.to_owned()) {
            std::collections::btree_map::Entry::Occupied(entry)
                if entry.get() != &secret =>
            {
                return Err(refuse(
                    "demo password differs between the administrator and trustee sheets",
                ));
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(secret);
            }
            _ => {}
        }
        Ok(())
    };
    for row in workbook.rows(SHEET_ADMIN_USERS) {
        if let Some(username) = row.text("username") {
            if admins.insert(username, row).is_some() {
                return Err(refuse(
                    "duplicate administrator identity in demo workbook",
                ));
            }
        }
        if let Some(secret) = password(row)? {
            relevant_admin(row, secret)?;
        }
    }
    let mut seen_trustees = std::collections::BTreeSet::new();
    for row in workbook.rows(SHEET_TRUSTEES) {
        let Some(secret) = password(row)? else {
            continue;
        };
        let name = row.text("name").ok_or_else(|| {
            refuse("demo trustee password requires a linked login")
        })?;
        let account = admins
            .get(name)
            .filter(|account| account.text("trustee") == Some(name))
            .ok_or_else(|| {
                refuse("demo trustee password requires a linked login")
            })?;
        if !seen_trustees.insert(name)
            || row
                .text("email")
                .filter(|text| !text.is_empty())
                .is_some_and(|email| account.text("email") != Some(email))
        {
            return Err(refuse("ambiguous demo trustee login"));
        }
        relevant_admin(account, secret)?;
    }

    let mut sheets = workbook.sheets().to_vec();
    for sheet in sheets.iter_mut().filter(|sheet| is_demo_sheet(sheet)) {
        sheet.headers.retain(|header| !credential_column(header));
        for row in &mut sheet.rows {
            row.cells.retain(|(header, _)| !credential_column(header));
        }
    }
    reject_ordinary_credentials(&sheets)?;
    let scrubbed = Workbook::new(sheets).map_err(Report::from_problem)?;
    let workbook = super::xlsx_write::write_xlsx(&scrubbed)
        .map_err(Report::from_problem)?;
    if scheduled.is_empty() {
        return Ok(DemoWorkbookExtract {
            workbook,
            manifest: None,
        });
    }
    let iterations = std::num::NonZeroU32::new(DEMO_LOGIN_ITERATIONS)
        .expect("fixed work factor");
    let random = SystemRandom::new();
    let accounts = scheduled
        .into_iter()
        .map(|(username, secret)| {
            let principal = approved
                .iter()
                .find(|entry| entry.username == username)
                .ok_or_else(|| {
                    refuse("demo password does not match an approved login")
                })?;
            let mut salt = [0u8; SALT_BYTES];
            random
                .fill(&mut salt)
                .map_err(|_| refuse("secure random salt unavailable"))?;
            let mut verifier = [0u8; VERIFIER_BYTES];
            pbkdf2::derive(
                pbkdf2::PBKDF2_HMAC_SHA256,
                iterations,
                &salt,
                secret.as_bytes(),
                &mut verifier,
            );
            Ok(DemoLoginVerifier {
                username,
                email: principal.email.clone(),
                group_name: principal.group_name.clone(),
                trustee: principal.trustee.clone(),
                iterations: DEMO_LOGIN_ITERATIONS,
                salt: B64.encode(salt),
                verifier: B64.encode(verifier),
            })
        })
        .collect::<Result<Vec<_>, Report>>()?;
    let manifest = DemoLoginVerifiers {
        format: DEMO_LOGIN_VERIFIERS_FORMAT.to_owned(),
        tenant_id: tenant_id.to_owned(),
        accounts,
    };
    verify_demo_login_verifiers(&manifest, tenant_id, approved)?;
    Ok(DemoWorkbookExtract {
        workbook,
        manifest: Some(manifest),
    })
}

/// Check the current authoritative draft still names exactly the approved
/// administrator/group/trustee for every retained verifier. Call before
/// associating a sidecar with an immutable signed revision.
pub fn validate_demo_draft(
    plan: &super::architect::Blueprint,
    tenant_id: &str,
    manifest: &DemoLoginVerifiers,
    approved: &[DemoLoginPrincipal],
) -> Result<(), Report> {
    reject_ordinary_credentials(&plan.platform)?;
    verify_demo_login_verifiers(manifest, tenant_id, approved)?;
    let mut sheets = plan
        .platform
        .iter()
        .filter(|sheet| sheet.key == SHEET_ADMIN_USERS);
    let sheet = sheets
        .next()
        .filter(|_| sheets.next().is_none())
        .ok_or_else(|| {
            refuse("expected one administrator roster in the managed draft")
        })?;
    for account in &manifest.accounts {
        let mut matching = sheet.rows.iter().filter(|row| {
            row.text("username") == Some(account.username.as_str())
        });
        let matches = matching.next().map(|row| {
            row.text("group_name") == Some(account.group_name.as_str())
                && row.text("email").filter(|text| !text.is_empty())
                    == account.email.as_deref()
                && row.text("trustee").filter(|text| !text.is_empty())
                    == account.trustee.as_deref()
        }) == Some(true)
            && matching.next().is_none();
        if !matches {
            return Err(refuse(
                "an approved demo login changed since workbook intake",
            ));
        }
        if let Some(trustee) = account.trustee.as_deref() {
            let mut matches =
                plan.trustees.iter().filter(|record| record.name == trustee);
            let found = matches.next().is_some_and(|record| {
                record.email.is_empty()
                    || Some(record.email.as_str()) == account.email.as_deref()
            });
            if !found || matches.next().is_some() {
                return Err(refuse(
                    "an approved demo trustee changed since workbook intake",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(all(
    test,
    feature = "election_config_xlsx",
    feature = "election_config_signing",
    not(target_arch = "wasm32")
))]
mod tests {
    use super::super::paths::Cell;
    use super::super::sheet::Workbook;
    use super::*;
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    use ring::pbkdf2;
    use std::num::NonZeroU32;

    const TENANT: &str = "12345678-1234-4567-8123-123456789abc";
    const SYNTHETIC: &str = "synthetic-demo-credential-441";

    fn fixture(
        pass: &str,
        trustee_pass: &str,
        include_column: bool,
    ) -> (Vec<u8>, Vec<DemoLoginPrincipal>) {
        let grid = |rows: &[&[&str]]| {
            rows.iter()
                .map(|row| {
                    row.iter().map(|v| Cell::text(*v)).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let admins = if include_column {
            grid(&[
                &["username", "group_name", "trustee", "Password"],
                &["trustee1", "trustee", "trustee1", pass],
            ])
        } else {
            grid(&[
                &["username", "group_name", "trustee"],
                &["trustee1", "trustee", "trustee1"],
            ])
        };
        let trustees = if include_column {
            grid(&[&["name", "pAsSwOrD"], &["trustee1", trustee_pass]])
        } else {
            grid(&[&["name"], &["trustee1"]])
        };
        let workbook = Workbook::new(vec![
            Sheet::from_grid("AdminUsers", &admins).unwrap(),
            Sheet::from_grid("Trustees", &trustees).unwrap(),
        ])
        .unwrap();
        (
            super::super::xlsx_write::write_xlsx(&workbook).unwrap(),
            vec![DemoLoginPrincipal {
                username: "trustee1".into(),
                email: None,
                group_name: "trustee".into(),
                trustee: Some("trustee1".into()),
            }],
        )
    }

    #[test]
    fn built_in_roster_is_exact_and_valid() {
        let roster = default_demo_login_roster();
        assert_eq!(roster.len(), 18);
        assert_eq!(roster.iter().filter(|p| p.trustee.is_some()).count(), 6);
        assert!(roster.iter().all(|p| p.username != "admin"));
        verify_demo_login_roster(&roster).unwrap();
    }

    #[test]
    fn installed_demo_rosters_reject_reserved_users_groups_domains_and_aliases()
    {
        let (_, fixture) = fixture(SYNTHETIC, SYNTHETIC, true);
        assert!(verify_demo_login_roster(&fixture).is_ok());
        for (user, group, trustee, email) in [
            ("admin", "sbei", None, None),
            ("service-account-fixture", "sbei", None, None),
            ("allowed-name", "security-officer", None, None),
            ("allowed-name", "sbei", None, Some("tester@example.com")),
            ("allowed-name", "trustee", Some("different-alias"), None),
        ] {
            assert!(verify_demo_login_roster(&[DemoLoginPrincipal {
                username: user.into(),
                email: email.map(str::to_owned),
                group_name: group.into(),
                trustee: trustee.map(str::to_owned),
            }])
            .is_err());
        }
        assert!(verify_demo_login_roster(&[]).is_err());
        assert!(verify_demo_login_roster(&[
            fixture[0].clone(),
            fixture[0].clone()
        ])
        .is_err());
    }

    #[test]
    fn redacts_fresh_xlsx_and_carries_only_a_compatible_verifier() {
        let (raw, approved) = fixture(SYNTHETIC, SYNTHETIC, true);
        let extracted = extract_demo_workbook(&raw, TENANT, &approved).unwrap();
        let manifest = extracted.manifest.unwrap();
        verify_demo_login_verifiers(&manifest, TENANT, &approved).unwrap();
        let serialized = serde_json::to_string(&manifest).unwrap();
        assert!(!serialized.contains(SYNTHETIC));
        assert_eq!(manifest.accounts.len(), 1);
        let account = &manifest.accounts[0];
        pbkdf2::verify(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(account.iterations).unwrap(),
            &B64.decode(&account.salt).unwrap(),
            SYNTHETIC.as_bytes(),
            &B64.decode(&account.verifier).unwrap(),
        )
        .unwrap();
        let use_zip =
            super::super::xlsx::read_xlsx(&extracted.workbook).unwrap();
        reject_ordinary_credentials(use_zip.sheets()).unwrap();
        for sheet in use_zip.sheets() {
            assert!(!sheet
                .headers
                .iter()
                .any(|header| credential_column(header)));
        }
        let plan = super::super::architect::Blueprint {
            platform: vec![use_zip.sheet(SHEET_ADMIN_USERS).unwrap().clone()],
            trustees: vec![super::super::architect::Trustee {
                name: "trustee1".into(),
                email: String::new(),
            }],
            ..Default::default()
        };
        validate_demo_draft(&plan, TENANT, &manifest, &approved).unwrap();
        let mut changed = plan;
        changed.trustees[0].name = "another-trustee".into();
        assert!(validate_demo_draft(&changed, TENANT, &manifest, &approved)
            .is_err());
    }

    #[test]
    fn wrong_scope_ambiguous_aliases_and_empty_inputs_do_not_leak() {
        let (raw, approved) =
            fixture(SYNTHETIC, "different-synthetic-demo-441", true);
        let refused = extract_demo_workbook(&raw, TENANT, &approved)
            .err()
            .unwrap();
        assert!(!format!("{refused:?}").contains(SYNTHETIC));
        let (raw, _) = fixture(SYNTHETIC, SYNTHETIC, true);
        assert!(extract_demo_workbook(&raw, TENANT, &[]).is_err());
        let (raw, _) = fixture("", "", false);
        let unchanged = extract_demo_workbook(&raw, TENANT, &[]).unwrap();
        assert_eq!(
            super::super::xlsx::read_xlsx(&unchanged.workbook).unwrap(),
            super::super::xlsx::read_xlsx(&raw).unwrap(),
        );
        assert!(unchanged.manifest.is_none());
    }
}
