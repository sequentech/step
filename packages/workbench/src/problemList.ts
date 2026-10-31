// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * `dist/problem-list/`: the problem list, built for another tool to import.
 *
 * `ui-essentials`' `ProblemList` and the sentences it reads (`ui-core`'s `problems.*`),
 * as one ES module with React, MUI and i18next left to the host. The Election Architect
 * shows an import's problems with it, so a complaint reads, and is translated, the same
 * there as in the Admin Portal. It ships beside `embed.html`, in the same build and the
 * same artifact, so a tool that fetches the voter preview fetches this with it.
 */
export * from "../../ui-essentials/src/components/ProblemList"

// Only the `problems` subtree of each catalogue, extracted when the library is built (see
// `vite.problem-list.config.ts`), rather than every language's whole catalogue.
import catalogues from "virtual:problem-translations"

/** `problems.*` of each language `ui-core` translates, keyed by its file name (`en`, `es`, …). */
export const problemTranslations: Record<string, {problems: Record<string, unknown>}> = catalogues
