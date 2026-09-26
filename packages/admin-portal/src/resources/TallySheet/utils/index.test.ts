// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {sortFunction, translateSharedValidationError} from "./index"

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../../ui-core/src/utils/typechecks")
)

describe("tally sheet name ordering", () => {
    it("orders named areas alphabetically", () => {
        expect([{name: "West"}, {name: "East"}, {name: "North"}].sort(sortFunction)).toEqual([
            {name: "East"},
            {name: "North"},
            {name: "West"},
        ])
    })

    it.each([
        [{}, {name: "West"}],
        [{name: "West"}, {}],
        [{name: null}, {name: null}],
    ])("leaves an unnamed pair in its current order (%j, %j)", (first, second) => {
        expect(sortFunction(first, second)).toBe(0)
    })
})

describe("shared tally validation error translations", () => {
    it.each([
        ["invalid_total_valid_votes", "totalValidDoesNotMatch"],
        ["total_votes_exceeds_census", "censusTooSmall"],
        ["invalid_total_invalid", "totalInvalidDoesNotMatch"],
        ["invalid_total_votes", "totalVotesDoesNotMatch"],
        ["unknown_counting_algorithm", "unknownCountingAlgorithm"],
        ["new_backend_rule", "new_backend_rule"],
    ])("renders %s with its translation key, interpolation values and fallback", (code, key) => {
        const translate = jest.fn().mockReturnValue("Translated validation message")
        const result = translateSharedValidationError(
            translate as unknown as Parameters<typeof translateSharedValidationError>[0],
            {code, message: "Backend validation message", params: {total: "12", census: "10"}}
        )
        expect(translate).toHaveBeenCalledWith(`tallysheet.inputError.${key}`, {
            total: "12",
            census: "10",
            defaultValue: "Backend validation message",
        })
        expect(result).toBe("Translated validation message")
    })

    it.each([undefined, null])("accepts omitted interpolation values (%s)", (params) => {
        const translate = jest.fn().mockReturnValue("Backend validation message")
        expect(
            translateSharedValidationError(
                translate as unknown as Parameters<typeof translateSharedValidationError>[0],
                {code: "new_backend_rule", message: "Backend validation message", params}
            )
        ).toBe("Backend validation message")
        expect(translate).toHaveBeenCalledWith("tallysheet.inputError.new_backend_rule", {
            defaultValue: "Backend validation message",
        })
    })
})
