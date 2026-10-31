// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The checks the ballot tracker runs on an audited ballot, the ones the
// standalone ballot verifier runs: the ballot's hash is the Tracking ID, its
// ciphertext reproduces from the plaintext and randomness it carries, and only
// then does it decode into the voter's selections.

import type {
    IAuditableMultiBallot,
    IAuditableSingleBallot,
    IDecodedVoteContest,
} from "@sequentech/ui-core"
import {EAuditOutcome, auditBallot, TAuditedRecord, TAuditService} from "./ballotAudit"
import {ERecordEncoding, ERecordKind} from "./ballotTracker"

const TRACKING_ID = "9020e3849a0ad24eb0f322e1cd400b60618ecd1830dda699448a968cfc9ca5f7"
const OTHER_ID = "FTBEMHRX0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789AB"

const SELECTIONS: IDecodedVoteContest[] = [
    {
        contest_id: "council-contest-1",
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: [{id: "council-ana-ferrer", selected: 0}],
    },
]

const SINGLE_BALLOT = {ballot_hash: TRACKING_ID} as IAuditableSingleBallot
const MULTI_BALLOT = {ballot_hash: TRACKING_ID} as IAuditableMultiBallot

const SINGLE: TAuditedRecord = {
    kind: ERecordKind.AUDITED,
    encoding: ERecordEncoding.SINGLE_CONTEST,
    ballot: SINGLE_BALLOT,
}
const MULTI: TAuditedRecord = {
    kind: ERecordKind.AUDITED,
    encoding: ERecordEncoding.MULTI_CONTEST,
    ballot: MULTI_BALLOT,
}

const failing = () => {
    throw new Error("The ballot could not be read")
}

const serviceOf = (patch: Partial<TAuditService> = {}) => ({
    hashBallot: jest.fn(() => TRACKING_ID),
    hashMultiBallot: jest.fn(() => TRACKING_ID),
    verifyAuditableBallotCiphertext: jest.fn(() => true),
    verifyAuditableMultiBallotCiphertext: jest.fn(() => true),
    decodeAuditableBallot: jest.fn(() => SELECTIONS),
    decodeAuditableMultiBallot: jest.fn(() => SELECTIONS),
    ...patch,
})

describe("an audited ballot that checks out", () => {
    it("decodes into the selections, after its hash and ciphertext were checked", () => {
        const service = serviceOf()
        expect(auditBallot(service, SINGLE, TRACKING_ID)).toEqual({
            outcome: EAuditOutcome.VERIFIED,
            selections: SELECTIONS,
        })
        expect(service.hashBallot).toHaveBeenCalledWith(SINGLE_BALLOT)
        expect(service.verifyAuditableBallotCiphertext).toHaveBeenCalledWith(SINGLE_BALLOT)
        expect(service.decodeAuditableBallot).toHaveBeenCalledWith(SINGLE_BALLOT)
        expect(service.hashMultiBallot).not.toHaveBeenCalled()
        expect(service.verifyAuditableMultiBallotCiphertext).not.toHaveBeenCalled()
        expect(service.decodeAuditableMultiBallot).not.toHaveBeenCalled()
    })

    it("is read with the multi-contest checks when it holds one ciphertext for all contests", () => {
        const service = serviceOf()
        expect(auditBallot(service, MULTI, TRACKING_ID)).toEqual({
            outcome: EAuditOutcome.VERIFIED,
            selections: SELECTIONS,
        })
        expect(service.hashMultiBallot).toHaveBeenCalledWith(MULTI_BALLOT)
        expect(service.verifyAuditableMultiBallotCiphertext).toHaveBeenCalledWith(MULTI_BALLOT)
        expect(service.decodeAuditableMultiBallot).toHaveBeenCalledWith(MULTI_BALLOT)
        expect(service.hashBallot).not.toHaveBeenCalled()
        expect(service.verifyAuditableBallotCiphertext).not.toHaveBeenCalled()
        expect(service.decodeAuditableBallot).not.toHaveBeenCalled()
    })

    it("matches its hash with the Tracking ID however it is spaced or cased", () => {
        const grouped = TRACKING_ID.toUpperCase().replace(/(.{4})/g, "$1 ")
        expect(auditBallot(serviceOf(), SINGLE, grouped).outcome).toBe(EAuditOutcome.VERIFIED)
    })
})

