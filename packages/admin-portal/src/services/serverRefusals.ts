// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {TFunction} from "i18next"
import {getGraphQLActionErrorReason, hasGraphQLActionErrorCode} from "./graphqlActionError"

// Refusals the server answers with a fixed text. Each is shown as the portal's
// own translation of that text, never as the server's words.

/** The code `/get-upload-url` answers for a document name it does not take. */
export const INVALID_DOCUMENT_NAME_ERROR_CODE = "invalid-document-name"

// The database refuses such a name with the same text and no code.
const INVALID_DOCUMENT_NAME_TEXT = "A document name cannot contain"

/** Whether the server refused a file because of its name. */
export const isInvalidDocumentNameError = (error: unknown): boolean =>
    hasGraphQLActionErrorCode(error, INVALID_DOCUMENT_NAME_ERROR_CODE, [INVALID_DOCUMENT_NAME_TEXT])

/** Why the server refused a file's name, translated; undefined for any other error. */
export const documentNameRefusal = (t: TFunction, error: unknown): string | undefined =>
    isInvalidDocumentNameError(error) ? t("refusals.documentName") : undefined

// How the server refuses what holds decoded ballots before post-processing,
// and any phase-bound operation while a phase or safety state change runs.
const BEFORE_POST_PROCESSING_TEXT = "requires electoral phase PostProcessing"
const PHASE_CHANGING_TEXT = "Election safety state is changing"

/** The key of the refusal while a phase or safety state change runs. */
export const PHASE_CHANGING_KEY = "refusals.phaseChanging"

// The refusals carry the generic code InternalServerError, so their text is
// matched, wherever Hasura or Apollo left it (a plain message as well).
const carriesText = (error: unknown, text: string): boolean =>
    typeof error === "string" ? error.includes(text) : hasGraphQLActionErrorCode(error, text)

/**
 * The translation key for a refusal before post-processing: `waitKey` for the
 * refusal itself, `PHASE_CHANGING_KEY` while the phase or safety state is
 * changing; undefined for any other error.
 */
export const phaseRefusalKey = (error: unknown, waitKey: string): string | undefined => {
    if (carriesText(error, BEFORE_POST_PROCESSING_TEXT)) return waitKey
    if (carriesText(error, PHASE_CHANGING_TEXT)) return PHASE_CHANGING_KEY
    return undefined
}

/** A refused results tally: the key of its translation and the election it names. */
export interface SecondTallyRefusal {
    key: string
    electionId: string
}

const SECOND_TALLY_REFUSALS: Array<[RegExp, string]> = [
    [/^Election ([^\s:]+): its results are already counted/, "refusals.resultsAlreadyCounted"],
    [
        /^Election ([^\s:]+): a results tally of it has not finished/,
        "refusals.resultsTallyUnfinished",
    ],
]

/**
 * Why the server refused a results tally of an election that is counted or
 * still being counted; undefined for any other error.
 */
export const secondTallyRefusal = (error: unknown): SecondTallyRefusal | undefined => {
    const reason = getGraphQLActionErrorReason(error)
    if (!reason) return undefined
    for (const [pattern, key] of SECOND_TALLY_REFUSALS) {
        const match = pattern.exec(reason)
        if (match) return {key, electionId: match[1]}
    }
    return undefined
}

/**
 * The translated refusal of a results tally of an election that is counted or
 * still being counted. The server names the election by its ID; `electionName`
 * gives the name the admin knows it by, if any. Undefined for any other error.
 */
export const secondTallyMessage = (
    t: TFunction,
    error: unknown,
    electionName: (electionId: string) => string | undefined
): string | undefined => {
    const refusal = secondTallyRefusal(error)
    if (!refusal) return undefined
    return t(refusal.key, {election: electionName(refusal.electionId) ?? refusal.electionId})
}
