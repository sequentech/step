// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ballotIdChars, FULL_BALLOT_ID_LENGTH, SHORT_BALLOT_ID_LENGTH} from "./vitrine/ballotId"

/** A pseudonym is eight characters: `K7QM:2XWD`. */
export const PSEUDONYM_LENGTH = 8
const MAX_SEARCH_LENGTH = 80
const ALPHABET = /^[0-9A-HJKMNP-TV-Z]*$/

/** What the search box asks the log for. The server reads the text the same way. */
export interface IBallotLogSearch {
    /** The text sent to the server, trimmed. */
    query: string
    /** The characters of a pseudonym start looked for, or null when none are. */
    pseudonym: string | null
    /** The characters of a Tracking ID start looked for, or null when none are. */
    trackingId: string | null
}

/**
 * Reads what was typed in the log's search box. A colon means a pseudonym
 * (`K7QM:2XWD`) and only pseudonyms are searched; a hyphen means a Tracking ID
 * (`FTBE-MHRX`) and only Tracking IDs are; with neither, both are. A Tracking
 * ID start has at least the 8 characters of the short ID. O is 0, and I and L
 * are 1. Null for text that is the start of neither, and so is never sent.
 * An empty search is the unsearched log.
 */
export const parseBallotLogSearch = (input: string): IBallotLogSearch | null => {
    const query = input.trim()
    if (query.length > MAX_SEARCH_LENGTH || /[^0-9A-Za-z\s:-]/.test(query)) return null
    const colon = query.includes(":")
    const hyphen = query.includes("-")
    const characters = ballotIdChars(query.replace(/:/g, ""))
    if (!ALPHABET.test(characters)) return null
    return {
        query,
        pseudonym:
            (colon || !hyphen) && characters.length >= 1 && characters.length <= PSEUDONYM_LENGTH
                ? characters
                : null,
        trackingId:
            !colon &&
            characters.length >= SHORT_BALLOT_ID_LENGTH &&
            characters.length <= FULL_BALLOT_ID_LENGTH
                ? characters
                : null,
    }
}

/** Whether a search that asks for nothing in particular can match: it has a
 * start of a pseudonym or of a Tracking ID to look for. */
export const searchesAnything = (search: IBallotLogSearch): boolean =>
    search.query === "" || search.pseudonym !== null || search.trackingId !== null

/** The start of a displayed code the search matched, in characters of the
 * code without its separators. Zero when the code and the search do not start
 * the same: a short ID shown for a longer search matches as far as it goes. */
export const matchedCharacters = (code: string, start: string | null): number => {
    if (!start) return 0
    const characters = ballotIdChars(code.replace(/:/g, ""))
    return characters.startsWith(start) || start.startsWith(characters)
        ? Math.min(start.length, characters.length)
        : 0
}
