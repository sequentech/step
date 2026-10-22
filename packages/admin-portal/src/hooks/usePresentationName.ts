// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateFromPresentation} from "@sequentech/ui-core"
import {useCallback} from "react"
import {useGetOne} from "react-admin"
import {useTranslation} from "react-i18next"

interface PresentationRecord {
    presentation?: {
        i18n?: Record<string, Record<string, string | null | undefined>>
        language_conf?: {default_language_code?: string | null} | null
    } | null
}

type LanguageConfigured = PresentationRecord["presentation"]

/**
 * The default language a record's `presentation.language_conf` sets. Accepts
 * the record or its presentation.
 */
export const configuredLanguage = (record: unknown): string | undefined => {
    const owner = record as (PresentationRecord & LanguageConfigured) | null | undefined
    const code =
        owner?.presentation?.language_conf?.default_language_code ??
        owner?.language_conf?.default_language_code
    return typeof code === "string" && code ? code : undefined
}

/**
 * Election events, elections, contests and candidates keep their names in
 * `presentation.i18n` (migration 1772358027729), so this renders a record's
 * name in the interface language, else in the record's default language, else
 * in English.
 */
export function usePresentationName() {
    const {i18n} = useTranslation()
    return useCallback(
        (record?: PresentationRecord | null): string =>
            translateFromPresentation(record, "name", i18n.language, {
                defaultLanguageCode: configuredLanguage(record),
            }) ??
            translateFromPresentation(record, "name", "en") ??
            "",
        [i18n.language]
    )
}

/** The form source of a record's name in its default language. */
export const presentationNameSource = (record?: PresentationRecord | null): string =>
    `presentation.i18n.${record?.presentation?.language_conf?.default_language_code || "en"}.name`

interface ChildRecord {
    election_id?: string | null
    election_event_id?: string | null
}

/**
 * The default language a contest or candidate inherits: its election's, else
 * its event's, as the voting portal resolves it.
 */
export function useInheritedLanguage(record?: ChildRecord | null): string | undefined {
    const electionId = record?.election_id ?? ""
    const eventId = record?.election_event_id ?? ""
    const {data: election} = useGetOne(
        "sequent_backend_election",
        {id: electionId},
        {enabled: Boolean(electionId)}
    )
    const {data: event} = useGetOne(
        "sequent_backend_election_event",
        {id: eventId},
        {enabled: Boolean(eventId)}
    )
    return configuredLanguage(election) ?? configuredLanguage(event)
}
