/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {TallyElectionsList} from "./TallyElectionsList"
import {EBallotBoxesReadiness, type IBallotBoxesSummary} from "@/services/tallyEligibility"

type Row = {id: string; name: string; active: boolean}
type Column = {
    field: string
    renderCell?: (params: {value: boolean; row: Row}) => React.ReactNode
}

jest.mock("@mui/x-data-grid", () => ({
    DataGrid: ({rows, columns}: {rows: Row[]; columns: Column[]}) =>
        require("react").createElement(
            "div",
            null,
            rows.map((row) =>
                require("react").createElement(
                    "label",
                    {key: row.id},
                    row.name,
                    columns
                        .find((column) => column.field === "active")
                        ?.renderCell?.({value: row.active, row})
                )
            )
        ),
}))
jest.mock("@mui/material/Checkbox", () => ({
    __esModule: true,
    default: ({checked, onChange}: {checked: boolean; onChange: () => void}) =>
        require("react").createElement("input", {type: "checkbox", checked, onChange}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
const mockAliasRenderer = (presentation: {name?: string} | null) => presentation?.name ?? ""
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => mockAliasRenderer}))
jest.mock("@sequentech/ui-core", () =>
    require("../../../../ui-core/src/services/presentationOrder")
)
jest.mock("@/hooks/useZonedTime", () => ({
    useZonedTime: () => (value?: string | null) => value ?? "",
}))

const election = (id: string, votingStatus = "CLOSED") =>
    ({
        id,
        keys_ceremony_id: "keys",
        presentation: {name: id},
        status: {voting_status: votingStatus},
    }) as unknown as Sequent_Backend_Election

const checkbox = (name: string) => screen.getByLabelText(name) as HTMLInputElement

const renderList = (
    elections: Sequent_Backend_Election[],
    update: jest.Mock,
    ballotBoxes?: Record<string, IBallotBoxesSummary>
) =>
    React.createElement(TallyElectionsList, {
        electionEventId: "event",
        elections,
        update,
        keysCeremonyId: "keys",
        ballotBoxes,
    })

const boxes = (readiness: Record<string, EBallotBoxesReadiness>) =>
    Object.fromEntries(
        Object.entries(readiness).map(([id, value]) => [
            id,
            {readiness: value, total: 1, sealed: 1, published: 1},
        ])
    )

it("keeps the admin's selection when the polled elections change", () => {
    const update = jest.fn()
    const view = render(renderList([election("El1"), election("El2")], update))
    expect(update).toHaveBeenLastCalledWith(["El1", "El2"])

    act(() => {
        fireEvent.click(checkbox("El1"))
    })
    expect(update).toHaveBeenLastCalledWith(["El2"])

    view.rerender(renderList([election("El1", "OPEN"), election("El2", "OPEN")], update))

    expect(checkbox("El1").checked).toBe(false)
    expect(checkbox("El2").checked).toBe(true)
    expect(update).toHaveBeenLastCalledWith(["El2"])
})

it("selects elections that appear after the first load and drops removed ones", () => {
    const update = jest.fn()
    const view = render(renderList([election("El1"), election("El2")], update))
    act(() => {
        fireEvent.click(checkbox("El1"))
    })

    view.rerender(renderList([election("El1"), election("El3")], update))

    expect(checkbox("El1").checked).toBe(false)
    expect(checkbox("El3").checked).toBe(true)
    expect(screen.queryByLabelText("El2")).toBeNull()
    expect(update).toHaveBeenLastCalledWith(["El3"])
})

it("selects a sealed Post once its seals have loaded (VOTE-FREEZE)", () => {
    const update = jest.fn()
    const elections = [election("El1"), election("El2")]
    const loading = boxes({El1: EBallotBoxesReadiness.LOADING, El2: EBallotBoxesReadiness.LOADING})
    const view = render(renderList(elections, update, loading))
    expect(checkbox("El1").checked).toBe(false)
    expect(update).toHaveBeenLastCalledWith([])

    view.rerender(
        renderList(
            elections,
            update,
            boxes({El1: EBallotBoxesReadiness.READY, El2: EBallotBoxesReadiness.PUBLISHING})
        )
    )
    expect(checkbox("El1").checked).toBe(true)
    expect(checkbox("El2").checked).toBe(false)
    expect(update).toHaveBeenLastCalledWith(["El1"])
})

it("selects a Post that becomes ready, unless the admin deselected it while ready", () => {
    const update = jest.fn()
    const elections = [election("El1"), election("El2")]
    const view = render(
        renderList(
            elections,
            update,
            boxes({El1: EBallotBoxesReadiness.READY, El2: EBallotBoxesReadiness.PUBLISHING})
        )
    )
    act(() => {
        fireEvent.click(checkbox("El1"))
    })
    expect(update).toHaveBeenLastCalledWith([])

    view.rerender(
        renderList(
            elections,
            update,
            boxes({El1: EBallotBoxesReadiness.READY, El2: EBallotBoxesReadiness.READY})
        )
    )
    expect(checkbox("El1").checked).toBe(false)
    expect(checkbox("El2").checked).toBe(true)
    expect(update).toHaveBeenLastCalledWith(["El2"])
})

it("keeps a Post unselectable while its seals can't be read", () => {
    const update = jest.fn()
    render(renderList([election("El1")], update, boxes({El1: EBallotBoxesReadiness.UNAVAILABLE})))
    expect(checkbox("El1").checked).toBe(false)
    expect(update).toHaveBeenLastCalledWith([])
})

it("says under the list why each election can't be selected (VOTE-FREEZE)", () => {
    const update = jest.fn()
    render(
        renderList(
            [election("El1"), election("El2")],
            update,
            boxes({El1: EBallotBoxesReadiness.READY, El2: EBallotBoxesReadiness.FAILED})
        )
    )
    expect(screen.getByText("tally.ballotBoxes.blocked")).toBeTruthy()
    expect(screen.getByText("tally.ballotBoxes.help")).toBeTruthy()
})
