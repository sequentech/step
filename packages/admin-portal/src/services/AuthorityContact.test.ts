// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    authorityContactPatch,
    contactErrors,
    EContactKind,
    EMPTY_CONTACT,
    IAuthorityContact,
    readAuthorityContact,
    sameContact,
} from "./AuthorityContact"

const contact = (values: Partial<IAuthorityContact>): IAuthorityContact => ({
    ...EMPTY_CONTACT,
    ...values,
})

const errorsOf = (kind: EContactKind, value: string) => contactErrors(contact({[kind]: value}))

describe("the contact's syntax (specs/policy.md, rule 19)", () => {
    it("accepts the example of the spec", () => {
        expect(
            contactErrors(
                contact({
                    name: "Electoral Board of Acme",
                    email: "elections@acme.example",
                    phone: "+34900000000",
                    url: "https://elections.acme.example/contact",
                })
            )
        ).toEqual([])
    })

    it("leaves empty kinds out of the errors, even when they only hold spaces", () => {
        expect(contactErrors(EMPTY_CONTACT)).toEqual([])
        expect(contactErrors(contact({name: "   ", email: " "}))).toEqual([])
    })

    it("limits a name to 128 bytes, not characters", () => {
        expect(errorsOf(EContactKind.NAME, "a".repeat(128))).toEqual([])
        expect(errorsOf(EContactKind.NAME, "a".repeat(129))).toEqual([EContactKind.NAME])
        expect(errorsOf(EContactKind.NAME, "ñ".repeat(64))).toEqual([])
        expect(errorsOf(EContactKind.NAME, "ñ".repeat(65))).toEqual([EContactKind.NAME])
    })

    it.each([
        ["two spaces", "Electoral  Board"],
        ["a line break", "Electoral\nBoard"],
        ["a tab", "Electoral\tBoard"],
        ["a zero-width space", "Bo\u200bard"],
        ["a right-to-left override", "Bo\u202eard"],
        ["a byte order mark", "Bo\ufeffard"],
        ["a C1 control", "Bo\u0085ard"],
    ])("refuses a name with %s", (_why, name) => {
        expect(errorsOf(EContactKind.NAME, name)).toEqual([EContactKind.NAME])
    })

    it("reads a name trimmed, as it is stored", () => {
        expect(errorsOf(EContactKind.NAME, "  Electoral Board ")).toEqual([])
    })

    it.each(["elections", "elections@acme", "@acme.example", ".a@acme.example", "a.@acme.example"])(
        "refuses the email %p",
        (email) => {
            expect(errorsOf(EContactKind.EMAIL, email)).toEqual([EContactKind.EMAIL])
        }
    )

    it.each([
        "a..b@acme.example",
        "a b@acme.example",
        "a@b@acme.example",
        "a@-acme.example",
        "a@acme-.example",
        "a@acme..example",
        "José@acme.example",
        "a@acmé.example",
        "a@acme.example.",
        `${"a".repeat(65)}@acme.example`,
        `a@${"a".repeat(64)}.example`,
    ])("refuses the email %p, which the policy can't carry", (email) => {
        expect(errorsOf(EContactKind.EMAIL, email)).toEqual([EContactKind.EMAIL])
    })

    it("accepts the characters the policy allows in an email", () => {
        expect(errorsOf(EContactKind.EMAIL, "a.b+c_d%e-f@sub.acme-x.example")).toEqual([])
    })

    it("limits an email to 254 bytes", () => {
        const label = "a".repeat(63)
        expect(
            errorsOf(EContactKind.EMAIL, `a@${[label, label, label, label].join(".")}.io`)
        ).toEqual([EContactKind.EMAIL])
    })

    it.each(["+34900000000", "+1234567", "+123456789012345"])("accepts the phone %p", (phone) => {
        expect(errorsOf(EContactKind.PHONE, phone)).toEqual([])
    })

    it.each([
        "34900000000",
        "+123456",
        "+1234567890123456",
        "+04900000000",
        "+34 900 000 000",
        "+34-900-000-000",
        "+3490000000a",
    ])("refuses the phone %p", (phone) => {
        expect(errorsOf(EContactKind.PHONE, phone)).toEqual([EContactKind.PHONE])
    })

    it("accepts only https addresses, not even http on localhost", () => {
        expect(errorsOf(EContactKind.URL, "https://elections.acme.example/contact")).toEqual([])
        for (const url of [
            "http://elections.acme.example",
            "http://localhost:8080/contact",
            "ftp://elections.acme.example",
            "https://user@elections.acme.example",
            "https://",
            "elections.acme.example",
            "https://elections.acme.example/a b",
        ]) {
            expect(errorsOf(EContactKind.URL, url)).toEqual([EContactKind.URL])
        }
    })

    it("names every kind that is wrong", () => {
        expect(
            contactErrors(
                contact({name: "ok", email: "nope", phone: "nope", url: "https://ok.example"})
            )
        ).toEqual([EContactKind.EMAIL, EContactKind.PHONE])
    })
})

describe("what is stored", () => {
    it("reads the contact of a presentation, or an object or string of one", () => {
        const stored = {authority_contact: {name: "Board", url: "https://acme.example"}}
        const expected = contact({name: "Board", url: "https://acme.example"})
        expect(readAuthorityContact(stored)).toEqual(expected)
        expect(readAuthorityContact(JSON.stringify(stored))).toEqual(expected)
    })

    it.each([undefined, null, "", "{", {}, {authority_contact: "Board"}, {authority_contact: [1]}])(
        "reads %p as no contact",
        (presentation) => {
            expect(readAuthorityContact(presentation)).toEqual(EMPTY_CONTACT)
        }
    )

    it("ignores a kind that is not text", () => {
        expect(readAuthorityContact({authority_contact: {name: 7, email: "a@b.io"}})).toEqual(
            contact({email: "a@b.io"})
        )
    })

    it("appends only the kinds that are set, trimmed, so that cleared kinds are dropped", () => {
        expect(
            authorityContactPatch(
                contact({name: " Board ", email: "", phone: "  ", url: "https://a.io"})
            )
        ).toEqual({authority_contact: {name: "Board", url: "https://a.io"}})
        expect(authorityContactPatch(EMPTY_CONTACT)).toEqual({authority_contact: {}})
    })

    it("compares contacts as they would be stored", () => {
        expect(sameContact(contact({name: "Board"}), contact({name: " Board "}))).toBe(true)
        expect(sameContact(contact({name: "Board"}), contact({name: "Boards"}))).toBe(false)
        expect(sameContact(contact({name: "  "}), EMPTY_CONTACT)).toBe(true)
    })
})
