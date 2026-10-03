// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The words printed on a PDF signature page and in each signature's
//! appearance, per language, from `signature_page_texts.toml`. A page is
//! printed in the election event's default language, else the tenant's,
//! else English ([`FALLBACK_LANGUAGE`]).

use anyhow::{anyhow, bail, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::signing::SigningAction;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;
use uuid::Uuid;

/// The language a page falls back to; the table always carries it.
pub const FALLBACK_LANGUAGE: &str = "en";

const SHIPPED_TABLE: &str = include_str!("signature_page_texts.toml");

/// The words of the page in one language. Each `{placeholder}` is filled in
/// by name.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageWording {
    /// The page's title: `{organization}`.
    pub title: String,
    /// The certification sentence above the boxes, for each action that
    /// signs a PDF.
    pub certify: HashMap<SigningAction, String>,
    /// The label under a signature box: `{n}`.
    pub signature: String,
    /// A signature box's tooltip: `{n}`, `{total}`.
    pub signature_of: String,
    /// The first line of a signature: `{name}`, its certificate's holder.
    pub signed_by: String,
    /// When it was signed: `{time}`.
    pub date: String,
    /// Who issued its certificate: `{issuer}`.
    pub issuer: String,
    /// The request's signing code: `{code}`.
    pub code: String,
    /// The SHA-256 of the document before any signature: `{sha256}`.
    pub document: String,
}

/// The words of the page, by language code.
pub type PageWordings = BTreeMap<String, PageWording>;

static SHIPPED: LazyLock<Result<PageWordings, String>> =
    LazyLock::new(|| parse_wordings(SHIPPED_TABLE).map_err(|error| format!("{error:#}")));

/// Reads a table of page texts, which must carry [`FALLBACK_LANGUAGE`].
pub fn parse_wordings(source: &str) -> Result<PageWordings> {
    let wordings: PageWordings =
        toml::from_str(source).context("Error reading the signature page texts")?;
    if !wordings.contains_key(FALLBACK_LANGUAGE) {
        bail!("The signature page texts have no {FALLBACK_LANGUAGE:?} texts");
    }
    Ok(wordings)
}

/// The page texts the signing code ships.
pub fn shipped_wordings() -> Result<&'static PageWordings> {
    SHIPPED
        .as_ref()
        .map_err(|error| anyhow!("The signature page texts are unusable: {error}"))
}

/// The words of the first of `languages` that `wordings` carries, else of
/// [`FALLBACK_LANGUAGE`].
pub fn pick_wording<'a>(
    wordings: &'a PageWordings,
    languages: &[String],
) -> Result<&'a PageWording> {
    languages
        .iter()
        .map(|language| language.trim().to_lowercase())
        .find_map(|language| wordings.get(&language))
        .or_else(|| wordings.get(FALLBACK_LANGUAGE))
        .ok_or_else(|| anyhow!("The signature page texts have no {FALLBACK_LANGUAGE:?} texts"))
}

