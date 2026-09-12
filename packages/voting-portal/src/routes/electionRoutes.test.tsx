// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import theme from "../../../ui-essentials/src/services/theme"
import {electionRoutes} from "./electionRoutes"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    stringToHtml: (value: string) => value,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        theme: jest.requireActual("../../../ui-essentials/src/services/theme").default,
        Loader: () => <div>Loading</div>,
        PageLimit: ({children}: any) => <div>{children}</div>,
        InfoDataBox: ({children}: any) => <div>{children}</div>,
        BreadCrumbSteps: () => null,
        Icon: () => null,
        IconButton: () => null,
        Dialog: () => null,
        ExpandableText: () => null,
    }),
    {virtual: true}
)
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {DISABLE_AUTH: false}}),
}))
jest.mock("../store/hooks", () => ({
    useAppDispatch: () => jest.fn(),
    useAppSelector: () => undefined,
}))
const mockVoterContext = jest.fn()
jest.mock("./StartScreen", () => ({__esModule: true, default: () => <div>Start voting</div>}))
jest.mock("./VotingScreen", () => ({__esModule: true, default: () => null, action: jest.fn()}))
jest.mock("./ReviewScreen", () => ({__esModule: true, default: () => null, action: jest.fn()}))
jest.mock("./ConfirmationScreen", () => ({__esModule: true, default: () => null}))
jest.mock("./AuditScreen", () => ({__esModule: true, default: () => null}))
const mockQueries: {name: string; variables: any}[] = []
jest.mock("../hooks/useVoterContext", () => ({
    useVoterContext: (...args: unknown[]) => mockVoterContext(...args),
}))
jest.mock("@apollo/client/react", () => ({
    useQuery: (query: any, options: any) => {
        const name = query.definitions.find(
            (definition: any) => definition.kind === "OperationDefinition"
        )?.name.value
        mockQueries.push({name, variables: options.variables})
        const data =
            name === "GetElectionEvent"
                ? {sequent_backend_election_event: [{id: "event", presentation: {}}]}
                : name === "GetElections"
                  ? {
                        sequent_backend_election: [
                            {id: "election", voting_channels: {telephone: true}},
                        ],
                    }
                  : name === "GetCastVote"
                    ? {
                          sequent_backend_cast_vote: [
                              {ballot_id: "a".repeat(64), content: "existing encrypted ballot"},
                          ],
                      }
                    : undefined
        return {data, loading: false, refetch: jest.fn()}
    },
}))

function renderElectionRoute(suffix: string) {
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId",
                errorElement: <div role="alert">publication unavailable</div>,
                children: [electionRoutes],
            },
        ],
        {initialEntries: ["/tenant/tenant/event/event/election/election/" + suffix]}
    )
    return render(
        <ThemeProvider theme={theme}>
            <RouterProvider router={router} />
        </ThemeProvider>
    )
}

beforeEach(() => {
    mockQueries.length = 0
    mockVoterContext.mockReset()
    mockVoterContext.mockImplementation(() => {
        throw new Error("No active ballot publication")
    })
})

test.each(["a".repeat(64), "abcd"])(
    "receipt %s remains readable without an active ballot publication",
    async (ballotId) => {
        const view = renderElectionRoute("ballot-locator/" + ballotId)
        expect(await screen.findByText("ballotLocator.found")).toBeVisible()
        expect(screen.getByText("existing encrypted ballot")).toBeVisible()
        expect(mockVoterContext).not.toHaveBeenCalled()
        expect(mockQueries.find(({name}) => name === "GetCastVote")?.variables).toEqual({
            tenantId: "tenant",
            electionEventId: "event",
            electionId: "election",
            ballotIdPattern: ballotId.length === 4 ? ballotId + "%" : ballotId,
        })
        view.unmount()
    }
)

test("the receipt search form also works without a publication", async () => {
    const view = renderElectionRoute("ballot-locator")
    expect(await screen.findByRole("button", {name: "ballotLocator.locate"})).toBeVisible()
    expect(mockVoterContext).not.toHaveBeenCalled()
    view.unmount()
})

test.each(["start", "vote", "review", "confirmation", "audit"])(
    "%s still waits for an authorized published ballot",
    async (path) => {
        mockVoterContext.mockReturnValue({loading: true})
        const view = renderElectionRoute(path)
        expect(await screen.findByRole("progressbar")).toBeVisible()
        expect(mockVoterContext).toHaveBeenCalledWith("election")
        expect(screen.queryByText("Start voting")).toBeNull()
        view.unmount()
    }
)
