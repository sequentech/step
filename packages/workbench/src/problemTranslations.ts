// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import cat from "../../ui-core/src/translations/cat"
import en from "../../ui-core/src/translations/en"
import es from "../../ui-core/src/translations/es"
import eu from "../../ui-core/src/translations/eu"
import fr from "../../ui-core/src/translations/fr"
import gl from "../../ui-core/src/translations/gl"
import nl from "../../ui-core/src/translations/nl"
import tl from "../../ui-core/src/translations/tl"

/** Every catalogue `ui-core` ships, by the language code its file is named after. */
export const CATALOGUES: Record<string, {translations: {problems?: unknown}}> = {
    cat,
    en,
    es,
    eu,
    fr,
    gl,
    nl,
    tl,
}

/**
 * `problems.*` of each catalogue: the sentences `ProblemList` reads. Built into
 * `dist/problem-list/` (see `vite.problem-list.config.ts`) without the rest of each
 * catalogue, which a tool showing an import's problems has no use for.
 */
export function problemTranslations(): Record<string, {problems: Record<string, unknown>}> {
    return Object.fromEntries(
        Object.entries(CATALOGUES).flatMap(([language, catalogue]) => {
            const problems = catalogue.translations.problems
            return typeof problems === "object" && problems !== null
                ? [[language, {problems: problems as Record<string, unknown>}]]
                : []
        })
    )
}
