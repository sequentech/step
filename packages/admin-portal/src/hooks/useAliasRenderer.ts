// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateFromPresentation} from "@sequentech/ui-core"
import {useCallback} from "react"
import {useTranslation} from "react-i18next"

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

export function useAliasRenderer() {
    const {i18n} = useTranslation()

    const aliasRenderer = useCallback(
        (item: unknown, defaultLang?: string) => {
            const t = (x: any) =>
                translateFromPresentation(x, "alias", i18n.language) ||
                translateFromPresentation(x, "name", i18n.language) ||
                (defaultLang
                    ? translateFromPresentation(x, "alias", defaultLang) ||
                      translateFromPresentation(x, "name", defaultLang)
                    : undefined) ||
                translateFromPresentation(x, "alias", "en") ||
                translateFromPresentation(x, "name", "en") ||
                "-"

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
