// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    canEditVoterField,
    EMAIL_AND_OR_MOBILE_ATTR_NAME,
    MOBILE_PHONE_ATTR_NAME,
    VoterFieldAccess,
} from "./voterFieldAccess"

const EMAIL_TLF = ["email", MOBILE_PHONE_ATTR_NAME, EMAIL_AND_OR_MOBILE_ATTR_NAME]
const OTHERS = ["first_name", "last_name", "enabled", "area-id", "authorized-election-ids"]
const ALL = [...EMAIL_TLF, ...OTHERS]

const editable = (overrides: Partial<VoterFieldAccess>, fields: string[] = ALL): string[] => {
    const access: VoterFieldAccess = {
        createMode: false,
        electionEventId: "event",
        hasVoted: false,
        canEditVoters: false,
        canEditVotersEmailTlf: false,
        canEditVotersWhoVoted: false,
        ...overrides,
    }
    return fields.filter((field) => canEditVoterField(access, field))
}

describe("canEditVoterField", () => {
    it("allows every field when creating a voter or editing a tenant user", () => {
        expect(editable({createMode: true})).toEqual(ALL)
        expect(editable({electionEventId: undefined})).toEqual(ALL)
    })

    it("allows every field with voter-write for a voter who has not voted", () => {
        expect(editable({canEditVoters: true})).toEqual(ALL)
    })

    it("locks a voter who has voted unless voter-voted-edit is held", () => {
        expect(editable({canEditVoters: true, hasVoted: true})).toEqual([])
        expect(editable({canEditVotersEmailTlf: true, hasVoted: true})).toEqual([])
        const votedEdit = {canEditVotersWhoVoted: true, hasVoted: true}
        expect(editable({...votedEdit, canEditVoters: true})).toEqual(ALL)
    })

    it("limits voter-email-tlf-edit without voter-write to email and mobile", () => {
        expect(editable({canEditVotersEmailTlf: true})).toEqual(EMAIL_TLF)
    })

    it("keeps that limit when voter-voted-edit is also held", () => {
        const votedEdit = {canEditVotersWhoVoted: true, hasVoted: true}
        expect(editable({...votedEdit, canEditVotersEmailTlf: true})).toEqual(EMAIL_TLF)
    })

    it("grants no field with voter-voted-edit alone", () => {
        expect(editable({canEditVotersWhoVoted: true})).toEqual([])
    })

    it("matches the mobile attribute by exact name", () => {
        const lookalikes = ["mobile-number", "landline-mobile-number"]
        expect(editable({canEditVotersEmailTlf: true}, lookalikes)).toEqual([])
    })
})
