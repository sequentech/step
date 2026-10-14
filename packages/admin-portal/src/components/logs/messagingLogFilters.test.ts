// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessageAttemptState, EMessageChannel} from "@/types/messaging"
import {LOGS_TAB_ID, logsFilterSearch} from "./messagingLogFilters"

describe("logsFilterSearch", () => {
    it("opens the Logs tab with the channel's failed messages, filters shown", () => {
        const params = new URLSearchParams(logsFilterSearch(EMessageChannel.WHATSAPP))
        expect(params.get("tabId")).toBe(LOGS_TAB_ID)
        expect(JSON.parse(params.get("filter") ?? "{}")).toEqual({
            statement_kind: "SendCommunications",
            channel: "WHATSAPP",
            outcome: EMessageAttemptState.FAILED,
        })
        expect(JSON.parse(params.get("displayedFilters") ?? "{}")).toEqual({
            statement_kind: true,
            channel: true,
            outcome: true,
        })
    })
})

it("opens the failures of voters with no contact channel with channel none (Follow-up 2)", () => {
    const params = new URLSearchParams(logsFilterSearch(null))
    expect(JSON.parse(params.get("filter") ?? "{}")).toMatchObject({
        channel: "none",
        outcome: EMessageAttemptState.FAILED,
    })
})