describe("an audited ballot that fails a check", () => {
    it("is a mismatch when its hash is not the Tracking ID, and nothing else is run", () => {
        const service = serviceOf({hashBallot: jest.fn(() => OTHER_ID)})
        const result = auditBallot(service, SINGLE, TRACKING_ID)
        expect(result).toEqual({outcome: EAuditOutcome.MISMATCH})
        expect(service.verifyAuditableBallotCiphertext).not.toHaveBeenCalled()
        expect(service.decodeAuditableBallot).not.toHaveBeenCalled()
    })

    it("is a mismatch when the multi-contest hash is not the Tracking ID", () => {
        const service = serviceOf({hashMultiBallot: jest.fn(() => OTHER_ID)})
        expect(auditBallot(service, MULTI, TRACKING_ID)).toEqual({outcome: EAuditOutcome.MISMATCH})
    })

    it.each([
        ["single", SINGLE, {verifyAuditableBallotCiphertext: jest.fn(() => false)}],
        ["multi-contest", MULTI, {verifyAuditableMultiBallotCiphertext: jest.fn(() => false)}],
    ])(
        "is a mismatch when the %s ciphertext does not reproduce, and shows no selections",
        (_name, record, patch) => {
            const service = serviceOf(patch)
            const result = auditBallot(service, record, TRACKING_ID)
            expect(result).toEqual({outcome: EAuditOutcome.MISMATCH})
            expect(result).not.toHaveProperty("selections")
            expect(service.decodeAuditableBallot).not.toHaveBeenCalled()
            expect(service.decodeAuditableMultiBallot).not.toHaveBeenCalled()
        }
    )
})

describe("an audited ballot that cannot be checked", () => {
    let logged: jest.SpyInstance

    beforeEach(() => {
        logged = jest.spyOn(console, "error").mockImplementation(() => undefined)
    })

    afterEach(() => {
        logged.mockRestore()
    })

    it.each([
        ["the hash can't be computed", {hashBallot: jest.fn(failing)}],
        ["the ciphertext check can't run", {verifyAuditableBallotCiphertext: jest.fn(failing)}],
        ["the ballot can't be decoded", {decodeAuditableBallot: jest.fn(failing)}],
        ["decoding finds nothing", {decodeAuditableBallot: jest.fn(() => null)}],
    ])("is not verifiable when %s", (_name, patch) => {
        const result = auditBallot(serviceOf(patch), SINGLE, TRACKING_ID)
        expect(result).toEqual({outcome: EAuditOutcome.NOT_VERIFIABLE})
        expect(result).not.toHaveProperty("selections")
    })

    it("tells what failed, without its selections", () => {
        auditBallot(serviceOf({hashBallot: jest.fn(failing)}), SINGLE, TRACKING_ID)
        expect(logged).toHaveBeenCalledTimes(1)
    })

    it("is not verifiable when the multi-contest ciphertext check fails to run", () => {
        const service = serviceOf({verifyAuditableMultiBallotCiphertext: jest.fn(failing)})
        expect(auditBallot(service, MULTI, TRACKING_ID)).toEqual({
            outcome: EAuditOutcome.NOT_VERIFIABLE,
        })
    })

    it("is not verifiable when there is no hash, or no Tracking ID, to compare", () => {
        const service = serviceOf({hashBallot: jest.fn(() => "")})
        expect(auditBallot(service, SINGLE, TRACKING_ID)).toEqual({
            outcome: EAuditOutcome.NOT_VERIFIABLE,
        })
        expect(auditBallot(serviceOf(), SINGLE, "")).toEqual({
            outcome: EAuditOutcome.NOT_VERIFIABLE,
        })
    })
})
