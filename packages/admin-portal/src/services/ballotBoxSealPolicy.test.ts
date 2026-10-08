// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {isSealAtClose} from "./ballotBoxSealPolicy"

it("is true only for an event set to seal at close", () => {
    expect(isSealAtClose({ballot_box_seal_policy: "seal-at-close"})).toBe(true)
    expect(isSealAtClose(JSON.stringify({ballot_box_seal_policy: "seal-at-close"}))).toBe(true)
    expect(isSealAtClose({ballot_box_seal_policy: "do-not-seal"})).toBe(false)
    expect(isSealAtClose({})).toBe(false)
    expect(isSealAtClose(null)).toBe(false)
    expect(isSealAtClose("not json")).toBe(false)
})
