// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {getNumberFormatPolicyChoices} from "./numberFormatPolicy"

describe("getNumberFormatPolicyChoices", () => {
    it("labels each policy with how it writes the same sample", () => {
        expect(getNumberFormatPolicyChoices()).toEqual([
            {id: ENumberFormatPolicy.COMMA_PERIOD, name: "1,234,567.89"},
            {id: ENumberFormatPolicy.PERIOD_COMMA, name: "1.234.567,89"},
            {id: ENumberFormatPolicy.SPACE_COMMA, name: "1\u00a0234\u00a0567,89"},
            {id: ENumberFormatPolicy.SPACE_PERIOD, name: "1\u00a0234\u00a0567.89"},
            {id: ENumberFormatPolicy.APOSTROPHE_PERIOD, name: "1’234’567.89"},
        ])
    })
})
