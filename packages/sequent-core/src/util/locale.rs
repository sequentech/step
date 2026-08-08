// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Maps an ISO 639-2/T three-letter code to its ISO 639-1 two-letter equivalent.
///
/// Keycloak (and BCP 47 in general) uses ISO 639-1 codes where they exist. The
/// internal system uses a mix of ISO 639-1 codes and ISO 639-2/T codes. Codes
/// that are already two letters (ISO 639-1) are returned unchanged; only the
/// three-letter ISO 639-2/T codes that have a different two-letter form need
/// mapping.
///
/// Reference: https://www.loc.gov/standards/iso639-2/php/code_list.php
pub fn iso_639_2t_to_bcp47(lang: &str) -> &str {
    match lang {
        "aar" => "aa", // Afar
        "abk" => "ab", // Abkhazian
        "afr" => "af", // Afrikaans
        "aka" => "ak", // Akan
        "amh" => "am", // Amharic
        "ara" => "ar", // Arabic
        "arg" => "an", // Aragonese
        "asm" => "as", // Assamese
        "ava" => "av", // Avaric
        "ave" => "ae", // Avestan
        "aym" => "ay", // Aymara
        "aze" => "az", // Azerbaijani
        "bak" => "ba", // Bashkir
        "bam" => "bm", // Bambara
        "bel" => "be", // Belarusian
        "ben" => "bn", // Bengali
        "bis" => "bi", // Bislama
        "bos" => "bs", // Bosnian
        "bre" => "br", // Breton
        "bul" => "bg", // Bulgarian
        "cat" => "ca", // Catalan
        "cha" => "ch", // Chamorro
        "che" => "ce", // Chechen
        "chu" => "cu", // Church Slavic
        "chv" => "cv", // Chuvash
        "cor" => "kw", // Cornish
        "cos" => "co", // Corsican
        "cre" => "cr", // Cree
        "ces" => "cs", // Czech
        "dan" => "da", // Danish
        "div" => "dv", // Divehi
        "dzo" => "dz", // Dzongkha
        "eng" => "en", // English
        "epo" => "eo", // Esperanto
        "est" => "et", // Estonian
        "ewe" => "ee", // Ewe
        "fao" => "fo", // Faroese
        "fij" => "fj", // Fijian
        "fin" => "fi", // Finnish
        "fra" => "fr", // French
        "ful" => "ff", // Fula
        "gla" => "gd", // Scottish Gaelic
        "gle" => "ga", // Irish
        "glg" => "gl", // Galician
        "glv" => "gv", // Manx
        "deu" => "de", // German
        "ell" => "el", // Greek
        "grn" => "gn", // Guaraní
        "guj" => "gu", // Gujarati
        "hat" => "ht", // Haitian Creole
        "hau" => "ha", // Hausa
        "heb" => "he", // Hebrew
        "her" => "hz", // Herero
        "hin" => "hi", // Hindi
        "hmo" => "ho", // Hiri Motu
        "hrv" => "hr", // Croatian
        "hun" => "hu", // Hungarian
        "hye" => "hy", // Armenian
        "ibo" => "ig", // Igbo
        "ido" => "io", // Ido
        "iii" => "ii", // Sichuan Yi
        "iku" => "iu", // Inuktitut
        "ile" => "ie", // Interlingue
        "ina" => "ia", // Interlingua
        "ind" => "id", // Indonesian
        "ipk" => "ik", // Inupiaq
        "isl" => "is", // Icelandic
        "ita" => "it", // Italian
        "jav" => "jv", // Javanese
        "jpn" => "ja", // Japanese
        "kal" => "kl", // Kalaallisut
        "kan" => "kn", // Kannada
        "kas" => "ks", // Kashmiri
        "kat" => "ka", // Georgian
        "kau" => "kr", // Kanuri
        "kaz" => "kk", // Kazakh
        "khm" => "km", // Khmer
        "kik" => "ki", // Kikuyu
        "kin" => "rw", // Kinyarwanda
        "kir" => "ky", // Kyrgyz
        "kom" => "kv", // Komi
        "kon" => "kg", // Kongo
        "kor" => "ko", // Korean
        "kua" => "kj", // Kuanyama
        "kur" => "ku", // Kurdish
        "lao" => "lo", // Lao
        "lat" => "la", // Latin
        "lav" => "lv", // Latvian
        "lim" => "li", // Limburgish
        "lin" => "ln", // Lingala
        "lit" => "lt", // Lithuanian
        "lub" => "lu", // Luba-Katanga
        "lug" => "lg", // Ganda
        "mkd" => "mk", // Macedonian
        "mah" => "mh", // Marshallese
        "mal" => "ml", // Malayalam
        "mri" => "mi", // Māori
        "mar" => "mr", // Marathi
        "msa" => "ms", // Malay
        "mlg" => "mg", // Malagasy
        "mlt" => "mt", // Maltese
        "mon" => "mn", // Mongolian
        "nau" => "na", // Nauru
        "nav" => "nv", // Navajo
        "nbl" => "nr", // South Ndebele
        "nde" => "nd", // North Ndebele
        "ndo" => "ng", // Ndonga
        "nep" => "ne", // Nepali
        "nno" => "nn", // Norwegian Nynorsk
        "nob" => "nb", // Norwegian Bokmål
        "nor" => "no", // Norwegian
        "nya" => "ny", // Chichewa
        "oci" => "oc", // Occitan
        "oji" => "oj", // Ojibwe
        "ori" => "or", // Oriya
        "orm" => "om", // Oromo
        "oss" => "os", // Ossetic
        "pan" => "pa", // Punjabi
        "fas" => "fa", // Persian
        "pol" => "pl", // Polish
        "por" => "pt", // Portuguese
        "pus" => "ps", // Pashto
        "que" => "qu", // Quechua
        "roh" => "rm", // Romansh
        "ron" => "ro", // Romanian
        "run" => "rn", // Kirundi
        "rus" => "ru", // Russian
        "sag" => "sg", // Sango
        "san" => "sa", // Sanskrit
        "sin" => "si", // Sinhalese
        "slk" => "sk", // Slovak
        "slv" => "sl", // Slovenian
        "sme" => "se", // Northern Sami
        "smo" => "sm", // Samoan
        "sna" => "sn", // Shona
        "snd" => "sd", // Sindhi
        "som" => "so", // Somali
        "sot" => "st", // Southern Sotho
        "spa" => "es", // Spanish
        "sqi" => "sq", // Albanian
        "srd" => "sc", // Sardinian
        "srp" => "sr", // Serbian
        "ssw" => "ss", // Swati
        "sun" => "su", // Sundanese
        "swa" => "sw", // Swahili
        "swe" => "sv", // Swedish
        "tah" => "ty", // Tahitian
        "tam" => "ta", // Tamil
        "tat" => "tt", // Tatar
        "tel" => "te", // Telugu
        "tgk" => "tg", // Tajik
        "tgl" => "tl", // Tagalog
        "tha" => "th", // Thai
        "bod" => "bo", // Tibetan
        "tir" => "ti", // Tigrinya
        "ton" => "to", // Tonga
        "tsn" => "tn", // Tswana
        "tso" => "ts", // Tsonga
        "tuk" => "tk", // Turkmen
        "tur" => "tr", // Turkish
        "twi" => "tw", // Twi
        "uig" => "ug", // Uighur
        "ukr" => "uk", // Ukrainian
        "urd" => "ur", // Urdu
        "uzb" => "uz", // Uzbek
        "ven" => "ve", // Venda
        "vie" => "vi", // Vietnamese
        "vol" => "vo", // Volapük
        "cym" => "cy", // Welsh
        "wln" => "wa", // Walloon
        "wol" => "wo", // Wolof
        "xho" => "xh", // Xhosa
        "yid" => "yi", // Yiddish
        "yor" => "yo", // Yoruba
        "zha" => "za", // Zhuang
        "zho" => "zh", // Chinese
        "zul" => "zu", // Zulu
        // Pass through codes that are already ISO 639-1 or have no ISO 639-1 equivalent
        other => other,
    }
}

