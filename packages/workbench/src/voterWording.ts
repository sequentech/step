// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import portalEnglish from "../../voting-portal/src/translations/en"
import coreEnglish from "../../ui-core/src/translations/en"

/** Beside `embed.html` in a build: every wording key the voter preview draws a string for. */
export const VOTER_WORDING_FILE = "voter-wording-keys.json"

const leaves = (node: unknown, prefix = ""): string[] =>
    typeof node === "string"
        ? [prefix]
        : typeof node === "object" && node !== null
          ? Object.entries(node).flatMap(([key, value]) =>
                leaves(value, prefix === "" ? key : `${prefix}.${key}`)
            )
          : []

/**
 * The dotted keys of the voting portal's English catalogue and `ui-core`'s, which the
 * portal also loads (`selectElection.*` lives there). A tool that edits an event's wording
 * overrides, such as the Election Architect, checks its keys against these, so an override
 * nothing draws is caught rather than silently ignored.
 */
export function voterWordingKeys(): string[] {
    const keys = new Set([
        ...leaves(coreEnglish.translations),
        ...leaves(portalEnglish.translations),
    ])
    return [...keys].sort()
}
