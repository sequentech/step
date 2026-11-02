// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {mismatchesOf} from "./EnrollmentFinish"

describe("mismatchesOf", () => {
    const mismatchedFields = [{name: "Middle name", value: null}]

    it("lists the fields that didn't match", () => {
        const outcome = {reason: "INSUFFICIENT_INFORMATION", mismatchedFields}
        expect(mismatchesOf("registration-manual-finish.ftl", outcome)).toEqual(mismatchedFields)
        expect(mismatchesOf("registration-rejected-finish.ftl", outcome)).toEqual(mismatchedFields)
    })

    it("lists none for a review without a voter in the registry", () => {
        const outcome = {reason: "NO_VOTER", mismatchedFields}
        expect(mismatchesOf("registration-manual-finish.ftl", outcome)).toEqual([])
        expect(mismatchesOf("registration-rejected-finish.ftl", outcome)).toEqual(mismatchedFields)
    })

    it("lists none without an outcome", () => {
        expect(mismatchesOf("registration-finish.ftl", undefined)).toEqual([])
    })
})
