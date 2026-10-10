// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Country calling codes (ITU-T E.164) and the ISO 3166-1 alpha-2 country
//! each one is assigned to, for message records and monthly volumes.
//!
//! A static table: no lookups at run time. Where several countries share a
//! code, the longer prefixes listed in [`SHARED_PREFIXES`] tell them apart
//! (North American area codes, Kazakhstan); otherwise the code's main
//! country is used. So volumes of these territories are counted under the
//! main country: Guernsey, Jersey and the Isle of Man (44, GB), Mayotte
//! (262, RE), Saint Barthélemy and Saint Martin (590, GP), the Caribbean
//! Netherlands (599, CW), Åland (358, FI), Svalbard (47, NO), Christmas and
//! Cocos Islands (61, AU), Western Sahara (212, MA) and Vatican City (39,
//! IT). Global services ([`NON_GEOGRAPHIC_CODES`], such as Inmarsat 870)
//! are valid calling codes with no country.

/// Calling code → main country. Codes are prefix-free, so the first match
/// by length is the code.
pub const CALLING_CODES: &[(&str, &str)] = &[
    ("1", "US"),
    ("7", "RU"),
    ("20", "EG"),
    ("27", "ZA"),
    ("30", "GR"),
    ("31", "NL"),
    ("32", "BE"),
    ("33", "FR"),
    ("34", "ES"),
    ("36", "HU"),
    ("39", "IT"),
    ("40", "RO"),
    ("41", "CH"),
    ("43", "AT"),
    ("44", "GB"),
    ("45", "DK"),
    ("46", "SE"),
    ("47", "NO"),
    ("48", "PL"),
    ("49", "DE"),
    ("51", "PE"),
    ("52", "MX"),
    ("53", "CU"),
    ("54", "AR"),
    ("55", "BR"),
    ("56", "CL"),
    ("57", "CO"),
    ("58", "VE"),
    ("60", "MY"),
    ("61", "AU"),
    ("62", "ID"),
    ("63", "PH"),
    ("64", "NZ"),
    ("65", "SG"),
    ("66", "TH"),
    ("81", "JP"),
    ("82", "KR"),
    ("84", "VN"),
    ("86", "CN"),
    ("90", "TR"),
    ("91", "IN"),
    ("92", "PK"),
    ("93", "AF"),
    ("94", "LK"),
    ("95", "MM"),
    ("98", "IR"),
    ("211", "SS"),
    ("212", "MA"),
    ("213", "DZ"),
    ("216", "TN"),
    ("218", "LY"),
    ("220", "GM"),
    ("221", "SN"),
    ("222", "MR"),
    ("223", "ML"),
    ("224", "GN"),
    ("225", "CI"),
    ("226", "BF"),
    ("227", "NE"),
    ("228", "TG"),
    ("229", "BJ"),
    ("230", "MU"),
    ("231", "LR"),
    ("232", "SL"),
    ("233", "GH"),
    ("234", "NG"),
    ("235", "TD"),
    ("236", "CF"),
    ("237", "CM"),
    ("238", "CV"),
    ("239", "ST"),
    ("240", "GQ"),
    ("241", "GA"),
    ("242", "CG"),
    ("243", "CD"),
    ("244", "AO"),
    ("245", "GW"),
    ("246", "IO"),
    ("247", "AC"),
    ("248", "SC"),
    ("249", "SD"),
    ("250", "RW"),
    ("251", "ET"),
    ("252", "SO"),
    ("253", "DJ"),
    ("254", "KE"),
    ("255", "TZ"),
    ("256", "UG"),
    ("257", "BI"),
    ("258", "MZ"),
    ("260", "ZM"),
    ("261", "MG"),
    ("262", "RE"),
    ("263", "ZW"),
    ("264", "NA"),
    ("265", "MW"),
    ("266", "LS"),
    ("267", "BW"),
    ("268", "SZ"),
    ("269", "KM"),
    ("290", "SH"),
    ("291", "ER"),
    ("297", "AW"),
    ("298", "FO"),
    ("299", "GL"),
    ("350", "GI"),
    ("351", "PT"),
    ("352", "LU"),
    ("353", "IE"),
    ("354", "IS"),
    ("355", "AL"),
    ("356", "MT"),
    ("357", "CY"),
    ("358", "FI"),
    ("359", "BG"),
    ("370", "LT"),
    ("371", "LV"),
    ("372", "EE"),
    ("373", "MD"),
    ("374", "AM"),
    ("375", "BY"),
    ("376", "AD"),
    ("377", "MC"),
    ("378", "SM"),
    ("379", "VA"),
    ("380", "UA"),
    ("381", "RS"),
    ("382", "ME"),
    ("383", "XK"),
    ("385", "HR"),
    ("386", "SI"),
    ("387", "BA"),
    ("389", "MK"),
    ("420", "CZ"),
    ("421", "SK"),
    ("423", "LI"),
    ("500", "FK"),
    ("501", "BZ"),
    ("502", "GT"),
    ("503", "SV"),
    ("504", "HN"),
    ("505", "NI"),
    ("506", "CR"),
    ("507", "PA"),
    ("508", "PM"),
    ("509", "HT"),
    ("590", "GP"),
    ("591", "BO"),
    ("592", "GY"),
    ("593", "EC"),
    ("594", "GF"),
    ("595", "PY"),
    ("596", "MQ"),
    ("597", "SR"),
    ("598", "UY"),
    ("599", "CW"),
    ("670", "TL"),
    ("672", "NF"),
    ("673", "BN"),
    ("674", "NR"),
    ("675", "PG"),
    ("676", "TO"),
    ("677", "SB"),
    ("678", "VU"),
    ("679", "FJ"),
    ("680", "PW"),
    ("681", "WF"),
    ("682", "CK"),
    ("683", "NU"),
    ("685", "WS"),
    ("686", "KI"),
    ("687", "NC"),
    ("688", "TV"),
    ("689", "PF"),
    ("690", "TK"),
    ("691", "FM"),
    ("692", "MH"),
    ("850", "KP"),
    ("852", "HK"),
    ("853", "MO"),
    ("855", "KH"),
    ("856", "LA"),
    ("880", "BD"),
    ("886", "TW"),
    ("960", "MV"),
    ("961", "LB"),
    ("962", "JO"),
    ("963", "SY"),
    ("964", "IQ"),
    ("965", "KW"),
    ("966", "SA"),
    ("967", "YE"),
    ("968", "OM"),
    ("970", "PS"),
    ("971", "AE"),
    ("972", "IL"),
    ("973", "BH"),
    ("974", "QA"),
    ("975", "BT"),
    ("976", "MN"),
    ("977", "NP"),
    ("992", "TJ"),
    ("993", "TM"),
    ("994", "AZ"),
    ("995", "GE"),
    ("996", "KG"),
    ("998", "UZ"),
];

