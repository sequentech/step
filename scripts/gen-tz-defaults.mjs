#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Generates the `timezones` section of every language in
 * `.devcontainer/minio/public-assets/i18n_defaults.json`: the default short
 * label (`abbr`) and long name (`name`) of every IANA zone, as Node's Intl
 * (CLDR) writes them, plus the combined strings the report helpers print with.
 * Reports and notifications read these defaults; an event overrides any of
 * them with `templates:`/`global:` keys in its Localization tab.
 *
 * - Zones: Intl's list with CLDR's legacy ids mapped to tzdata canonical names
 *   (Asia/Calcutta → Asia/Kolkata), plus UTC.
 * - Zones with daylight saving time get `abbrDaylight`/`nameDaylight` too; the
 *   helpers pick the variant in effect at the printed instant. The two reference
 *   instants are in January and July of REFERENCE_YEAR.
 * - Other sections of the file (`application`, ...) are kept as they are.
 *
 * Usage: node scripts/gen-tz-defaults.mjs [path/to/i18n_defaults.json]
 * Re-run after a Node/ICU upgrade and commit the diff.
 */

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const target =
  process.argv[2] ??
  resolve(repoRoot, ".devcontainer/minio/public-assets/i18n_defaults.json");

const REFERENCE_YEAR = 2028;

/** The portals' language codes and the Intl locale each one formats with. */
const LANGUAGES = {
  en: "en",
  es: "es",
  cat: "ca",
  fr: "fr",
  tl: "fil",
  gl: "gl",
  nl: "nl",
  eu: "eu",
};

/** CLDR keeps these legacy ids; Step stores tzdata canonical names. */
const CLDR_TO_TZDATA = {
  "Africa/Asmera": "Africa/Asmara",
  "America/Buenos_Aires": "America/Argentina/Buenos_Aires",
  "America/Catamarca": "America/Argentina/Catamarca",
  "America/Coral_Harbour": "America/Atikokan",
  "America/Cordoba": "America/Argentina/Cordoba",
  "America/Godthab": "America/Nuuk",
  "America/Indianapolis": "America/Indiana/Indianapolis",
  "America/Jujuy": "America/Argentina/Jujuy",
  "America/Louisville": "America/Kentucky/Louisville",
  "America/Mendoza": "America/Argentina/Mendoza",
  "Asia/Calcutta": "Asia/Kolkata",
  "Asia/Katmandu": "Asia/Kathmandu",
  "Asia/Rangoon": "Asia/Yangon",
  "Asia/Saigon": "Asia/Ho_Chi_Minh",
  "Atlantic/Faeroe": "Atlantic/Faroe",
  "Europe/Kiev": "Europe/Kyiv",
  "Pacific/Enderbury": "Pacific/Kanton",
  "Pacific/Ponape": "Pacific/Pohnpei",
  "Pacific/Truk": "Pacific/Chuuk",
};

/**
 * Zone defaults that differ from CLDR (design §1): a zone's common
 * abbreviation, never a client choice. Keyed by language, then zone.
 */
const ABBR_DEFAULTS = {
  en: { "Asia/Manila": "PhST" },
};

/** The combined strings the report helpers print (design §4). */
const COMBINED_TEXTS = {
  dateTimeZone: "{{dateTime}} {{zone}}",
  voterDateTimeZone: "{{dateTime}} {{zoneName}}",
};

const zones = () => {
  const names = new Set(
    Intl.supportedValuesOf("timeZone").map(
      (zone) => CLDR_TO_TZDATA[zone] ?? zone,
    ),
  );
  names.add("UTC");
  return [...names].sort();
};

const zoneNamePart = (locale, zone, instant, style) =>
  new Intl.DateTimeFormat(locale, { timeZone: zone, timeZoneName: style })
    .formatToParts(instant)
    .find((part) => part.type === "timeZoneName")?.value;

/** Minutes east of UTC of `zone` at `instant`. */
const offsetMinutes = (zone, instant) => {
  const text = zoneNamePart("en-US", zone, instant, "longOffset") ?? "GMT";
  const match = /GMT([+-])(\d{2}):(\d{2})/.exec(text);
  if (!match) return 0;
  const minutes = Number(match[2]) * 60 + Number(match[3]);
  return match[1] === "-" ? -minutes : minutes;
};

/** The standard and (if any) daylight reference instants of a zone. */
const referenceInstants = (zone) => {
  const january = new Date(Date.UTC(REFERENCE_YEAR, 0, 15, 12));
  const july = new Date(Date.UTC(REFERENCE_YEAR, 6, 15, 12));
  const januaryOffset = offsetMinutes(zone, january);
  const julyOffset = offsetMinutes(zone, july);
  if (januaryOffset === julyOffset) return { standard: january };
  return januaryOffset < julyOffset
    ? { standard: january, daylight: july }
    : { standard: july, daylight: january };
};

const timezonesSection = (language, locale) => {
  const abbr = {};
  const abbrDaylight = {};
  const name = {};
  const nameDaylight = {};
  for (const zone of zones()) {
    const { standard, daylight } = referenceInstants(zone);
    abbr[zone] = zoneNamePart(locale, zone, standard, "short");
    name[zone] = zoneNamePart(locale, zone, standard, "long");
    if (daylight) {
      const shortDaylight = zoneNamePart(locale, zone, daylight, "short");
      const longDaylight = zoneNamePart(locale, zone, daylight, "long");
      if (shortDaylight !== abbr[zone]) abbrDaylight[zone] = shortDaylight;
      if (longDaylight !== name[zone]) nameDaylight[zone] = longDaylight;
    }
  }
  for (const [zone, label] of Object.entries(ABBR_DEFAULTS[language] ?? {})) {
    abbr[zone] = label;
    delete abbrDaylight[zone];
  }
  return { ...COMBINED_TEXTS, abbr, abbrDaylight, name, nameDaylight };
};

const defaults = JSON.parse(readFileSync(target, "utf8"));
for (const [language, locale] of Object.entries(LANGUAGES)) {
  defaults[language] = {
    ...(defaults[language] ?? {}),
    timezones: timezonesSection(language, locale),
  };
}
writeFileSync(target, JSON.stringify(defaults, null, 2));
console.log(
  `Wrote ${zones().length} zones × ${Object.keys(LANGUAGES).length} languages to ${target} (ICU ${process.versions.icu}, tz ${process.versions.tz})`,
);
