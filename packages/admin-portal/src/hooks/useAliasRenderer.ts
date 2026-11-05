// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateFromPresentation} from "@sequentech/ui-core"
import {useCallback} from "react"
import {useTranslation} from "react-i18next"
import {configuredLanguage} from "./usePresentationName"

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

export function useAliasRenderer() {
    const {i18n} = useTranslation()

    // Fallback order: the interface language, then the given default language or
    // the record's own, then English.
    const aliasRenderer = useCallback(
        (item: unknown, defaultLang?: string) => {
            const t = (x: any) => {
                const fallback = defaultLang ?? configuredLanguage(x)
                return (
                    translateFromPresentation(x, "alias", i18n.language) ||
                    translateFromPresentation(x, "name", i18n.language) ||
                    (fallback
                        ? translateFromPresentation(x, "alias", fallback) ||
                          translateFromPresentation(x, "name", fallback)
                        : undefined) ||
                    translateFromPresentation(x, "alias", "en") ||
                    translateFromPresentation(x, "name", "en") ||
                    "-"
                )
            }

            if (item == null) return "-"

            if (isPlainObject(item)) {
                return t(item)
            }

            const s = String(item).trim()
            if (!s) return "-"

            if (s.startsWith("{")) {
                try {
                    return t(JSON.parse(s))
                } catch {
                    return "-"
                }
            }

            return "-"
        },
        [i18n.language]
    )

    return aliasRenderer
}