/// Longer prefixes (calling code plus the first national digits) of a
/// country that shares its calling code: Canada and the Caribbean in the
/// North American plan, and Kazakhstan.
pub const SHARED_PREFIXES: &[(&str, &str)] = &[
    // Canada.
    ("1204", "CA"),
    ("1226", "CA"),
    ("1236", "CA"),
    ("1249", "CA"),
    ("1250", "CA"),
    ("1263", "CA"),
    ("1289", "CA"),
    ("1306", "CA"),
    ("1343", "CA"),
    ("1354", "CA"),
    ("1365", "CA"),
    ("1367", "CA"),
    ("1368", "CA"),
    ("1382", "CA"),
    ("1387", "CA"),
    ("1403", "CA"),
    ("1416", "CA"),
    ("1418", "CA"),
    ("1428", "CA"),
    ("1431", "CA"),
    ("1437", "CA"),
    ("1438", "CA"),
    ("1450", "CA"),
    ("1460", "CA"),
    ("1468", "CA"),
    ("1474", "CA"),
    ("1506", "CA"),
    ("1514", "CA"),
    ("1519", "CA"),
    ("1537", "CA"),
    ("1548", "CA"),
    ("1579", "CA"),
    ("1581", "CA"),
    ("1584", "CA"),
    ("1587", "CA"),
    ("1600", "CA"),
    ("1604", "CA"),
    ("1613", "CA"),
    ("1639", "CA"),
    ("1647", "CA"),
    ("1672", "CA"),
    ("1683", "CA"),
    ("1705", "CA"),
    ("1709", "CA"),
    ("1742", "CA"),
    ("1753", "CA"),
    ("1778", "CA"),
    ("1780", "CA"),
    ("1782", "CA"),
    ("1807", "CA"),
    ("1819", "CA"),
    ("1825", "CA"),
    ("1867", "CA"),
    ("1873", "CA"),
    ("1879", "CA"),
    ("1902", "CA"),
    ("1905", "CA"),
    ("1942", "CA"),
    // The Caribbean and the Pacific in the North American plan.
    ("1242", "BS"),
    ("1246", "BB"),
    ("1264", "AI"),
    ("1268", "AG"),
    ("1284", "VG"),
    ("1340", "VI"),
    ("1345", "KY"),
    ("1441", "BM"),
    ("1473", "GD"),
    ("1649", "TC"),
    ("1658", "JM"),
    ("1664", "MS"),
    ("1670", "MP"),
    ("1671", "GU"),
    ("1684", "AS"),
    ("1721", "SX"),
    ("1758", "LC"),
    ("1767", "DM"),
    ("1784", "VC"),
    ("1787", "PR"),
    ("1809", "DO"),
    ("1829", "DO"),
    ("1849", "DO"),
    ("1868", "TT"),
    ("1869", "KN"),
    ("1876", "JM"),
    ("1939", "PR"),
    // Kazakhstan.
    ("76", "KZ"),
    ("77", "KZ"),
];

