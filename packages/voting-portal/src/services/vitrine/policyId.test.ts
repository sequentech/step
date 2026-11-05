// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {policyIdDisplay} from "@vitrine/verify"
import {isPolicyIdTyped} from "./policyId"

const POLICY_ID = `sha256:${"0123456789abcdef".repeat(4)}`
const OTHER_ID = `sha256:${"fedcba9876543210".repeat(4)}`
const GROUPS = policyIdDisplay(POLICY_ID)

describe("isPolicyIdTyped", () => {
    it("takes the first four groups of the policy ID, or more, in any case, with or without hyphens", () => {
        const four = GROUPS.split("-").slice(0, 4).join("-")
        expect(isPolicyIdTyped(four, POLICY_ID)).toBe(true)
        expect(isPolicyIdTyped(four.toLowerCase(), POLICY_ID)).toBe(true)
        expect(isPolicyIdTyped(GROUPS.replace(/-/g, ""), POLICY_ID)).toBe(true)
        expect(isPolicyIdTyped(GROUPS, POLICY_ID)).toBe(true)
    })

    it("reads I and L as 1 and O as 0, as a Tracking ID is read", () => {
        const four = GROUPS.split("-").slice(0, 4).join("-")
        const misread = four.replace(/1/g, "l").replace(/0/g, "o")
        expect(misread).not.toBe(four)
        expect(isPolicyIdTyped(misread, POLICY_ID)).toBe(true)
    })

    it("is not a short Tracking ID, which has two groups", () => {
        const two = GROUPS.split("-").slice(0, 2).join("-")
        expect(isPolicyIdTyped(two, POLICY_ID)).toBe(false)
        expect(isPolicyIdTyped(GROUPS.split("-").slice(0, 3).join("-"), POLICY_ID)).toBe(false)
    })

    it("is not another policy's ID, nor text that is no ID", () => {
        expect(isPolicyIdTyped(policyIdDisplay(OTHER_ID), POLICY_ID)).toBe(false)
        expect(isPolicyIdTyped("K7QM:2XWD:0000:1111", POLICY_ID)).toBe(false)
        expect(isPolicyIdTyped("", POLICY_ID)).toBe(false)
    })

    it("is nothing without a policy", () => {
        expect(isPolicyIdTyped(GROUPS, undefined)).toBe(false)
    })
})