/// Suffix marking the informal-register variant of an internal language code,
/// as in `es-tu` (addressing the voter as *tú* rather than *usted*).
pub const INFORMAL_SUFFIX: &str = "tu";

/// Splits a locale into its primary subtag and whether it asks for the
/// informal register.
///
/// The marker may arrive bare, as in the internal code `es-tu`, or behind the
/// BCP 47 private-use singleton, as in `es-x-tu` — which is what the frontends
/// put in `<html lang>`. Region and script subtags are ignored, so `es-ES-x-tu`
/// is still Spanish informal.
fn split_register(lang: &str) -> (String, bool) {
    let mut parts = lang.split('-');
    let primary = parts.next().unwrap_or(lang).to_ascii_lowercase();
    let informal = parts.any(|part| part.eq_ignore_ascii_case(INFORMAL_SUFFIX));
    (primary, informal)
}

/// Normalizes an external locale representation into the internal language code
/// used by the web frontends.
///
/// Inputs may be BCP 47 tags (for example `ca` or `es-ES`), already-internal
/// language codes (for example `cat` or `es-tu`), private-use register tags
/// (`es-x-tu`), or ISO 639-2/T codes (for example `eng`). The
/// returned value is the internal application language code.
pub fn locale_to_internal_language_code(lang: &str) -> String {
    let (primary, informal) = split_register(lang);
    let base = match iso_639_2t_to_bcp47(primary.as_str()) {
        "ca" => "cat",
        normalized => normalized,
    };
    if informal {
        format!("{base}-{INFORMAL_SUFFIX}")
    } else {
        base.to_string()
    }
}

