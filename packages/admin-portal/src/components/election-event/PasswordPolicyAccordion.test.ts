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

    it.each([
        [1, 1],
        [1, 256],
        [256, 256],
    ])("accepts the inclusive length boundaries %i to %i", (minimum_length, maximum_length) => {
        expect(
            validatePasswordPolicy({...validPolicy, minimum_length, maximum_length})
        ).toBeUndefined()
    })

    it.each(["minimum_length", "maximum_length"] as const)(
        "rejects invalid %s values before comparing the bounds",
        (field) => {
            for (const value of [0, -1, 257, 1.5, Number.NaN, Number.POSITIVE_INFINITY]) {
                expect(validatePasswordPolicy({...validPolicy, [field]: value})).toBe("lengthRange")
            }
        }
    )

    it("rejects a minimum above the maximum", () => {
        expect(validatePasswordPolicy({...validPolicy, minimum_length: 73})).toBe(
            "minimumExceedsMaximum"
        )
    })

    it.each([
        "include_uppercase",
        "include_lowercase",
        "include_digits",
        "include_special_characters",
    ] as const)("accepts %s as the only enabled character class", (field) => {
        expect(
            validatePasswordPolicy({
                ...validPolicy,
                include_uppercase: false,
                include_lowercase: false,
                include_digits: false,
                include_special_characters: false,
                [field]: true,
            })
        ).toBeUndefined()
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