/// Calling codes of global services (freephone, Inmarsat, satellite and
/// international networks, UIFN): valid numbers with no country.
pub const NON_GEOGRAPHIC_CODES: &[&str] = &[
    "800", "808", "870", "878", "881", "882", "883", "888", "979",
];

/// The calling code of an E.164 number (`+` and digits), as digits.
pub fn calling_code(e164: &str) -> Option<&'static str> {
    let digits = e164.strip_prefix('+')?;
    (1..=3)
        .filter_map(|length| digits.get(..length))
        .find_map(|prefix| {
            CALLING_CODES
                .iter()
                .map(|(code, _)| *code)
                .chain(NON_GEOGRAPHIC_CODES.iter().copied())
                .find(|code| *code == prefix)
        })
}

/// The ISO 3166-1 alpha-2 country of an E.164 number, or `None` when its
/// calling code is unknown.
pub fn iso_country(e164: &str) -> Option<&'static str> {
    let digits = e164.strip_prefix('+')?;
    if let Some((_, country)) = SHARED_PREFIXES
        .iter()
        .find(|(prefix, _)| digits.starts_with(prefix))
    {
        return Some(country);
    }
    let code = calling_code(e164)?;
    CALLING_CODES
        .iter()
        .find(|(candidate, _)| *candidate == code)
        .map(|(_, country)| *country)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn calling_codes_are_matched_by_length() {
        assert_eq!(calling_code("+15550123456"), Some("1"));
        assert_eq!(calling_code("+447700900123"), Some("44"));
        assert_eq!(calling_code("+971501234567"), Some("971"));
        assert_eq!(calling_code("+0123456789"), None);
        assert_eq!(calling_code("639171234567"), None);
    }

    #[test]
    fn numbers_map_to_their_iso_country() {
        assert_eq!(iso_country("+639171234567"), Some("PH"));
        assert_eq!(iso_country("+966501234567"), Some("SA"));
        assert_eq!(iso_country("+34600000001"), Some("ES"));
        assert_eq!(iso_country("+12025550123"), Some("US"));
        assert_eq!(iso_country("+14165550123"), Some("CA"));
        assert_eq!(iso_country("+18765550123"), Some("JM"));
        assert_eq!(iso_country("+77011234567"), Some("KZ"));
        assert_eq!(iso_country("+79161234567"), Some("RU"));
        assert_eq!(iso_country("+999"), None);
        // A global service has a calling code and no country.
        assert_eq!(calling_code("+870773112345"), Some("870"));
        assert_eq!(iso_country("+870773112345"), None);
    }

    #[test]
    fn the_table_is_prefix_free_and_well_formed() {
        let codes: Vec<&str> = CALLING_CODES
            .iter()
            .map(|(code, _)| *code)
            .chain(NON_GEOGRAPHIC_CODES.iter().copied())
            .collect();
        assert_eq!(codes.iter().collect::<HashSet<_>>().len(), codes.len());
        for code in &codes {
            assert!(code.chars().all(|c| c.is_ascii_digit()));
            for other in &codes {
                assert!(
                    code == other || !other.starts_with(code),
                    "{code} is a prefix of {other}"
                );
            }
        }
        for (_, country) in CALLING_CODES.iter().chain(SHARED_PREFIXES) {
            assert_eq!(country.len(), 2);
            assert!(country.chars().all(|c| c.is_ascii_uppercase()));
        }
        for (prefix, _) in SHARED_PREFIXES {
            assert!(calling_code(&format!("+{prefix}0000000")).is_some());
        }
    }
}
