/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import {TallyFailureAlert} from "./TallyFailureAlert"
import type {Sequent_Backend_Tally_Session_Execution} from "@/gql/graphql"
import englishTranslation from "@/translations/en"
import {ETallyFailureReason} from "@/types/ceremonies"

const mockKnown = new Set(Object.keys(englishTranslation.translations.tally.failure.reason))

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: {defaultValue?: string}) => {
            const reason = key.replace("tally.failure.reason.", "")
            return key === reason || mockKnown.has(reason) ? key : (options?.defaultValue ?? key)
        },
    }),
}))

const execution = (status: unknown) =>
    ({id: "execution", status}) as Sequent_Backend_Tally_Session_Execution

it("shows the reason and the detail of a failed tally", () => {
    render(
        <TallyFailureAlert
            tallySessionExecution={execution({
                logs: [],
                trustees: [],
                elections_status: [],
                failure: {
                    reason: ETallyFailureReason.SEALED_BALLOTS_REFUSED,
                    detail: "area North: 2 sealed ballot(s) don't open",
                    failed_at: "2026-01-15T12:00:00Z",
                },
            })}
        />
    )
    expect(screen.getByText("tally.failure.title")).toBeTruthy()
    expect(screen.getByText("tally.failure.reason.SEALED_BALLOTS_REFUSED")).toBeTruthy()
    expect(
        screen.getByText("tally.failure.detail: area North: 2 sealed ballot(s) don't open")
    ).toBeTruthy()
})

it("falls back to a general text for a reason it has no text for", () => {
    render(
        <TallyFailureAlert
            tallySessionExecution={execution({
                failure: {reason: "A_LATER_REASON", detail: "", failed_at: "2026-01-15T12:00:00Z"},
            })}
        />
    )
    expect(screen.getByText("tally.failure.reason.unknown")).toBeTruthy()
    expect(document.querySelector(".tally-failure-detail")).toBeNull()
})

it("shows nothing for a tally that didn't fail", () => {
    const {container} = render(
        <TallyFailureAlert tallySessionExecution={execution({logs: [], trustees: []})} />
    )
    expect(container.firstChild).toBeNull()
    expect(render(<TallyFailureAlert />).container.firstChild).toBeNull()
})

it("has a text for every failure reason", () => {
    for (const reason of Object.values(ETallyFailureReason)) {
        expect(mockKnown.has(reason)).toBe(true)
    }
})
