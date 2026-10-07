// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Old and alternative zone names and the zone each one names, from the
//! tzdata release chrono-tz builds with (2025b), file `backward`: the
//! pre-1993 and 1995 names (`US/Eastern`), duplicates of other locations and
//! alternate spellings (`Asia/Calcutta`). The "Pre-2013 practice" section is
//! left out on purpose: those are still zone.tab locations people pick
//! (`Asia/Brunei`, `Europe/Oslo`), so they are kept as written. Names
//! without a `/` aren't listed: they are refused. The janitor's
//! `time_zone_links.py` is generated from the same lines.

use crate::services::time_zones::canonical_zone;
use chrono_tz::Tz;

/// The zone a row names, in its canonical form: an old or alternative name
/// (`US/Eastern`, `Asia/Calcutta`) becomes the zone it names. `UTC` is kept
/// as it is (the platform's default zone). Any other name without a `/`
/// (`EST`, `CET`, `Japan`) is refused: an abbreviation isn't a place.
pub fn canonical_time_zone(name: &str) -> Option<Tz> {
    let name = canonical_zone(name.trim());
    if name == "UTC" {
        return Some(Tz::UTC);
    }
    if !name.contains('/') {
        return None;
    }
    let name = TIME_ZONE_LINKS
        .binary_search_by(|(link, _)| (*link).cmp(name))
        .map(|index| TIME_ZONE_LINKS[index].1)
        .unwrap_or(name);
    name.parse::<Tz>().ok()
}