/// The languages a page of the event is printed in, in order of
/// preference: the event's default language, then the tenant's
/// (`language_conf.default_language_code` of the event's `presentation` and
/// of the tenant's `settings`).
pub async fn page_languages(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<String>> {
    let row = hasura_transaction
        .query_opt(
            "SELECT
                 e.presentation->'language_conf'->>'default_language_code',
                 t.settings->'language_conf'->>'default_language_code'
             FROM sequent_backend.tenant AS t
             LEFT JOIN sequent_backend.election_event AS e
                 ON e.tenant_id = t.id AND e.id = $2
             WHERE t.id = $1",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the event's language")?
        .ok_or_else(|| anyhow!("There is no tenant {tenant_id}"))?;
    let languages = [row.try_get::<_, Option<String>>(0)?, row.try_get(1)?];
    Ok(languages
        .into_iter()
        .flatten()
        .map(|language| language.trim().to_owned())
        .filter(|language| !language.is_empty())
        .collect())
}

/// The words of the event's signature pages ([`page_languages`]).
pub async fn event_wording(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<&'static PageWording> {
    let languages = page_languages(hasura_transaction, tenant_id, election_event_id).await?;
    pick_wording(shipped_wordings()?, &languages)
}

/// Fills each `{key}` of `template` with its value, in one pass: a value is
/// never read as a placeholder. Unknown placeholders stay as they are.
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut text = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        text.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let value = after.find('}').and_then(|end| {
            let key = &after[..end];
            values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value, end))
        });
        match value {
            Some((value, end)) => {
                text.push_str(value);
                rest = &after[end + 1..];
            }
            None => {
                text.push('{');
                rest = after;
            }
        }
    }
    text.push_str(rest);
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::signing::DocumentKind;
    use strum::IntoEnumIterator;

    const PLACEHOLDERS: [(&str, &[&str]); 8] = [
        ("title", &["organization"]),
        ("signature", &["n"]),
        ("signature_of", &["n", "total"]),
        ("signed_by", &["name"]),
        ("date", &["time"]),
        ("issuer", &["issuer"]),
        ("code", &["code"]),
        ("document", &["sha256"]),
    ];

    fn languages(codes: &[&str]) -> Vec<String> {
        codes.iter().map(|code| code.to_string()).collect()
    }

    /// English and a second language, as a deployment's table could be.
    fn two_languages() -> PageWordings {
        let mut source = SHIPPED_TABLE.to_owned();
        source.push_str(
            r#"
[xx]
title = "XX {organization}"
signature = "XX {n}"
signature_of = "XX {n} / {total}"
signed_by = "XX {name}"
date = "XX {time}"
issuer = "XX {issuer}"
code = "XX {code}"
document = "XX {sha256}"

[xx.certify]
generate-election-returns = "XX returns"
generate-reports = "XX report"
"#,
        );
        parse_wordings(&source).unwrap()
    }

    #[test]
    fn signing_page_texts_ship_english_with_every_text_of_every_pdf_action() {
        let wordings = shipped_wordings().unwrap();
        assert!(wordings.contains_key(FALLBACK_LANGUAGE));
        let pdf_actions: Vec<SigningAction> = SigningAction::iter()
            .filter(|action| action.document() == DocumentKind::Pdf)
            .collect();
        assert!(!pdf_actions.is_empty());
        for (language, wording) in wordings {
            let mut actions: Vec<&SigningAction> = wording.certify.keys().collect();
            actions.sort_by_key(|action| action.to_string());
            let mut expected: Vec<&SigningAction> = pdf_actions.iter().collect();
            expected.sort_by_key(|action| action.to_string());
            assert_eq!(actions, expected, "{language}");
            let texts = [
                &wording.title,
                &wording.signature,
                &wording.signature_of,
                &wording.signed_by,
                &wording.date,
                &wording.issuer,
                &wording.code,
                &wording.document,
            ];
            for ((key, placeholders), text) in PLACEHOLDERS.iter().zip(texts) {
                for placeholder in *placeholders {
                    assert!(
                        text.contains(&format!("{{{placeholder}}}")),
                        "{language}.{key} has no {{{placeholder}}}: {text}"
                    );
                }
            }
        }
    }

    /// The admin portal's languages (`getAllLangs`).
    #[test]
    fn signing_page_texts_ship_every_portal_language() {
        let shipped: Vec<&str> = shipped_wordings()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(shipped, ["cat", "en", "es", "eu", "fr", "gl", "nl", "tl"]);
    }

    #[test]
    fn signing_page_language_is_the_first_the_table_carries() {
        let wordings = two_languages();
        let xx = &wordings["xx"];
        assert_eq!(pick_wording(&wordings, &languages(&["xx"])).unwrap(), xx);
        assert_eq!(
            pick_wording(&wordings, &languages(&["de", " XX "])).unwrap(),
            xx
        );
        assert_eq!(
            pick_wording(&wordings, &languages(&["en", "xx"])).unwrap(),
            &wordings["en"]
        );
    }

    #[test]
    fn signing_page_language_unknown_falls_back_to_english() {
        let wordings = two_languages();
        for codes in [&["de"][..], &["de", "pt"], &[]] {
            assert_eq!(
                pick_wording(&wordings, &languages(codes)).unwrap(),
                &wordings[FALLBACK_LANGUAGE],
                "{codes:?}"
            );
        }
    }

    #[test]
    fn signing_page_texts_without_english_are_refused() {
        let error = parse_wordings(&SHIPPED_TABLE.replace("[en", "[de")).unwrap_err();
        assert!(error.to_string().contains("no \"en\" texts"), "{error}");
        assert!(parse_wordings("[en]\ntitle = 1").is_err());
    }

    #[test]
    fn signing_page_placeholders_are_filled_by_name() {
        assert_eq!(
            fill("{n} of {total}: {n}", &[("n", "2"), ("total", "3")]),
            "2 of 3: 2"
        );
        assert_eq!(fill("{name}", &[]), "{name}");
        // A value that looks like a placeholder is printed as it is.
        assert_eq!(
            fill("{name} {code}", &[("name", "{code}"), ("code", "7F3A")]),
            "{code} 7F3A"
        );
        assert_eq!(fill("{ {n}} {", &[("n", "1")]), "{ 1} {");
    }
}
