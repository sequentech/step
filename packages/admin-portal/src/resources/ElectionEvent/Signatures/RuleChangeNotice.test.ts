// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {appliesKey} from "./RuleChangeNotice"

jest.mock("@apollo/client", () => ({gql: (s: TemplateStringsArray) => s.join("")}))
jest.mock("react-i18next", () => ({}))

describe("appliesKey", () => {
    it.each([
        ["tightens", "scheduledOutcome.applies.tightens"],
        ["loosens", "scheduledOutcome.applies.loosens"],
        ["tightens-and-loosens", "scheduledOutcome.applies.tightensAndLoosens"],
    ])("words the server's %s", (applies, key) => {
        expect(appliesKey(applies)).toBe(key)
    })

    it("says nothing when the server says nothing changes", () => {
        expect(appliesKey(null)).toBeNull()
    })
})