/// (old or alternative name, zone), sorted by name.
pub const TIME_ZONE_LINKS: &[(&str, &str)] = &[
    ("Africa/Asmera", "Africa/Nairobi"),
    ("Africa/Timbuktu", "Africa/Abidjan"),
    (
        "America/Argentina/ComodRivadavia",
        "America/Argentina/Catamarca",
    ),
    ("America/Atka", "America/Adak"),
    ("America/Buenos_Aires", "America/Argentina/Buenos_Aires"),
    ("America/Catamarca", "America/Argentina/Catamarca"),
    ("America/Coral_Harbour", "America/Panama"),
    ("America/Cordoba", "America/Argentina/Cordoba"),
    ("America/Ensenada", "America/Tijuana"),
    ("America/Fort_Wayne", "America/Indiana/Indianapolis"),
    ("America/Godthab", "America/Nuuk"),
    ("America/Indianapolis", "America/Indiana/Indianapolis"),
    ("America/Jujuy", "America/Argentina/Jujuy"),
    ("America/Knox_IN", "America/Indiana/Knox"),
    ("America/Louisville", "America/Kentucky/Louisville"),
    ("America/Mendoza", "America/Argentina/Mendoza"),
    ("America/Montreal", "America/Toronto"),
    ("America/Nipigon", "America/Toronto"),
    ("America/Pangnirtung", "America/Iqaluit"),
    ("America/Porto_Acre", "America/Rio_Branco"),
    ("America/Rainy_River", "America/Winnipeg"),
    ("America/Rosario", "America/Argentina/Cordoba"),
    ("America/Santa_Isabel", "America/Tijuana"),
    ("America/Shiprock", "America/Denver"),
    ("America/Thunder_Bay", "America/Toronto"),
    ("America/Virgin", "America/Puerto_Rico"),
    ("America/Yellowknife", "America/Edmonton"),
    ("Antarctica/South_Pole", "Pacific/Auckland"),
    ("Asia/Ashkhabad", "Asia/Ashgabat"),
    ("Asia/Calcutta", "Asia/Kolkata"),
    ("Asia/Choibalsan", "Asia/Ulaanbaatar"),
    ("Asia/Chongqing", "Asia/Shanghai"),
    ("Asia/Chungking", "Asia/Shanghai"),
    ("Asia/Dacca", "Asia/Dhaka"),
    ("Asia/Harbin", "Asia/Shanghai"),
    ("Asia/Istanbul", "Europe/Istanbul"),
    ("Asia/Kashgar", "Asia/Urumqi"),
    ("Asia/Katmandu", "Asia/Kathmandu"),
    ("Asia/Macao", "Asia/Macau"),
    ("Asia/Rangoon", "Asia/Yangon"),
    ("Asia/Saigon", "Asia/Ho_Chi_Minh"),
    ("Asia/Tel_Aviv", "Asia/Jerusalem"),
    ("Asia/Thimbu", "Asia/Thimphu"),
    ("Asia/Ujung_Pandang", "Asia/Makassar"),
    ("Asia/Ulan_Bator", "Asia/Ulaanbaatar"),
    ("Atlantic/Faeroe", "Atlantic/Faroe"),
    ("Atlantic/Jan_Mayen", "Europe/Berlin"),
    ("Australia/ACT", "Australia/Sydney"),
    ("Australia/Canberra", "Australia/Sydney"),
    ("Australia/Currie", "Australia/Hobart"),
    ("Australia/LHI", "Australia/Lord_Howe"),
    ("Australia/NSW", "Australia/Sydney"),
    ("Australia/North", "Australia/Darwin"),
    ("Australia/Queensland", "Australia/Brisbane"),
    ("Australia/South", "Australia/Adelaide"),
    ("Australia/Tasmania", "Australia/Hobart"),
    ("Australia/Victoria", "Australia/Melbourne"),
    ("Australia/West", "Australia/Perth"),
    ("Australia/Yancowinna", "Australia/Broken_Hill"),
    ("Brazil/Acre", "America/Rio_Branco"),
    ("Brazil/DeNoronha", "America/Noronha"),
    ("Brazil/East", "America/Sao_Paulo"),
    ("Brazil/West", "America/Manaus"),
    ("Canada/Atlantic", "America/Halifax"),
    ("Canada/Central", "America/Winnipeg"),
    ("Canada/Eastern", "America/Toronto"),
    ("Canada/Mountain", "America/Edmonton"),
    ("Canada/Newfoundland", "America/St_Johns"),
    ("Canada/Pacific", "America/Vancouver"),
    ("Canada/Saskatchewan", "America/Regina"),
    ("Canada/Yukon", "America/Whitehorse"),
    ("Chile/Continental", "America/Santiago"),
    ("Chile/EasterIsland", "Pacific/Easter"),
    ("Etc/GMT+0", "Etc/GMT"),
    ("Etc/GMT-0", "Etc/GMT"),
    ("Etc/GMT0", "Etc/GMT"),
    ("Etc/Greenwich", "Etc/GMT"),
    ("Etc/UCT", "Etc/UTC"),
    ("Etc/Universal", "Etc/UTC"),
    ("Etc/Zulu", "Etc/UTC"),
    ("Europe/Belfast", "Europe/London"),
    ("Europe/Kiev", "Europe/Kyiv"),
    ("Europe/Nicosia", "Asia/Nicosia"),
    ("Europe/Tiraspol", "Europe/Chisinau"),
    ("Europe/Uzhgorod", "Europe/Kyiv"),
    ("Europe/Zaporozhye", "Europe/Kyiv"),
    ("Mexico/BajaNorte", "America/Tijuana"),
    ("Mexico/BajaSur", "America/Mazatlan"),
    ("Mexico/General", "America/Mexico_City"),
    ("Pacific/Enderbury", "Pacific/Kanton"),
    ("Pacific/Johnston", "Pacific/Honolulu"),
    ("Pacific/Ponape", "Pacific/Guadalcanal"),
    ("Pacific/Samoa", "Pacific/Pago_Pago"),
    ("Pacific/Truk", "Pacific/Port_Moresby"),
    ("Pacific/Yap", "Pacific/Port_Moresby"),
    ("US/Alaska", "America/Anchorage"),
    ("US/Aleutian", "America/Adak"),
    ("US/Arizona", "America/Phoenix"),
    ("US/Central", "America/Chicago"),
    ("US/East-Indiana", "America/Indiana/Indianapolis"),
    ("US/Eastern", "America/New_York"),
    ("US/Hawaii", "Pacific/Honolulu"),
    ("US/Indiana-Starke", "America/Indiana/Knox"),
    ("US/Michigan", "America/Detroit"),
    ("US/Mountain", "America/Denver"),
    ("US/Pacific", "America/Los_Angeles"),
    ("US/Samoa", "Pacific/Pago_Pago"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_names_are_followed_locations_kept_and_abbreviations_refused() {
        let name = |zone: &str| canonical_time_zone(zone).map(|tz| tz.name().to_string());
        assert_eq!(name("US/Eastern").as_deref(), Some("America/New_York"));
        assert_eq!(name("Asia/Calcutta").as_deref(), Some("Asia/Kolkata"));
        assert_eq!(name("Europe/Kiev").as_deref(), Some("Europe/Kyiv"));
        assert_eq!(name(" Asia/Manila ").as_deref(), Some("Asia/Manila"));
        assert_eq!(name("Asia/Brunei").as_deref(), Some("Asia/Brunei"));
        assert_eq!(name("Europe/Oslo").as_deref(), Some("Europe/Oslo"));
        assert_eq!(name("UTC").as_deref(), Some("UTC"));
        assert_eq!(name("Etc/GMT+5").as_deref(), Some("Etc/GMT+5"));
        for refused in ["EST", "CET", "Japan", "PST8PDT", "Mars/Olympus", ""] {
            assert_eq!(name(refused), None, "{refused}");
        }
    }

    #[test]
    fn the_table_is_sorted_and_names_zones() {
        assert!(TIME_ZONE_LINKS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (link, zone) in TIME_ZONE_LINKS {
            assert!(zone.parse::<Tz>().is_ok(), "{link} -> {zone}");
            assert!(
                TIME_ZONE_LINKS
                    .binary_search_by(|(l, _)| (*l).cmp(zone))
                    .is_err(),
                "{zone} is itself listed"
            );
        }
    }
}
