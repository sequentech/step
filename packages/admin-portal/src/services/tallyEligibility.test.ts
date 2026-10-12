// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {getTallyDisabledReason} from "./tallyEligibility"
import {EAllowTally, EVotingStatus} from "@sequentech/ui-core"
jest.mock("@sequentech/ui-core", () => require("../../../ui-core/src/types/CoreTypes"))
const election = (id: string, voting_status = EVotingStatus.CLOSED) => ({
    id,
    status: {
        is_published: true,
        voting_status,
        allow_tally: EAllowTally.REQUIRES_VOTING_PERIOD_END,
    },
})

it("only checks selected elections, regardless of the other election's open state", () => {
    const elections = [election("closed"), election("open", EVotingStatus.OPEN)]
    expect(getTallyDisabledReason(elections, ["closed"])).toBeUndefined()
    expect(getTallyDisabledReason(elections, ["closed", "open"])).toBe("endVoting")
})
it.each(["voting_status", "kiosk_voting_status", "early_voting_status", "telephone_voting_status"])(
    "blocks an active %s",
    (channel) => {
        for (const status of [EVotingStatus.OPEN, EVotingStatus.PAUSED]) {
            const selected = election("selected")
            expect(
                getTallyDisabledReason(
                    [{...selected, status: {...selected.status, [channel]: status}}],
                    ["selected"]
                )
            ).toBe("endVoting")
        }
    }
)
it("requires a selection and publication and honors explicit policy", () => {
    expect(getTallyDisabledReason([], [])).toBe("selectElection")
    expect(getTallyDisabledReason(undefined, ["selected"])).toBe("selectElection")
    expect(getTallyDisabledReason([], ["missing"])).toBe("publishElection")
    const selected = election("selected", EVotingStatus.OPEN)
    expect(
        getTallyDisabledReason(
            [{...selected, status: {...selected.status, allow_tally: EAllowTally.ALLOWED}}],
            ["selected"]
        )
    ).toBeUndefined()
    expect(
        getTallyDisabledReason(
            [{...selected, status: {...selected.status, allow_tally: EAllowTally.DISALLOWED}}],
            ["selected"]
        )
    ).toBe("tallyDisallowed")
})
