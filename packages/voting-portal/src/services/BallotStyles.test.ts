// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ELECTION_WITH_INVALID} from "../fixtures/election"
import {BallotStyleConfigurationError, getBallotStyleConfigurationError} from "./BallotStyles"

describe("ballot style slate validation", () => {
    it("accepts a sound slate configuration", () => {
        const ballot = structuredClone(ELECTION_WITH_INVALID)
        const [contest] = ballot.contests
        ballot.election_annotations = {
            "sequent.slates": JSON.stringify({
                version: 1,
                slates: [
                    {
                        id: "forward",
                        name: {en: "Forward Together"},
                        members: {[contest.id]: [contest.candidates[0].id]},
                    },
                ],
            }),
        }
        expect(getBallotStyleConfigurationError(ballot)).toBeUndefined()
    })

    it("rejects an invalid slate configuration with the reason", () => {
        const ballot = structuredClone(ELECTION_WITH_INVALID)
        ballot.election_annotations = {"sequent.slates": "{"}
        const error = getBallotStyleConfigurationError(ballot)
        expect(error).toBeInstanceOf(BallotStyleConfigurationError)
        expect(error?.translationKey).toBe("errors.configuration.invalidSlateConfiguration")
        expect(error?.translationParams.reason).toContain(
            "the slate configuration is not valid JSON"
        )
    })
})
