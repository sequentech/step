// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import type {DocumentNode} from "graphql"
import theme from "../../../ui-essentials/src/services/theme"
import BallotLocator from "./BallotLocator"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        // The key, then the values it is given, in order.
        t: (key: string, options?: Record<string, string>) =>
            options === undefined ? key : `${key}: ${Object.values(options).join("|")}`,
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
const castVotes = (total: number) => ({data: {list_cast_vote_messages: {list: [], total}}})
// The cast-vote list request; a test can hold its answer back.
const mockRefetch = jest.fn()

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
            refetch: (variables: unknown) => mockRefetch(variables),
        }
    },
}))

const LOOKUP_PATH = "/tenant/tenant/event/event/election/election/ballot-locator"

beforeEach(() => {
    mockRefetch.mockReset()
    mockRefetch.mockImplementation(async () => castVotes(MOCK_CAST_VOTES_TOTAL))
})

afterEach(() => {
    jest.restoreAllMocks()
})

const renderBallotLocator = () => {
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/ballot-locator/:ballotId?",
                element: <BallotLocator />,
            },
        ],
        {initialEntries: [LOOKUP_PATH]}
    )
    const view = render(
        <ThemeProvider theme={theme}>
            <RouterProvider router={router} />
        </ThemeProvider>
    )
    return {router, view}
}

const renderCastVoteLogs = async () => {
    const {view} = renderBallotLocator()
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
    expect(screen.getByText("ballotLocator.pagination.displayedRows: 1|5|1.234.567")).toBeVisible()
    expect(screen.getByText("ballotLocator.pagination.rowsPerPage")).toBeVisible()
    expect(screen.getByRole("button", {name: "ballotLocator.pagination.nextPage"})).toBeVisible()
    view.unmount()
})

test("the cast vote logs group the ballot count with commas by default", async () => {
    mockEventPresentation.current = {show_cast_vote_logs: "show-logs-tab"}
    const view = await renderCastVoteLogs()

    expect(
        await screen.findByRole("heading", {name: "ballotLocator.totalBallots: 1,234,567"})
    ).toBeVisible()
    expect(screen.getByText("ballotLocator.pagination.displayedRows: 1|5|1,234,567")).toBeVisible()
    view.unmount()
})

test("the ballot id filter is rejected when longer than a ballot id", async () => {
    mockEventPresentation.current = {show_cast_vote_logs: "show-logs-tab"}
    const view = await renderCastVoteLogs()
    const input = await screen.findByLabelText("a11y.ballotIdLabel")

    fireEvent.change(input, {target: {value: "ab".repeat(32)}})
    expect(screen.queryByText("ballotLocator.wrongFormatBallotId")).toBeNull()

    fireEvent.change(input, {target: {value: "ab".repeat(33)}})
    expect(await screen.findByText("ballotLocator.wrongFormatBallotId")).toBeVisible()
    view.unmount()
})

test("enter does not look up a ballot id the locate button rejects", async () => {
    mockEventPresentation.current = {}
    const {router, view} = renderBallotLocator()
    const input = await screen.findByLabelText("a11y.ballotIdLabel")

    fireEvent.change(input, {target: {value: "abc"}})
    fireEvent.keyDown(input, {key: "Enter"})
    await act(async () => {})
    expect(router.state.location.pathname).toBe(LOOKUP_PATH)
    expect(input).toHaveValue("abc")
    expect(screen.getByText("ballotLocator.wrongFormatBallotId")).toBeVisible()

    fireEvent.change(input, {target: {value: "abcd"}})
    fireEvent.keyDown(input, {key: "Enter"})
    await waitFor(() => expect(router.state.location.pathname).toBe(`${LOOKUP_PATH}/abcd`))
    view.unmount()
})

test("a deferred cast-vote request supersedes the one still in flight", async () => {
    mockEventPresentation.current = {show_cast_vote_logs: "show-logs-tab"}
    const clock = jest.spyOn(Date, "now").mockReturnValue(1_000_000)
    let answerFirst: (answer: ReturnType<typeof castVotes>) => void = () => {}
    mockRefetch.mockImplementationOnce(
        () =>
            new Promise((resolve) => {
                answerFirst = resolve
            })
    )
    mockRefetch.mockImplementation(async () => castVotes(222))
    const view = await renderCastVoteLogs()
    expect(mockRefetch).toHaveBeenCalledTimes(1)

    // 100 ms later the voter sorts by ballot id: too quick, so that request
    // waits a second before it is sent.
    clock.mockReturnValue(1_000_100)
    fireEvent.click(screen.getByText("ballotLocator.column.ballot_id"))
    expect(mockRefetch).toHaveBeenCalledTimes(1)

    // The first request answers while the second one waits: its rows are stale.
    await act(async () => answerFirst(castVotes(111)))
    expect(screen.getByRole("heading", {name: "ballotLocator.totalBallots: 0"})).toBeVisible()

    await waitFor(() => expect(mockRefetch).toHaveBeenCalledTimes(2), {timeout: 2000})
    expect(mockRefetch).toHaveBeenLastCalledWith(
        expect.objectContaining({orderBy: {ballot_id: "asc"}})
    )
    expect(
        await screen.findByRole("heading", {name: "ballotLocator.totalBallots: 222"})
    ).toBeVisible()
    view.unmount()
})

test("a malformed ballot id filter typed too quickly is never sent", async () => {
    mockEventPresentation.current = {show_cast_vote_logs: "show-logs-tab"}
    const clock = jest.spyOn(Date, "now").mockReturnValue(2_000_000)
    const view = await renderCastVoteLogs()
    expect(mockRefetch).toHaveBeenCalledTimes(1)
    const input = screen.getByLabelText("a11y.ballotIdLabel")

    // 100 ms later the voter has typed an odd number of characters.
    clock.mockReturnValue(2_000_100)
    fireEvent.keyDown(input, {key: "c"})
    fireEvent.change(input, {target: {value: "abc"}})
    expect(screen.getByText("ballotLocator.wrongFormatBallotId")).toBeVisible()

    // Longer than the second a deferred request waits.
    await act(() => new Promise((resolve) => setTimeout(resolve, 1500)))
    expect(mockRefetch).toHaveBeenCalledTimes(1)
    view.unmount()
})

test("a filter pasted without a key press is sent and supersedes a deferred request", async () => {
    mockEventPresentation.current = {show_cast_vote_logs: "show-logs-tab"}
    const clock = jest.spyOn(Date, "now").mockReturnValue(3_000_000)
    const view = await renderCastVoteLogs()
    expect(mockRefetch).toHaveBeenCalledTimes(1)
    const input = screen.getByLabelText("a11y.ballotIdLabel")

    // 100 ms later the voter types "ab": too quick, so it waits a second...
    clock.mockReturnValue(3_000_100)
    fireEvent.keyDown(input, {key: "b"})
    fireEvent.change(input, {target: {value: "ab"}})
    // ...and before it is sent pastes "abc" with the mouse, without a key press.
    clock.mockReturnValue(3_000_200)
    fireEvent.change(input, {target: {value: "abc"}})
    await act(() => new Promise((resolve) => setTimeout(resolve, 1500)))
    expect(mockRefetch).toHaveBeenCalledTimes(1)

    // A well-formed pasted filter is sent.
    clock.mockReturnValue(3_005_000)
    fireEvent.change(input, {target: {value: "abcd"}})
    await waitFor(() => expect(mockRefetch).toHaveBeenCalledTimes(2))
    expect(mockRefetch).toHaveBeenLastCalledWith(expect.objectContaining({ballotId: "abcd"}))
    view.unmount()
})