/// Converts an internal language code to a BCP 47 tag fit for `<html lang>`.
///
/// Register is not a registered BCP 47 subtag, so the informal variants are
/// emitted as private-use tags: `es-tu` becomes `es-x-tu`. That is
/// well-formed, and assistive technology falls back to the base language
/// instead of rejecting an unregistered variant.
pub fn internal_language_code_to_bcp47(lang: &str) -> String {
    let (primary, informal) = split_register(lang);
    let base = iso_639_2t_to_bcp47(primary.as_str());
    if informal {
        format!("{base}-x-{INFORMAL_SUFFIX}")
    } else {
        base.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_639_2t_catalan() {
        assert_eq!(iso_639_2t_to_bcp47("cat"), "ca");
    }

    #[test]
    fn test_iso_639_2t_already_bcp47() {
        assert_eq!(iso_639_2t_to_bcp47("en"), "en");
        assert_eq!(iso_639_2t_to_bcp47("es"), "es");
        assert_eq!(iso_639_2t_to_bcp47("fr"), "fr");
        assert_eq!(iso_639_2t_to_bcp47("nl"), "nl");
        assert_eq!(iso_639_2t_to_bcp47("eu"), "eu");
        assert_eq!(iso_639_2t_to_bcp47("gl"), "gl");
        assert_eq!(iso_639_2t_to_bcp47("tl"), "tl");
    }

    #[test]
    fn test_iso_639_2t_common_languages() {
        assert_eq!(iso_639_2t_to_bcp47("fra"), "fr");
        assert_eq!(iso_639_2t_to_bcp47("deu"), "de");
        assert_eq!(iso_639_2t_to_bcp47("spa"), "es");
        assert_eq!(iso_639_2t_to_bcp47("por"), "pt");
        assert_eq!(iso_639_2t_to_bcp47("rus"), "ru");
        assert_eq!(iso_639_2t_to_bcp47("ara"), "ar");
        assert_eq!(iso_639_2t_to_bcp47("zho"), "zh");
        assert_eq!(iso_639_2t_to_bcp47("jpn"), "ja");
    }

    #[test]
    fn test_iso_639_2t_unknown_passthrough() {
        assert_eq!(iso_639_2t_to_bcp47("xyz"), "xyz");
    }

    #[test]
    fn test_locale_to_internal_language_code_catalan() {
        assert_eq!(locale_to_internal_language_code("ca"), "cat");
        assert_eq!(locale_to_internal_language_code("ca-ES"), "cat");
        assert_eq!(locale_to_internal_language_code("cat"), "cat");
    }

    #[test]
    fn test_locale_to_internal_language_code_common_languages() {
        assert_eq!(locale_to_internal_language_code("en"), "en");
        assert_eq!(locale_to_internal_language_code("en-US"), "en");
        assert_eq!(locale_to_internal_language_code("eng"), "en");
        assert_eq!(locale_to_internal_language_code("es-ES"), "es");
        assert_eq!(locale_to_internal_language_code("spa"), "es");
    }

    #[test]
    fn test_locale_to_internal_language_code_informal_variants() {
        // the internal code round-trips
        assert_eq!(locale_to_internal_language_code("es-tu"), "es-tu");
        assert_eq!(locale_to_internal_language_code("cat-tu"), "cat-tu");
        // the private-use tag the frontends emit maps back to it
        assert_eq!(locale_to_internal_language_code("es-x-tu"), "es-tu");
        assert_eq!(locale_to_internal_language_code("ca-x-tu"), "cat-tu");
        // region and ISO 639-2/T inputs still reach the variant
        assert_eq!(locale_to_internal_language_code("es-ES-x-tu"), "es-tu");
        assert_eq!(locale_to_internal_language_code("spa-tu"), "es-tu");
        assert_eq!(locale_to_internal_language_code("CA-X-TU"), "cat-tu");
    }

    #[test]
    fn test_locale_to_internal_language_code_region_is_not_a_register() {
        // a region subtag must never be mistaken for the register marker
        assert_eq!(locale_to_internal_language_code("es-MX"), "es");
        assert_eq!(locale_to_internal_language_code("ca-ES-valencia"), "cat");
    }

    #[test]
    fn test_internal_language_code_to_bcp47() {
        assert_eq!(internal_language_code_to_bcp47("es"), "es");
        assert_eq!(internal_language_code_to_bcp47("cat"), "ca");
        assert_eq!(internal_language_code_to_bcp47("en"), "en");
        assert_eq!(internal_language_code_to_bcp47("es-tu"), "es-x-tu");
        assert_eq!(internal_language_code_to_bcp47("cat-tu"), "ca-x-tu");
    }

    #[test]
    fn test_bcp47_and_internal_code_round_trip() {
        for internal in [
            "en", "es", "cat", "eu", "gl", "nl", "tl", "fr", "es-tu", "cat-tu",
        ] {
            let tag = internal_language_code_to_bcp47(internal);
            assert_eq!(
                locale_to_internal_language_code(&tag),
                internal,
                "round trip failed for {internal} via {tag}"
            );
        }
    }
}
