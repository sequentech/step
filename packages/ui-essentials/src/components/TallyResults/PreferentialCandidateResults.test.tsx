// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"

// ui-core's built dist is unavailable when this package's tests run alone, so
// load the number format modules from source.
jest.mock(
    "@sequentech/ui-core",
    () => ({
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/types/VotingChannel"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/types/ElectionEventPresentation"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/numberFormat"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/NumberFormatContext"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/percentFormatter"
        ),
    }),
    {virtual: true}
)

import {ENumberFormatPolicy, NumberFormatProvider} from "@sequentech/ui-core"
import {PreferentialCandidateResults} from "./PreferentialCandidateResults"
import type {PreferentialProcessResults} from "./types"

const candidateA = {id: "candidate-a", name: "Candidate A"}
const candidateB = {id: "candidate-b", name: "Candidate B"}

const processResults: PreferentialProcessResults = {
    name_references: [candidateA, candidateB],
    round_count: 1,
    max_rounds: 1,
    rounds: [
        {
            winner: candidateA,
            candidates_wins: {
                [candidateA.id]: {
                    name: candidateA.name,
                    wins: 1234567,
                    transference: 0,
                    percentage: 0.456789,
                },
                [candidateB.id]: {
                    name: candidateB.name,
                    wins: 999,
                    transference: 0,
                    percentage: 0.000809,
                },
            },
            eliminated_candidates: null,
            active_candidates_count: 2,
            active_ballots_count: 1235566,
            exhausted_ballots_count: 0,
        },
    ],
}

// The visible text of each round's votes cell, without the separators React
// writes between adjacent text nodes.
const roundVotes = (markup: string): string[] =>
    Array.from(
        markup
            .replace(/<!-- -->/g, "")
            .matchAll(/seq-tally-results-preferential-results__round-votes[^>]*>([^<]*)</g),
        (match) => match[1]
    )

describe("PreferentialCandidateResults", () => {
    it("writes round votes and percentages in the event's number format", () => {
        const markup = renderToStaticMarkup(
            <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                <PreferentialCandidateResults
                    processResults={processResults}
                    candidates={[candidateA, candidateB]}
                />
            </NumberFormatProvider>
        )

        expect(roundVotes(markup)).toEqual(["1.234.567 (45,68%)", "999 (0,08%)"])
    })

    it("keeps the comma grouped output for events without a number format", () => {
        const markup = renderToStaticMarkup(
            <PreferentialCandidateResults
                processResults={processResults}
                candidates={[candidateA, candidateB]}
            />
        )

        expect(roundVotes(markup)).toEqual(["1,234,567 (45.68%)", "999 (0.08%)"])
    })
})
