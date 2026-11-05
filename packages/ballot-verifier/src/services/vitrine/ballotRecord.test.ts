// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What a ballot's record in the public ballot locator holds (the voting
// portal's services/vitrine/ballotTracker.test.ts).

import {classifyRecord, ERecordEncoding, ERecordKind} from "./ballotRecord"

const FULL = "FTBEMHRX0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789AB"

const hashableBallot = (extra: Record<string, unknown> = {}) => ({
    version: 2,
    issue_date: "30/10/2026",
    contests: ["EQAAAGNvdW5jaWwtY29udGVzdC0x"],
    config: "council-style",
    ballot_style_hash: "1dce3614efd2cc5d0ddb8a1c47daf79b",
    voter_signing_pk: null,
    voter_ballot_signature: null,
    ...extra,
})

const auditableBallot = (extra: Record<string, unknown> = {}) => ({
    version: 2,
    issue_date: "30/10/2026",
    config: {id: "council-style", contests: []},
    contests: ["EQAAAGNvdW5jaWwtY29udGVzdC0x"],
    ballot_hash: FULL,
    voter_signing_pk: null,
    voter_ballot_signature: null,
    ...extra,
})

describe("what a record holds", () => {
    it("takes a ballot whose config is a string, as the ballot box keeps it, for a cast one", () => {
        expect(classifyRecord(JSON.stringify(hashableBallot()))).toEqual({kind: ERecordKind.CAST})
        expect(classifyRecord(JSON.stringify(hashableBallot({contests: "EQAAAGNv"})))).toEqual({
            kind: ERecordKind.CAST,
        })
    })

    it("takes an auditable ballot, whose config is an object, for an audited one", () => {
        const ballot = auditableBallot()
        expect(classifyRecord(JSON.stringify(ballot))).toEqual({
            kind: ERecordKind.AUDITED,
            encoding: ERecordEncoding.SINGLE_CONTEST,
            ballot,
        })
    })

    it("tells a multi-contest audited ballot by its contests, one string", () => {
        const ballot = auditableBallot({contests: "EQAAAGNvdW5jaWw"})
        expect(classifyRecord(JSON.stringify(ballot))).toEqual({
            kind: ERecordKind.AUDITED,
            encoding: ERecordEncoding.MULTI_CONTEST,
            ballot,
        })
    })

    it("prefers the config's type to the other fields a record carries", () => {
        const cast = hashableBallot({ballot_hash: FULL})
        expect(classifyRecord(JSON.stringify(cast)).kind).toBe(ERecordKind.CAST)
    })

    it.each([
        ["no content", undefined],
        ["null content", null],
        ["empty content", ""],
        ["text", "a ballot, as plain text"],
        ["truncated JSON", '{"config": "council-style", "contests": ['],
        ["a JSON list", "[1, 2, 3]"],
        ["a JSON number", "42"],
        ["JSON null", "null"],
        ["a JSON string", '"30ffd314"'],
        ["an empty object", "{}"],
    ])("cannot read %s", (_name, content) => {
        expect(classifyRecord(content)).toEqual({kind: ERecordKind.UNREADABLE})
    })

    it.each([
        ["a config that is a number", hashableBallot({config: 7})],
        ["a config that is a list", auditableBallot({config: []})],
        ["a config that is null", auditableBallot({config: null})],
        ["a cast ballot without contests", hashableBallot({contests: undefined})],
        ["a cast ballot whose contests are a number", hashableBallot({contests: 3})],
        ["an audited ballot without a hash", auditableBallot({ballot_hash: undefined})],
        ["an audited ballot with an empty hash", auditableBallot({ballot_hash: ""})],
        ["an audited ballot with a hash that is a number", auditableBallot({ballot_hash: 12})],
        ["an audited ballot without contests", auditableBallot({contests: undefined})],
        ["an audited ballot whose contests are a number", auditableBallot({contests: 3})],
        ["an audited ballot whose contests are not text", auditableBallot({contests: [1, 2]})],
    ])("cannot read a record with %s", (_name, record) => {
        expect(classifyRecord(JSON.stringify(record))).toEqual({kind: ERecordKind.UNREADABLE})
    })
})
