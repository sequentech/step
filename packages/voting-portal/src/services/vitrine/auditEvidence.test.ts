// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What a voter whose audited ballot decodes to something else has to report:
// the evidence file, and who to report it to.

import type {IAuditableSingleBallot, IDecodedVoteContest} from "@sequentech/ui-core"
import type {Policy} from "@vitrine/verify"
import {
    auditEvidence,
    authorityContact,
    EContactSource,
    evidenceFileName,
    mailtoOf,
    EVIDENCE_MEDIA_TYPE,
} from "./auditEvidence"
import {ERecordEncoding, ERecordKind} from "./ballotTracker"
import type {TAuditedRecord} from "./ballotAudit"

const FULL = "FTBEMHRX0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789AB"
const GROUPED = "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB"

const BALLOT = {
    version: 2,
    issue_date: "30/10/2026",
    config: {id: "council-style", contests: []},
    contests: ["EQAAAGNvdW5jaWwtY29udGVzdC0x"],
    ballot_hash: FULL,
    voter_signing_pk: undefined,
    voter_ballot_signature: undefined,
} as unknown as IAuditableSingleBallot

const RECORD: TAuditedRecord = {
    kind: ERecordKind.AUDITED,
    encoding: ERecordEncoding.SINGLE_CONTEST,
    ballot: BALLOT,
}

const SELECTIONS: IDecodedVoteContest[] = [
    {
        contest_id: "mayor",
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: [{id: "ortega", selected: 0}],
    },
]

describe("the evidence file of an audited ballot that decodes to something else", () => {
    const file = () =>
        JSON.parse(auditEvidence({ballotId: FULL, record: RECORD, selections: SELECTIONS}))

    it("holds the ballot as the record has it, with its ciphertext and the secret the portal revealed", () => {
        expect(file().record).toEqual(JSON.parse(JSON.stringify(BALLOT)))
    })

    it("holds what the ballot decodes to", () => {
        expect(file().decoded).toEqual(SELECTIONS)
    })

    it("names the ballot by its full Tracking ID, in groups", () => {
        expect(file().trackingId).toBe(GROUPED)
    })

    it("says what the file is and where each part is, for whoever reads it", () => {
        expect(file().type).toBe("ballot-audit-evidence")
        expect(file().about).toMatch(/record/)
        expect(file().about).toMatch(/decoded/)
    })

    it("is JSON that people can read", () => {
        expect(auditEvidence({ballotId: FULL, record: RECORD, selections: SELECTIONS})).toContain(
            '\n  "type"'
        )
        expect(EVIDENCE_MEDIA_TYPE).toBe("application/json")
    })

    it("is named after the ballot's short Tracking ID", () => {
        expect(evidenceFileName(FULL)).toBe("ballot-audit-FTBEMHRX.json")
        expect(evidenceFileName(GROUPED)).toBe("ballot-audit-FTBEMHRX.json")
        expect(evidenceFileName("")).toBe("ballot-audit.json")
    })
})

describe("who an audit mismatch is reported to", () => {
    const policyWith = (contact: Policy["contact"]) => ({contact}) as unknown as Policy

    it("is the contact in the policy's contact lines, marked as the policy's", () => {
        expect(
            authorityContact(
                policyWith({
                    name: "City election office",
                    email: "help@vote.city.example",
                    phone: "+34 900 000 000",
                    url: "https://vote.city.example/help",
                })
            )
        ).toEqual({
            name: "City election office",
            email: "help@vote.city.example",
            phone: "+34 900 000 000",
            url: "https://vote.city.example/help",
            source: EContactSource.POLICY,
        })
    })

    it("keeps the lines the policy has, and nothing else", () => {
        expect(authorityContact(policyWith({email: "help@vote.city.example"}))).toEqual({
            email: "help@vote.city.example",
            source: EContactSource.POLICY,
        })
    })

    it("is the settings' contact, marked as the settings', when the policy has no contact line", () => {
        const settings = {name: "Support", phone: "+34 900 111 222"}
        expect(authorityContact(policyWith({}), settings)).toEqual({
            ...settings,
            source: EContactSource.SETTINGS,
        })
        expect(authorityContact(undefined, settings)).toEqual({
            ...settings,
            source: EContactSource.SETTINGS,
        })
    })

    it("prefers the policy's contact to the settings'", () => {
        expect(authorityContact(policyWith({name: "Board"}), {name: "Support"})).toEqual({
            name: "Board",
            source: EContactSource.POLICY,
        })
    })

    it("is no one when neither names an office, an address, a phone or a page", () => {
        expect(authorityContact(policyWith({}))).toBeUndefined()
        expect(authorityContact({} as unknown as Policy)).toBeUndefined()
        expect(authorityContact(undefined)).toBeUndefined()
        expect(authorityContact(policyWith({}), {})).toBeUndefined()
    })
})

describe("the email to the authority", () => {
    const CONTACT = {
        name: "City election office",
        email: "help@vote.city.example",
        phone: "+34 900 000 000",
        source: EContactSource.POLICY,
    }

    it("is a mailto to the contact's address, with the subject and the body as the voter's language has them", () => {
        const url = mailtoOf(
            CONTACT,
            "Ballot audit: not what I chose",
            "Tracking ID: FTBE\nI saved the evidence."
        )
        expect(url).toBe(
            "mailto:help@vote.city.example?subject=Ballot%20audit%3A%20not%20what%20I%20chose&body=Tracking%20ID%3A%20FTBE%0AI%20saved%20the%20evidence."
        )
    })

    it("is none without an address", () => {
        expect(mailtoOf({...CONTACT, email: undefined}, "s", "b")).toBeUndefined()
    })

    it.each(["not an address", "a@b.example?cc=x@y.example", "a@b.example,c@d.example", "a@b#c"])(
        "is none for %s, which would add recipients or fields",
        (email) => {
            expect(mailtoOf({...CONTACT, email}, "s", "b")).toBeUndefined()
        }
    )
})
