// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {isBallotBoxSealedError, sealErrorText, sealText} from "./ballotBoxSealErrors"

const DETAIL = "This election has sealed ballot boxes and cannot be deleted."

describe("isBallotBoxSealedError", () => {
    it("finds the refusal in the message", () => {
        expect(isBallotBoxSealedError(new Error("ballot_box_sealed"))).toBe(true)
        expect(isBallotBoxSealedError("GraphQL error: ballot_box_sealed")).toBe(true)
    })

    it("finds the refusal in Hasura's extensions, as an Apollo error carries them", () => {
        const error = Object.assign(new Error("database query error"), {
            graphQLErrors: [
                {
                    message: "database query error",
                    extensions: {
                        code: "unexpected",
                        internal: {
                            error: {
                                message: "ballot_box_sealed",
                                status_code: "42501",
                                description: DETAIL,
                            },
                        },
                    },
                },
            ],
        })
        expect(isBallotBoxSealedError(error)).toBe(true)
    })

    it("does not take other errors, or nothing, for the refusal", () => {
        expect(isBallotBoxSealedError(new Error("Foreign key violation"))).toBe(false)
        expect(isBallotBoxSealedError({body: {errors: [{message: "permission denied"}]}})).toBe(
            false
        )
        expect(isBallotBoxSealedError(undefined)).toBe(false)
        expect(isBallotBoxSealedError(null)).toBe(false)
    })

    it("copes with errors that refer to themselves", () => {
        const error: Record<string, unknown> = {message: "ballot_box_sealed"}
        error.self = error
        expect(isBallotBoxSealedError(error)).toBe(true)
    })
})

describe("sealText and sealErrorText", () => {
    const t = (key: string) => `t:${key}`
    it.each([
        [
            "a ballot does not match its Ballot ID (stored x, content hashes to y)",
            "dashboard.ballotBoxes.failure.ballotIdMismatch",
        ],
        [
            "the ballot 1 has no content or no Ballot ID",
            "dashboard.ballotBoxes.failure.missingContent",
        ],
        [
            "a seal for this ballot box is already on the bulletin board",
            "dashboard.ballotBoxes.failure.alreadyOnBoard",
        ],
        ["Election event e1 has no bulletin board", "dashboard.ballotBoxes.failure.noBoard"],
        [
            'a ballot has the unknown voting channel "PAPER"',
            "dashboard.ballotBoxes.failure.unknownChannel",
        ],
        [
            "Voting can't start again: with the Ballot Box Seal Policy set to Seal at close, …",
            "publish.sealRefusals.startAgain",
        ],
        [
            "This election event has sealed ballot boxes and cannot be deleted. Archive it instead.",
            "sideMenu.menuActions.messages.notification.error.deleteSealedEvent",
        ],
        [
            "The weighted_voting_policy of election event e1 can't change after voting has opened",
            "electionEventScreen.field.ballotBoxSealPolicy.settingRefused",
        ],
        [
            "The contest_encryption_policy of election event e1 can't change after voting has opened",
            "electionEventScreen.field.ballotBoxSealPolicy.settingRefused",
        ],
        [
            "The ballot_box_seal_record_policy of election event e1 can't change after voting has opened",
            "electionEventScreen.field.ballotBoxSealPolicy.settingRefused",
        ],
        [
            "The bulletin board of election event e1 can't change after voting has opened",
            "electionEventScreen.field.ballotBoxSealPolicy.boardRefused",
        ],
        ["ballot_box_seal_closed_is_final", "publish.sealRefusals.closedIsFinal"],
    ])("translates %s", (text, key) => {
        expect(sealText(t, text)).toBe(`t:${key}`)
    })
    it("shows any other text as it is", () => {
        expect(sealText(t, "something else")).toBe("something else")
        expect(sealText(t, null)).toBe("")
    })
    it("finds the policy lock refusal anywhere in a save error", () => {
        const error = {
            graphQLErrors: [
                {
                    extensions: {
                        internal: {
                            error: {
                                message:
                                    "The ballot box seal policy of election event e can only change before voting opens",
                            },
                        },
                    },
                },
            ],
        }
        expect(sealErrorText(t, error)).toBe(
            "t:electionEventScreen.field.ballotBoxSealPolicy.refused"
        )
        expect(sealErrorText(t, new Error("database query error"))).toBeUndefined()
    })
})
