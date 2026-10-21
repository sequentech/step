// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {ballotLogDetails, pseudonymResolution} from "./ballotLogDetails"

test("recognizes only signed privacy statement variants for ballot proof links", () => {
    const message = {
        ballot_id: "FTBE-MHRX",
        election_id: "election",
        username: "7QKM-2HXD",
        statement: {body: {BallotReceived: {}}},
    }
    expect(ballotLogDetails(message)).toEqual({
        ballotId: "FTBE-MHRX",
        electionId: "election",
        pseudonym: "7QKM-2HXD",
    })
    expect(ballotLogDetails({...message, statement: {body: {CastVote: []}}})).toBeNull()
    expect(ballotLogDetails({...message, election_id: null})).toBeNull()
})

test("resolution shows only pseudonym and reason, never the returned voter", () => {
    expect(
        pseudonymResolution({
            statement: {
                body: {
                    PseudonymResolved: {
                        pseudonym: "7QKM-2HXD",
                        reason: "Election protest",
                        voter_id: "secret",
                    },
                },
            },
        })
    ).toEqual({pseudonym: "7QKM-2HXD", reason: "Election protest"})
})
