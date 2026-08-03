// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {RealmPasswordPolicy} from "@/queries/RealmPasswordPolicy"
import {validatePasswordPolicy} from "./passwordPolicyValidation"

const validPolicy: RealmPasswordPolicy = {
    configured: true,
    minimum_length: 12,
    maximum_length: 72,
    include_uppercase: true,
    include_lowercase: true,
    include_digits: true,
    include_special_characters: true,
}

describe("validatePasswordPolicy", () => {
    it("accepts a valid policy", () => {
        expect(validatePasswordPolicy(validPolicy)).toBeUndefined()
    })

    it("requires at least one character class", () => {
        expect(
            validatePasswordPolicy({
                ...validPolicy,
                include_uppercase: false,
                include_lowercase: false,
                include_digits: false,
                include_special_characters: false,
            })
        ).toBe("characterClassRequired")
    })
})
