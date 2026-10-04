// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import type {DocumentNode} from "graphql"
import theme from "../../../ui-essentials/src/services/theme"
import BallotLocator from "./BallotLocator"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: {total?: string}) =>
            options?.total === undefined ? key : `${key}: ${options.total}`,
        i18n: {language: "en"},
    }),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual<typeof import("@sequentech/ui-core")>("@sequentech/ui-core"),
    stringToHtml: (value: string) => value,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        theme: jest.requireActual<typeof import("../../../ui-essentials/src/services/theme")>(
            "../../../ui-essentials/src/services/theme"
        ).default,
        PageLimit: ({children}: {children: React.ReactNode}) => <div>{children}</div>,
        InfoDataBox: ({children}: {children: React.ReactNode}) => <div>{children}</div>,
        BreadCrumbSteps: () => null,
        Icon: () => null,
        IconButton: () => null,
        Dialog: () => null,
        ExpandableText: () => null,
    }),
    {virtual: true}
)
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: jest
        .requireActual<typeof import("react")>("react")
        .createContext({globalSettings: {DISABLE_AUTH: false}}),
}))
jest.mock("../store/hooks", () => ({
    useAppDispatch: () => jest.fn(),
    useAppSelector: () => undefined,
}))

const mockEventPresentation: {current: Record<string, string>} = {current: {}}
const MOCK_CAST_VOTES_TOTAL = 1234567

jest.mock("@apollo/client/react", () => ({
    useQuery: (query: DocumentNode) => {
        const operation = query.definitions.find(
            (definition) => definition.kind === "OperationDefinition"
        )
        const name = operation?.kind === "OperationDefinition" ? operation.name?.value : undefined

        return {
            data:
                name === "GetElectionEvent"
                    ? {
                          sequent_backend_election_event: [
                              {id: "event", presentation: mockEventPresentation.current},
                          ],
                      }
                    : undefined,
            loading: false,
            refetch: async () => ({
                data: {list_cast_vote_messages: {list: [], total: MOCK_CAST_VOTES_TOTAL}},
            }),
        }
    },
}))

const renderCastVoteLogs = async () => {
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/ballot-locator",
                element: <BallotLocator />,
            },
        ],
        {initialEntries: ["/tenant/tenant/event/event/election/election/ballot-locator"]}
    )
    const view = render(
        <ThemeProvider theme={theme}>
            <RouterProvider router={router} />
        </ThemeProvider>
    )
    fireEvent.click(await screen.findByRole("tab", {name: "ballotLocator.tabs.logs"}))
    return view
}

test("the cast vote logs count ballots in the event's number format", async () => {
    mockEventPresentation.current = {
        show_cast_vote_logs: "show-logs-tab",
        number_format_policy: "period-comma",
    }
    const view = await renderCastVoteLogs()

    expect(
        await screen.findByRole("heading", {name: "ballotLocator.totalBallots: 1.234.567"})
    ).toBeVisible()
    expect(screen.getByText("1–5 of 1.234.567")).toBeVisible()
    view.unmount()
})

test("the cast vote logs group the ballot count with commas by default", async () => {
    mockEventPresentation.current = {show_cast_vote_logs: "show-logs-tab"}
    const view = await renderCastVoteLogs()

    expect(
        await screen.findByRole("heading", {name: "ballotLocator.totalBallots: 1,234,567"})
    ).toBeVisible()
    expect(screen.getByText("1–5 of 1,234,567")).toBeVisible()
    view.unmount()
})
