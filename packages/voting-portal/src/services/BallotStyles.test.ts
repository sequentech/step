// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ELECTION_WITH_INVALID} from "../fixtures/election"
import {BallotStyleConfigurationError, getBallotStyleConfigurationError} from "./BallotStyles"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual<typeof import("@sequentech/ui-core")>("@sequentech/ui-core"),
    ...jest.requireActual<typeof import("../../../ui-core/src/services/candidatePresentation")>(
        "../../../ui-core/src/services/candidatePresentation"
    ),
}))

describe("ballot style validation", () => {
    it("accepts ordinary candidates plus one explicit blank and one invalid marker", () => {
        expect(getBallotStyleConfigurationError(ELECTION_WITH_INVALID)).toBeUndefined()
    })

    it.each([
        ["is_explicit_blank", "errors.configuration.multipleExplicitBlankCandidates"],
        ["is_explicit_invalid", "errors.configuration.multipleExplicitInvalidCandidates"],
    ] as const)("rejects duplicate %s markers with an operator-readable error", (flag, key) => {
        const ballot = structuredClone(ELECTION_WITH_INVALID)
        const original = ballot.contests[0].candidates[0]
        ballot.contests[0].candidates = [0, 1].map((index) => ({
            ...original,
            id: `marker-${index}`,
            presentation: {[flag]: true},
        }))
        const error = getBallotStyleConfigurationError(ballot)
        expect(error).toBeInstanceOf(BallotStyleConfigurationError)
        expect(error?.name).toBe("BallotStyleConfigurationError")
        expect(error?.message).toBe(key)
        expect(error?.translationKey).toBe(key)
        expect(error?.translationParams).toEqual({count: "2"})
    })

    it("checks later contests instead of stopping after the first valid contest", () => {
        const ballot = structuredClone(ELECTION_WITH_INVALID)
        const second = structuredClone(ballot.contests[0])
        second.id = "second-contest"
        second.candidates = second.candidates.slice(0, 2).map((candidate) => ({
            ...candidate,
            presentation: {is_explicit_invalid: true},
        }))
        ballot.contests.push(second)
        expect(getBallotStyleConfigurationError(ballot)?.translationKey).toBe(
            "errors.configuration.multipleExplicitInvalidCandidates"
        )
    })
})
