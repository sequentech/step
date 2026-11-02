// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {policyOptions} from "./policyOptions"

describe("policyOptions", () => {
    it("labels each value with the translation of its key", () => {
        const translate = jest.fn((key: string) => `translated ${key}`)

        expect(policyOptions(["warn", "refuse"], "field.policy.options", translate)).toEqual([
            {id: "warn", name: "translated field.policy.options.warn"},
            {id: "refuse", name: "translated field.policy.options.refuse"},
        ])
        expect(translate).toHaveBeenCalledTimes(2)
    })

    it("has no options without values", () => {
        const translate = jest.fn((key: string) => key)

        expect(policyOptions([], "field.policy.options", translate)).toEqual([])
        expect(translate).not.toHaveBeenCalled()
    })
})
