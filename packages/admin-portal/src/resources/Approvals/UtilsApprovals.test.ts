// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {convertOneToSnakeCase, convertToCamelCase, convertToSnakeCase} from "./UtilsApprovals"

describe("approval profile field names", () => {
    it.each([
        ["firstName", "first_name"],
        ["lastName", "last_name"],
        ["email", "email"],
        ["birth_date", "birth_date"],
        ["", ""],
    ])("maps %s to the wire field %s", (input, expected) => {
        expect(convertOneToSnakeCase(input)).toBe(expected)
    })

    it.each([
        ["first_name", "firstName"],
        ["email_verified", "emailVerified"],
        ["voter_area_id", "voterAreaId"],
        ["custom-field", "custom-field"],
        ["", ""],
    ])("maps %s to the form field %s", (input, expected) => {
        expect(convertToCamelCase(input)).toBe(expected)
    })

    it("renames only top-level profile fields without modifying values or the input", () => {
        const attributes = Object.freeze({districtCode: ["NORTH", "02"]})
        const input = Object.freeze({
            firstName: "Carol",
            lastName: null,
            emailVerified: false,
            attempts: 0,
            attributes,
        })
        expect(convertToSnakeCase(input)).toEqual({
            first_name: "Carol",
            last_name: null,
            email_verified: false,
            attempts: 0,
            attributes: {districtCode: ["NORTH", "02"]},
        })
        expect(Object.keys(input)).toEqual([
            "firstName",
            "lastName",
            "emailVerified",
            "attempts",
            "attributes",
        ])
    })

    it("returns an empty profile for no attributes", () => {
        expect(convertToSnakeCase({})).toEqual({})
    })
})
