// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The fields the entry popup names: per object type, a changed path and the
 * `logsScreen.entry.field.<type>.<key>` text that labels it. Any other path
 * is shown as logged.
 */
const FIELD_LABELS: Record<string, Record<string, string>> = {
    tenant: {
        "slug": "slug",
        "is_active": "isActive",
        "settings.logs_timezone": "logsTimezone",
        "settings.language_conf": "languages",
        "settings.language_conf.enabled_language_codes": "enabledLanguages",
        "settings.language_conf.default_language_code": "defaultLanguage",
        "settings.language_conf.language_detection_policy": "languageDetection",
        "settings.i18n": "translations",
        "settings.sms": "sms",
        "settings.mail": "mail",
        "settings.gapi_key": "gapiKey",
        "settings.gapi_email": "gapiEmail",
        "settings.help_links": "helpLinks",
        "settings.display_name": "displayName",
        "settings.voting_countries": "votingCountries",
        "settings.enroll_countries": "enrollCountries",
        "annotations.logo_url": "logoUrl",
        "annotations.css": "css",
        "voting_channels": "votingChannels",
        "voting_channels.online": "onlineVoting",
        "voting_channels.kiosk": "kioskVoting",
        "voting_channels.telephone": "telephoneVoting",
        "voting_channels.early_voting": "earlyVoting",
        "voting_channels.paper": "paperVoting",
    },
}

const OBJECT_TYPES = [
    "tenant",
    "tenant_log",
    "staff_account",
    "role",
    "template",
    "trustee",
    "election_type",
    "election_event",
    "election",
    "contest",
    "candidate",
    "area",
]

const owns = (map: object, key: string): boolean => Object.prototype.hasOwnProperty.call(map, key)

/** The translation key that names `path` of an object of `type`, if the portal names it. */
export const logFieldLabelKey = (type: string | null | undefined, path: string): string | null => {
    const fields = type && owns(FIELD_LABELS, type) ? FIELD_LABELS[type] : undefined
    return fields && owns(fields, path) ? `logsScreen.entry.field.${type}.${fields[path]}` : null
}

/** The translation key that names an object type, if the portal names it. */
export const logObjectTypeKey = (type: string): string | null =>
    OBJECT_TYPES.includes(type) ? `logsScreen.entry.objectType.${type}` : null
