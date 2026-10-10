// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateFromPresentation} from "@sequentech/ui-core"
import {useCallback} from "react"
import {useTranslation} from "react-i18next"

interface PresentationRecord {
    presentation?: {
        i18n?: Record<string, Record<string, string | null | undefined>>
        language_conf?: {default_language_code?: string | null} | null
    } | null
}

/**
 * Election events, elections, contests and candidates keep their names in
 * `presentation.i18n` (migration 1772358027729), so this renders a record's
 * name in the interface language.
 */
export function usePresentationName() {
    const {i18n} = useTranslation()
    return useCallback(
        (record?: PresentationRecord | null): string =>
            translateFromPresentation(record, "name", i18n.language, {
                defaultLanguageCode: "en",
            }) ?? "",
        [i18n.language]
    )
}

/** The form source of a record's name in its default language. */
export const presentationNameSource = (record?: PresentationRecord | null): string =>
    `presentation.i18n.${record?.presentation?.language_conf?.default_language_code || "en"}.name`
