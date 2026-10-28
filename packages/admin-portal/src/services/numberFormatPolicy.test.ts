// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {
    getElectionEventNumberFormatPolicy,
    getNumberFormatPolicyChoices,
} from "./numberFormatPolicy"

// Stands in for i18next: writes the key and its values.
const t = (key: string, {policy, sample}: {policy: string; sample: string}): string =>
    `${key} ${policy} ${sample}`

describe("getNumberFormatPolicyChoices", () => {
    it("labels each policy with how it writes the same sample", () => {
        expect(getNumberFormatPolicyChoices({}, t)).toEqual([
            {id: ENumberFormatPolicy.COMMA_PERIOD, name: "1,234,567.89"},
            {id: ENumberFormatPolicy.PERIOD_COMMA, name: "1.234.567,89"},
            {id: ENumberFormatPolicy.SPACE_COMMA, name: "1\u00a0234\u00a0567,89"},
            {id: ENumberFormatPolicy.SPACE_PERIOD, name: "1\u00a0234\u00a0567.89"},
            {id: ENumberFormatPolicy.APOSTROPHE_PERIOD, name: "1’234’567.89"},
        ])
    })

    it("also lists a policy this version does not know, so the event can keep it", () => {
        const unknown = {
            id: "dot-dot",
            name: "electionEventScreen.field.numberFormatPolicy.unknownPolicy dot-dot 1,234,567.89",
        }
        expect(getNumberFormatPolicyChoices({number_format_policy: "dot-dot"}, t)).toEqual([
            ...getNumberFormatPolicyChoices({}, t),
            unknown,
        ])
        expect(
            getNumberFormatPolicyChoices(JSON.stringify({number_format_policy: "dot-dot"}), t)
        ).toContainEqual(unknown)
    })

    it("lists only the policies for a known policy, a missing one or one that is not text", () => {
        const policies = Object.values(ENumberFormatPolicy)
        for (const presentation of [
            {number_format_policy: ENumberFormatPolicy.SPACE_COMMA},
            {},
            {number_format_policy: null},
            {number_format_policy: ""},
            {number_format_policy: 5},
            undefined,
            "{not json",
        ]) {
            expect(getNumberFormatPolicyChoices(presentation, t).map(({id}) => id)).toEqual(
                policies
            )
        }
    })
})

describe("getElectionEventNumberFormatPolicy", () => {
    it("reads the policy from the event's presentation", () => {
        expect(
            getElectionEventNumberFormatPolicy({
                number_format_policy: ENumberFormatPolicy.SPACE_COMMA,
            })
        ).toBe(ENumberFormatPolicy.SPACE_COMMA)
    })

    it("reads a presentation stored as JSON text", () => {
        expect(
            getElectionEventNumberFormatPolicy(
                JSON.stringify({number_format_policy: ENumberFormatPolicy.PERIOD_COMMA})
            )
        ).toBe(ENumberFormatPolicy.PERIOD_COMMA)
    })

    it("uses comma grouping for events saved before the policy existed", () => {
        expect(getElectionEventNumberFormatPolicy({})).toBe(ENumberFormatPolicy.COMMA_PERIOD)
        expect(getElectionEventNumberFormatPolicy({number_format_policy: null})).toBe(
            ENumberFormatPolicy.COMMA_PERIOD
        )
        expect(getElectionEventNumberFormatPolicy(undefined)).toBe(ENumberFormatPolicy.COMMA_PERIOD)
        expect(getElectionEventNumberFormatPolicy(null)).toBe(ENumberFormatPolicy.COMMA_PERIOD)
    })

    it("uses comma grouping for a policy this version does not know or a broken presentation", () => {
        expect(getElectionEventNumberFormatPolicy({number_format_policy: "dot-dot"})).toBe(
            ENumberFormatPolicy.COMMA_PERIOD
        )
        expect(getElectionEventNumberFormatPolicy("{not json")).toBe(
            ENumberFormatPolicy.COMMA_PERIOD
        )
    })
})
