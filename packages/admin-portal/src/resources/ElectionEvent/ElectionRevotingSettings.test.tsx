/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {MockedProvider} from "@apollo/client/testing"
import {GraphQLError} from "graphql"
import {
    DISABLE_ELECTION_REVOTING,
    ElectionRevotingSettings,
    GET_ELECTION_REVOTING,
} from "./ElectionRevotingSettings"

jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Dialog: ({
            open,
            ok,
            cancel,
            handleClose,
            children,
        }: {
            open: boolean
            ok: string
            cancel: string
            handleClose: (confirmed: boolean) => void
            children: React.ReactNode
        }) => {
            const {createElement} = require("react")
            return open
                ? createElement(
                      "div",
                      {role: "dialog"},
                      children,
                      createElement("button", {onClick: () => handleClose(true)}, ok),
                      createElement("button", {onClick: () => handleClose(false)}, cancel)
                  )
                : null
        },
    }),
    {virtual: true}
)
const mockNotify = jest.fn()
jest.mock("react-admin", () => ({
    useNotify: () => mockNotify,
    useRecordContext: () => ({id: "event"}),
}))
jest.mock("react-i18next", () => {
    const en = jest.requireActual("@/translations/en").default.translations
    const t = (key: string, values: Record<string, unknown> = {}) => {
        const text = key.split(".").reduce((node: any, part: string) => node?.[part], en)
        return typeof text === "string"
            ? text.replace(/{{(\w+)}}/g, (_: string, name: string) => String(values[name]))
            : key
    }
    return {useTranslation: () => ({t, i18n: {language: "en"}})}
})
jest.mock("@/hooks/useAliasRenderer", () => ({
    useAliasRenderer: () => (presentation: any) => presentation?.i18n?.en?.name ?? "-",
}))
let mockAdministrator = true
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        hasRole: (role: string) => mockAdministrator && role === "admin-user",
    }),
}))

const election = (id: string, name: string | null, num_allowed_revotes: number | null) => ({
    id,
    presentation: {i18n: name ? {en: {name}} : {}},
    num_allowed_revotes,
})
const read = (council: number | null = 3) => ({
    request: {query: GET_ELECTION_REVOTING, variables: {electionEventId: "event"}},
    result: {
        data: {
            sequent_backend_election: [
                election("mayor", "Mayor", 0),
                election("council", "Council", council),
                election("referendum", "Referendum", 1),
                election("survey", "Survey", null),
            ],
        },
    },
})
const disableCouncil = {
    request: {
        query: DISABLE_ELECTION_REVOTING,
        variables: {electionEventId: "event", electionId: "council"},
    },
}
const renderWith = (mocks: ReadonlyArray<unknown>) =>
    render(
        <MockedProvider mocks={mocks as any} addTypename={false}>
            <ElectionRevotingSettings />
        </MockedProvider>
    )

beforeEach(() => {
    mockAdministrator = true
    mockNotify.mockReset()
})

it("shows each election's re-voting and offers to disable it only where voters may vote again", async () => {
    renderWith([read()])
    expect(await screen.findByText("Allowed, unlimited votes")).toBeInTheDocument()
    expect(screen.getByText("Allowed, up to 3 votes")).toBeInTheDocument()
    expect(screen.getAllByText("Disabled, one vote")).toHaveLength(2)
    expect(screen.getByRole("button", {name: "Disable re-voting for Mayor"})).toBeEnabled()
    expect(screen.getByRole("button", {name: "Disable re-voting for Council"})).toBeEnabled()
    for (const name of ["Referendum", "Survey"])
        expect(
            screen.queryByRole("button", {name: `Disable re-voting for ${name}`})
        ).not.toBeInTheDocument()
})

it("disables re-voting after a confirmation and says the board approves the change", async () => {
    renderWith([
        read(),
        {
            ...disableCouncil,
            result: {
                data: {
                    disable_election_revoting: {election_id: "council", num_allowed_revotes: 1},
                },
            },
        },
        read(1),
    ])
    fireEvent.click(await screen.findByRole("button", {name: "Disable re-voting for Council"}))
    expect(screen.getByRole("dialog")).toHaveTextContent(
        "Each voter of Council will be able to cast one vote. Re-voting can't be enabled again."
    )
    fireEvent.click(screen.getByRole("button", {name: "Disable re-voting"}))
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith(
            "Re-voting disabled for Council. The election board must approve the updated configuration before voting starts.",
            {type: "success"}
        )
    )
    await waitFor(() =>
        expect(
            screen.queryByRole("button", {name: "Disable re-voting for Council"})
        ).not.toBeInTheDocument()
    )
    expect(screen.getAllByText("Disabled, one vote")).toHaveLength(3)
})

it("shows the server's refusal and keeps the election as it was", async () => {
    const refusal =
        "Voting has started on a channel of this election event; re-voting can no longer be disabled."
    renderWith([read(), {...disableCouncil, result: {errors: [new GraphQLError(refusal)]}}, read()])
    fireEvent.click(await screen.findByRole("button", {name: "Disable re-voting for Council"}))
    fireEvent.click(screen.getByRole("button", {name: "Disable re-voting"}))
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith(
            `Re-voting was not disabled for Council: ${refusal}`,
            {type: "error"}
        )
    )
    expect(mockNotify).not.toHaveBeenCalledWith(expect.anything(), {type: "success"})
    expect(await screen.findByText("Allowed, up to 3 votes")).toBeInTheDocument()
})

it("changes nothing when the administrator cancels", async () => {
    renderWith([read()])
    fireEvent.click(await screen.findByRole("button", {name: "Disable re-voting for Mayor"}))
    fireEvent.click(screen.getByRole("button", {name: "Cancel"}))
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    expect(mockNotify).not.toHaveBeenCalled()
    expect(screen.getByText("Allowed, unlimited votes")).toBeInTheDocument()
})

it("names an election by its id when it has no name", async () => {
    renderWith([
        {
            ...read(),
            result: {data: {sequent_backend_election: [election("unnamed", null, 0)]}},
        },
    ])
    expect(await screen.findByRole("button", {name: "Disable re-voting for unnamed"})).toBeEnabled()
})

it("offers nothing to an account without the administrator permission", () => {
    mockAdministrator = false
    const {container} = renderWith([])
    expect(container).toBeEmptyDOMElement()
})
