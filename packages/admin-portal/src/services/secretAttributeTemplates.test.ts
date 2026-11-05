// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    getReferencedSecretAttributeNames,
    PIN_LINK_TEMPLATE_VARIABLES,
} from "./secretAttributeTemplates"

describe("getReferencedSecretAttributeNames", () => {
    const names = ["customerReference", "pin"]

    it("declares attributes used as template variables", () => {
        const contents = JSON.stringify({
            email: {body: "<p>Your reference is {{user.customerReference}}.</p>"},
            sms: {message: 'PIN {{lookup user.attributes "pin"}}'},
        })

        expect(getReferencedSecretAttributeNames(contents, names)).toEqual([
            "customerReference",
            "pin",
        ])
    })

    it("ignores names that only appear in prose", () => {
        const contents = JSON.stringify({
            email: {
                body: "<p>Keep your customerReference and pin private. Hi {{user.username}}.</p>",
            },
        })

        expect(getReferencedSecretAttributeNames(contents, names)).toEqual([])
    })

    it("does not match longer identifiers that contain the name", () => {
        const contents = "{{user.pincode}} {{user.customerReferenceOld}}"

        expect(getReferencedSecretAttributeNames(contents, names)).toEqual([])
    })

    it("declares the PIN link only when the template uses it", () => {
        const known = [...names, ...PIN_LINK_TEMPLATE_VARIABLES]

        expect(
            getReferencedSecretAttributeNames(
                "<a href='{{user.pin_link_url}}'>{{user.pin_link_token}}</a>",
                known
            )
        ).toEqual(["pin_link_token", "pin_link_url"])
        expect(
            getReferencedSecretAttributeNames(
                "Your PIN link is in this letter, pin_link_url",
                known
            )
        ).toEqual([])
    })
})
