// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What an election event already states about itself, expressed as realm
//! settings.
//!
//! The languages an event enables, its name, and its login CSS all belong on the
//! login page, and the platform copies none of them there: it syncs the default
//! locale only under a force-default detection policy, never syncs
//! `supportedLocales`, and has no path at all from an event name to a realm
//! display name. Stating them twice in a source document would be a way to get
//! them out of step, so they are derived from what the event already says.
//!
//! The language codes go through [`crate::util::locale::iso_639_2t_to_bcp47`] —
//! the platform's own table, rather than a second copy of it. The Python this was
//! ported from transcribed all 177 entries by hand, which is exactly the kind of
//! duplication this work exists to remove.

use crate::util::locale::iso_639_2t_to_bcp47;
use serde_json::{json, Map, Value};

/// The realm localization key the login theme renders.
///
/// See `sequent-theme/.../sequent.admin-portal/login/template.ftl`.
pub const LOGIN_CUSTOM_CSS_KEY: &str = "loginCustomCss";

/// Escape CSS for Keycloak's `${msg(...)}`, which is `java.text.MessageFormat`.
///
/// MessageFormat treats `{` and `}` as format-element delimiters and `'` as a
/// quoting character. CSS is made of braces, and its `url('…')` is made of
/// quotes, so pasting CSS in raw produces either a parse error or silently
/// mangled output — a login page with no styling and nothing in the logs.
///
/// The escaping MessageFormat defines is: `''` for a literal single quote, and a
/// brace is literal when quoted, so `{` becomes `'{'`.
///
/// Quotes are escaped **first**, so the quotes added around braces are not
/// themselves doubled:
///
/// ```text
/// a { b: url('x'); }   ->   a '{' b: url(''x''); '}'
/// ```
///
/// which MessageFormat renders back as the original. The platform's own realm
/// writes braces exactly this way.
pub fn escape_message_format(text: &str) -> String {
    text.replace('\'', "''")
        .replace('{', "'{'")
        .replace('}', "'}'")
}

/// Strip one enclosing pair of single quotes, if present.
///
/// Quoting an entire string is the other way to make MessageFormat treat it
/// literally, so realm values are often passed around already wrapped that way.
/// Accepting both means a value copied out of a working realm and a value typed
/// as plain CSS both do the right thing.
pub fn unwrap_quoted(text: &str) -> &str {
    let stripped = text.trim();
    if stripped.len() >= 2
        && stripped.starts_with('\'')
        && stripped.ends_with('\'')
    {
        &stripped[1..stripped.len() - 1]
    } else {
        text
    }
}

/// Realm i18n settings from the event's `presentation.language_conf`.
///
/// `supportedLocales` is what puts a language in Keycloak's login-page picker,
/// and nothing else sets it — so an event offering a language the realm does not
/// list is a ballot whose login page the voter cannot read.
pub fn language_patch(language_conf: Option<&Value>) -> Map<String, Value> {
    let mut patch = Map::new();
    let Some(Value::Object(language_conf)) = language_conf else {
        return patch;
    };

    if let Some(codes) = enabled_locales(language_conf) {
        // Keycloak needs this on for supportedLocales to have any effect.
        patch.insert("internationalizationEnabled".to_string(), json!(true));
        patch.insert("supportedLocales".to_string(), json!(codes));
    }

    if let Some(default) = language_conf
        .get("default_language_code")
        .and_then(Value::as_str)
        .filter(|code| !code.is_empty())
    {
        patch.insert(
            "defaultLocale".to_string(),
            json!(iso_639_2t_to_bcp47(default)),
        );
    }
    patch
}

/// The BCP 47 locales a realm localization string should be written for.
pub fn locales_of(
    language_conf: Option<&Value>,
    fallback: &str,
) -> Vec<String> {
    if let Some(Value::Object(language_conf)) = language_conf {
        if let Some(codes) = enabled_locales(language_conf) {
            return codes;
        }
        if let Some(default) = language_conf
            .get("default_language_code")
            .and_then(Value::as_str)
            .filter(|code| !code.is_empty())
        {
            return vec![iso_639_2t_to_bcp47(default).to_string()];
        }
    }
    vec![fallback.to_string()]
}

/// The enabled codes as sorted, deduplicated BCP 47.
fn enabled_locales(language_conf: &Map<String, Value>) -> Option<Vec<String>> {
    let Some(Value::Array(enabled)) =
        language_conf.get("enabled_language_codes")
    else {
        return None;
    };
    if enabled.is_empty() {
        return None;
    }

    let mut codes: Vec<String> = enabled
        .iter()
        .filter_map(Value::as_str)
        .filter(|code| !code.is_empty())
        .map(|code| iso_639_2t_to_bcp47(code).to_string())
        .collect();
    if codes.is_empty() {
        return None;
    }
    codes.sort();
    codes.dedup();
    Some(codes)
}

