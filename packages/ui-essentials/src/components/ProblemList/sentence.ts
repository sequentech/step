// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {Problem} from "./types"

/** Where a problem's sentence lives in the catalogue (`ui-core` translations). */
export const problemKey = (id: string, part: "lead" | "text"): string =>
    `problems.messages.${id}.${part}`

/**
 * The slice of i18next's `t` this needs.
 *
 * Structural rather than i18next's own `TFunction`, whose overloads change between
 * majors: the Election Architect loads this through a vendored bundle beside its
 * own i18next, and must be able to hand its `t` in.
 */
export type ProblemTranslate = (key: string, options: Record<string, unknown>) => string

export interface ProblemSentence {
    /** The whole sentence. */
    text: string
    /** Its opening words, which name the broken thing — the part a row links. */
    lead: string
    /** `text` after `lead`. */
    rest: string
}

/**
 * What a problem says, and which words of it name the broken thing.
 *
 * The core writes English. It also names each complaint and hands over the
 * specifics that complaint interpolates, so this looks the sentence up by that
 * name and substitutes them — falling back to the core's own English when there
 * is no name or no translation. `defaultValue` makes that fallback the same
 * expression rather than a branch, so a missing key can never render blank or
 * render the key itself.
 *
 * `message` is interpolated too: a few complaints wrap a sentence the core could
 * only get in English (a CSV reader's own error), and quote it.
 *
 * `lead` is carried beside the sentence in the catalogue rather than derived from
 * it, and is a prefix of it — which is what lets a row render it as a link and the
 * remainder as plain text without duplicating or losing a word. A lead that is not
 * a prefix (a half-translated language) makes the whole sentence the lead.
 */
export const problemSentence = (t: ProblemTranslate, problem: Problem): ProblemSentence => {
    if (problem.id === undefined) {
        return {text: problem.message, lead: problem.message, rest: ""}
    }
    const values: Record<string, unknown> = {message: problem.message, ...problem.details}
    // The core's details are strings, and i18next picks a plural form (`text_one`)
    // only for a numeric `count`: "2" would read every count as the general form.
    const count = problem.details?.count
    if (count !== undefined && /^\d+$/.test(count)) {
        values.count = Number(count)
    }
    const text = t(problemKey(problem.id, "text"), {...values, defaultValue: problem.message})
    const lead = t(problemKey(problem.id, "lead"), {...values, defaultValue: text})
    return text.startsWith(lead)
        ? {text, lead, rest: text.slice(lead.length)}
        : {text, lead: text, rest: ""}
}
