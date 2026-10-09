// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {effectiveSealRecordPolicy, isSealAtClose} from "./ballotBoxSealPolicy"

it("is true only for an event set to seal at close", () => {
    expect(isSealAtClose({ballot_box_seal_policy: "seal-at-close"})).toBe(true)
    expect(isSealAtClose(JSON.stringify({ballot_box_seal_policy: "seal-at-close"}))).toBe(true)
    expect(isSealAtClose({ballot_box_seal_policy: "do-not-seal"})).toBe(false)
    expect(isSealAtClose({})).toBe(false)
    expect(isSealAtClose(null)).toBe(false)
    expect(isSealAtClose("not json")).toBe(false)
})

it("reads the seal record policy as public only when set to public", () => {
    expect(effectiveSealRecordPolicy({ballot_box_seal_record_policy: "public"})).toBe("public")
    expect(effectiveSealRecordPolicy({ballot_box_seal_record_policy: "restricted"})).toBe(
        "restricted"
    )
    expect(effectiveSealRecordPolicy({ballot_box_seal_record_policy: null})).toBe("restricted")
    expect(effectiveSealRecordPolicy({ballot_box_seal_record_policy: "other"})).toBe("restricted")
    expect(effectiveSealRecordPolicy({})).toBe("restricted")
    expect(effectiveSealRecordPolicy(undefined)).toBe("restricted")
})