/// The realm `displayName`, taken from the event's own name.
///
/// Keycloak shows it above the login form, so leaving it at the platform default
/// means every client's voters see "Election Event".
///
/// `displayNameHtml` is deliberately left alone: Keycloak renders it as raw markup
/// and prefers it over `displayName` when set, so writing an event name into it
/// would turn any `&` in a client's name into a rendering bug.
pub fn title_patch(
    i18n: Option<&Value>,
    default_language: Option<&str>,
) -> Map<String, Value> {
    let mut patch = Map::new();
    let Some(Value::Object(i18n)) = i18n else {
        return patch;
    };
    if i18n.is_empty() {
        return patch;
    }

    // The event's default language first, then the raw code it was written as,
    // then English.
    let converted = default_language.map(iso_639_2t_to_bcp47);
    let preferred: Vec<&str> = converted
        .into_iter()
        .chain(default_language)
        .chain(std::iter::once("en"))
        .collect();

    for key in preferred {
        if let Some(name) = named(i18n.get(key)) {
            patch.insert("displayName".to_string(), json!(name));
            return patch;
        }
    }

    // No default language matched, so take whichever entry has a name. Sorted so
    // the choice does not depend on map iteration order.
    let mut keys: Vec<&String> = i18n.keys().collect();
    keys.sort();
    for key in keys {
        if let Some(name) = named(i18n.get(key)) {
            patch.insert("displayName".to_string(), json!(name));
            return patch;
        }
    }
    patch
}

fn named(entry: Option<&Value>) -> Option<&str> {
    entry?
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
}

