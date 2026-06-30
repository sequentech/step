// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import SelectElection from "./SelectElection"

const ELECTION_DATES = {
    first_started_at: "2025-10-29T14:00:00.000Z",
}

const legacyFormat = (input: string): string =>
    new Intl.DateTimeFormat("en-GB", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
    }).format(new Date(input))

describe("SelectElection", () => {
    it("uses the injected formatDateTime when provided", () => {
        const markup = renderToStaticMarkup(
            <SelectElection
                isActive
                isOpen
                title="Executive Board"
                hasVoted={false}
                electionDates={ELECTION_DATES}
                isStarted
                formatDateTime={() => "CUSTOM_DATE"}
            />
        )

        expect(markup).toContain("CUSTOM_DATE")
        expect(markup).not.toContain(legacyFormat(ELECTION_DATES.first_started_at))
    })

    it("falls back to the legacy GB format when formatDateTime is absent", () => {
        const markup = renderToStaticMarkup(
            <SelectElection
                isActive
                isOpen
                title="Executive Board"
                hasVoted={false}
                electionDates={ELECTION_DATES}
                isStarted
            />
        )

        expect(markup).toContain(legacyFormat(ELECTION_DATES.first_started_at))
        expect(markup).not.toContain("CUSTOM_DATE")
    })
})
