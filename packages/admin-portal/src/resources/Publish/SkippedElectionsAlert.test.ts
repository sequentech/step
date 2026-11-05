// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {skippedElectionText} from "./SkippedElectionsAlert"

const t = (key: string, options?: Record<string, unknown>) => `${key} ${JSON.stringify(options)}`

it("explains a Post left closed by the seal policy", () => {
    expect(
        skippedElectionText(t, {
            election_id: "e1",
            election_name: "Madrid Post",
            reason: "ballot-box-seal-policy",
        })
    ).toBe('publish.skippedElections.ballotBoxSealPolicy {"name":"Madrid Post"}')
})

it("names the election by id without a name, and shows an unknown reason as is", () => {
    expect(skippedElectionText(t, {election_id: "e1", reason: "future-reason"})).toBe(
        'publish.skippedElections.other {"name":"e1","reason":"future-reason"}'
    )
})
