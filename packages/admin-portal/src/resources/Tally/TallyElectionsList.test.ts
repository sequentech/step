/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {TallyElectionsList} from "./TallyElectionsList"

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

const election = (id: string, votingStatus = "CLOSED") =>
    ({
        id,
        keys_ceremony_id: "keys",
        presentation: {name: id},
        status: {voting_status: votingStatus},
    }) as unknown as Sequent_Backend_Election

const checkbox = (name: string) => screen.getByLabelText(name) as HTMLInputElement

const renderList = (elections: Sequent_Backend_Election[], update: jest.Mock) =>
    React.createElement(TallyElectionsList, {
        electionEventId: "event",
        elections,
        update,
        keysCeremonyId: "keys",
    })

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