/// `localizationTexts.<locale>.loginCustomCss`, escaped and per language.
///
/// Written for every enabled locale: Keycloak looks the message up in the voter's
/// language, so CSS present only under `en` disappears the moment a voter switches
/// to Spanish.
pub fn login_css_patch(css: &str, locales: &[String]) -> Map<String, Value> {
    let escaped = escape_message_format(unwrap_quoted(css));
    let mut texts = Map::new();
    for locale in locales {
        texts.insert(
            locale.clone(),
            json!({LOGIN_CUSTOM_CSS_KEY: escaped.clone()}),
        );
    }

    let mut patch = Map::new();
    patch.insert("localizationTexts".to_string(), Value::Object(texts));
    patch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_braces_and_quotes_are_escaped_the_way_message_format_reads_them() {
        // Raw CSS in a ${msg(...)} is either a parse error or silently mangled
        // output: a login page with no styling and nothing in the logs.
        assert_eq!(
            escape_message_format("a { b: url('x'); }"),
            "a '{' b: url(''x''); '}'"
        );
    }

    #[test]
    fn quotes_are_escaped_before_braces_are_wrapped_in_them() {
        // The other order doubles the quotes this function just added, and the
        // result renders as a literal '{' instead of a brace.
        assert_eq!(escape_message_format("{"), "'{'");
        assert_eq!(escape_message_format("'"), "''");
        assert_eq!(escape_message_format("'{'"), "'''{'''");
    }

    #[test]
    fn text_with_nothing_message_format_cares_about_is_left_alone() {
        // Most of a stylesheet is not braces, and mangling it would be as bad as
        // not escaping the parts that are.
        assert_eq!(escape_message_format("color: red;"), "color: red;");
        assert_eq!(
            escape_message_format("background: #fff url(logo.png)"),
            "background: #fff url(logo.png)"
        );
    }

    #[test]
    fn a_value_already_quoted_for_message_format_is_unwrapped_first() {
        // A value copied out of a working realm arrives wrapped; one typed as
        // plain CSS does not. Both have to work.
        assert_eq!(unwrap_quoted("'a { b: c }'"), "a { b: c }");
        assert_eq!(unwrap_quoted("a { b: c }"), "a { b: c }");
        assert_eq!(unwrap_quoted("'"), "'");
        assert_eq!(unwrap_quoted(""), "");
    }

    #[test]
    fn the_enabled_languages_become_the_login_pages_picker() {
        // Nothing else in the platform sets supportedLocales.
        let patch = language_patch(Some(&json!({
            "enabled_language_codes": ["eng", "spa", "cat"],
            "default_language_code": "spa",
        })));
        assert_eq!(patch["internationalizationEnabled"], json!(true));
        assert_eq!(patch["supportedLocales"], json!(["ca", "en", "es"]));
        assert_eq!(patch["defaultLocale"], json!("es"));
    }

    #[test]
    fn codes_are_converted_by_the_platforms_own_table() {
        // Not a second transcription of it: the Python copied all 177 entries by
        // hand, which is exactly the duplication this work removes.
        let patch = language_patch(Some(&json!({
            "enabled_language_codes": ["cat", "glg", "spa"],
        })));
        assert_eq!(patch["supportedLocales"], json!(["ca", "es", "gl"]));
    }

    #[test]
    fn a_code_the_table_does_not_know_passes_through_unchanged() {
        // Which is the safe answer: an unconverted code is a locale Keycloak may
        // not recognise, whereas a guessed one is a locale it recognises wrongly.
        //
        // Basque ("eus") and Dutch ("nld") are among the codes the platform's
        // table lacks. Both the Rust and the Python it replaces behave this way,
        // so this is a gap in `util::locale`, not a regression here.
        let patch = language_patch(Some(&json!({
            "enabled_language_codes": ["en", "eus", "nld"],
        })));
        assert_eq!(patch["supportedLocales"], json!(["en", "eus", "nld"]));
    }

    #[test]
    fn duplicate_codes_collapse() {
        // "en" and "eng" are the same locale, and a realm listing it twice is a
        // picker showing it twice.
        let patch = language_patch(Some(&json!({
            "enabled_language_codes": ["en", "eng"],
        })));
        assert_eq!(patch["supportedLocales"], json!(["en"]));
    }

    #[test]
    fn no_language_configuration_means_no_language_patch() {
        // Leaving the realm alone is the right answer, not writing an empty list
        // that would empty its picker.
        assert!(language_patch(None).is_empty());
        assert!(language_patch(Some(&json!({}))).is_empty());
        assert!(language_patch(Some(&json!("not an object"))).is_empty());
        assert!(language_patch(Some(&json!({"enabled_language_codes": []})))
            .is_empty());
    }

    #[test]
    fn a_default_language_alone_still_sets_the_default_locale() {
        let patch =
            language_patch(Some(&json!({"default_language_code": "cat"})));
        assert_eq!(patch["defaultLocale"], json!("ca"));
        assert!(patch.get("supportedLocales").is_none());
    }

    #[test]
    fn the_locales_css_is_written_for_follow_the_enabled_languages() {
        assert_eq!(
            locales_of(
                Some(&json!({"enabled_language_codes": ["eng", "spa"]})),
                "en"
            ),
            ["en", "es"]
        );
        assert_eq!(
            locales_of(Some(&json!({"default_language_code": "spa"})), "en"),
            ["es"]
        );
        assert_eq!(locales_of(None, "en"), ["en"]);
    }

    #[test]
    fn the_event_name_becomes_the_realms_display_name() {
        // Otherwise every client's voters see "Election Event" above the form.
        let patch = title_patch(
            Some(&json!({
                "en": {"name": "Union Election 2027"},
                "es": {"name": "Elección Sindical 2027"},
            })),
            Some("spa"),
        );
        assert_eq!(patch["displayName"], json!("Elección Sindical 2027"));
    }

    #[test]
    fn the_title_falls_back_to_english_then_to_whatever_has_a_name() {
        assert_eq!(
            title_patch(Some(&json!({"en": {"name": "English"}})), Some("spa"))
                ["displayName"],
            json!("English")
        );
        assert_eq!(
            title_patch(
                Some(&json!({"fr": {"name": "Français"}})),
                Some("spa")
            )["displayName"],
            json!("Français")
        );
    }

    #[test]
    fn a_default_language_written_as_bcp47_matches_too() {
        // An author who writes "es" rather than "spa" means the same thing.
        assert_eq!(
            title_patch(
                Some(
                    &json!({"es": {"name": "Español"}, "en": {"name": "English"}})
                ),
                Some("es")
            )["displayName"],
            json!("Español")
        );
    }

    #[test]
    fn a_nameless_i18n_block_leaves_the_display_name_alone() {
        assert!(title_patch(Some(&json!({"en": {}})), Some("eng")).is_empty());
        assert!(
            title_patch(Some(&json!({"en": {"name": ""}})), None).is_empty()
        );
        assert!(title_patch(None, None).is_empty());
        assert!(title_patch(Some(&json!({})), None).is_empty());
    }

    #[test]
    fn display_name_html_is_never_written() {
        // Keycloak renders it as raw markup and prefers it when set, so an "&" in
        // a client's name would become a rendering bug.
        let patch = title_patch(
            Some(&json!({"en": {"name": "Smith & Sons Local"}})),
            None,
        );
        assert_eq!(patch["displayName"], json!("Smith & Sons Local"));
        assert!(patch.get("displayNameHtml").is_none());
    }

    #[test]
    fn login_css_is_written_for_every_enabled_language() {
        // Keycloak looks the message up in the voter's language, so CSS under `en`
        // alone vanishes when a voter switches to Spanish.
        let patch = login_css_patch(
            ".logo { display: none; }",
            &["en".to_string(), "es".to_string()],
        );
        let texts = &patch["localizationTexts"];
        assert_eq!(
            texts["en"][LOGIN_CUSTOM_CSS_KEY],
            json!(".logo '{' display: none; '}'")
        );
        assert_eq!(
            texts["es"][LOGIN_CUSTOM_CSS_KEY],
            texts["en"][LOGIN_CUSTOM_CSS_KEY]
        );
    }

    #[test]
    fn login_css_copied_out_of_a_realm_is_not_escaped_twice() {
        // It arrives already wrapped in quotes; escaping that as content would
        // double every quote in it.
        let patch =
            login_css_patch("'.logo { display: none; }'", &["en".to_string()]);
        assert_eq!(
            patch["localizationTexts"]["en"][LOGIN_CUSTOM_CSS_KEY],
            json!(".logo '{' display: none; '}'")
        );
    }
}
