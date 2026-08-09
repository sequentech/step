// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IAreaContestResults, ICandidateResults, IInvalidVotes} from "@/types/TallySheets"

const unsignedIntegerRegExp = /^[0-9]+$/

const toOptionalU64 = (value: unknown): number | undefined => {
    const parsed =
        typeof value === "number"
            ? value
            : typeof value === "string" && unsignedIntegerRegExp.test(value)
              ? Number(value)
              : undefined

    return typeof parsed === "number" && Number.isSafeInteger(parsed) && parsed >= 0
        ? parsed
        : undefined
}

const normalizeInvalidVotes = (invalidVotes?: IInvalidVotes): IInvalidVotes | undefined =>
    invalidVotes
        ? {
              total_invalid: toOptionalU64(invalidVotes.total_invalid),
              implicit_invalid: toOptionalU64(invalidVotes.implicit_invalid),
              explicit_invalid: toOptionalU64(invalidVotes.explicit_invalid),
          }
        : undefined

const normalizeCandidateResults = (
    candidateResults: IAreaContestResults["candidate_results"]
): {[id: string]: ICandidateResults} =>
    Object.fromEntries(
        Object.entries(candidateResults ?? {}).map(([candidateId, result]) => [
            candidateId,
            {
                ...result,
                total_votes: toOptionalU64(result.total_votes),
            },
        ])
    )

/**
 * Keeps transient form values safe for the Rust/WASM u64 boundary. Text inputs and
 * old local-storage drafts can contain numeric strings or an empty string even though
 * the public TypeScript model only exposes numbers.
 */
export const normalizeAreaContestResults = (content: IAreaContestResults): IAreaContestResults => ({
    ...content,
    total_votes: toOptionalU64(content.total_votes),
    total_valid_votes: toOptionalU64(content.total_valid_votes),
    invalid_votes: normalizeInvalidVotes(content.invalid_votes),
    total_blank_votes: toOptionalU64(content.total_blank_votes),
    census: toOptionalU64(content.census),
    candidate_results: normalizeCandidateResults(content.candidate_results),
})
