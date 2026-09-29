// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {conditionHolds} from "./when"
import {hidden, value} from "./selectors"

describe("conditionHolds", () => {
    it("holds without a condition", () => {
        expect(conditionHolds(undefined, {})).toBe(true)
    })

    it("holds while the earlier selector has one of the values", () => {
        const condition = {selector: "grain", in: ["hour"]}
        expect(conditionHolds(condition, {grain: value("hour")})).toBe(true)
        expect(conditionHolds(condition, {grain: value("day")})).toBe(false)
    })

    it("does not hold when the earlier selector is hidden or undeclared", () => {
        const condition = {selector: "grain", in: ["hour"]}
        expect(conditionHolds(condition, {grain: hidden()})).toBe(false)
        expect(conditionHolds(condition, {})).toBe(false)
    })
})
