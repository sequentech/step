// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, waitFor, act} from "@testing-library/react"
import {ApolloClient, ApolloLink, InMemoryCache, Observable} from "@apollo/client"
import {ApolloProvider} from "@apollo/client/react"
import {Provider} from "react-redux"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import {ThemeProvider} from "@mui/material/styles"
import theme from "../../../ui-essentials/src/services/theme"
import {store, clearVoterSession} from "../store/store"
import {addCastVotes, CastVoteStatus} from "../store/castVotes/castVotesSlice"
import {setElection} from "../store/elections/electionsSlice"
import {GET_VOTER_STATUS} from "../queries/GetVoterStatus"
import ElectionSelectionScreen from "./ElectionSelectionScreen"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {updateBallotStyleAndSelection as loadPreview} from "./PreviewPublicationEvent"
import {ELECTION_WITH_INVALID} from "../fixtures/election"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
    Trans: () => null,
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    sortElectionList: (items: unknown[]) => items,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        theme: jest.requireActual("../../../ui-essentials/src/services/theme").default,
        PageLimit: ({children}: any) => <div>{children}</div>,
        Dialog: () => null,
        IconButton: () => null,
        SelectElection: ({isActive, hasVoted}: any) => (
            <button disabled={!isActive}>{hasVoted ? "Already voted" : "Vote"}</button>
        ),
    }),
    {virtual: true}
)
jest.mock("../providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isKiosk: () => false}),
}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {DISABLE_AUTH: false, QUERY_POLL_INTERVAL_MS: 20},
    }),
}))
jest.mock("../components/Stepper", () => ({__esModule: true, default: () => null}))
jest.mock("../hooks/useElectionClassName", () => ({useElectionClassName: () => [() => ""]}))

const election: any = {
    id: "election",
    tenant_id: "tenant",
    election_event_id: "event",
    name: "Election",
    presentation: {},
    description: null,
    annotations: null,
    created_at: null,
    labels: null,
    last_updated_at: null,
    is_consolidated_ballot_encoding: false,
    spoil_ballot_option: false,
    num_allowed_revotes: 1,
    status: {voting_status: "OPEN"},
    voting_channels: {online: true},
}
const cast: any = {
    id: "new-cast",
    tenant_id: "tenant",
    election_id: "election",
    election_event_id: "event",
}
const urls = {
    event_url: "https://objects/event",
    election_url: "https://objects/election",
    summary_url: "https://objects/summary",
    style_url: "https://objects/style",
}
const event = {
    id: "event",
    tenant_id: "tenant",
    presentation: {},
    status: {is_published: true, voting_status: "OPEN"},
    description: null,
}
let savedFetch: typeof fetch
beforeEach(() => {
    store.dispatch(clearVoterSession())
    savedFetch = global.fetch
    jest.spyOn(window, "scrollTo").mockImplementation(() => {})
    global.fetch = jest.fn(
        async (url) =>
            new Response(
                JSON.stringify(
                    String(url) === urls.event_url
                        ? event
                        : String(url) === urls.election_url
                          ? election
                          : {id: "style", area_presentation: {}, election_dates: {}}
                )
            )
    )
})
afterEach(() => {
    global.fetch = savedFetch
    jest.restoreAllMocks()
    store.dispatch(clearVoterSession())
})

const cases = [
    {
        name: "new cast accepted immediately",
        initialStatus: undefined,
        replies: [CastVoteStatus.VALID],
    },
    {
        name: "new cast accepted after validation",
        initialStatus: undefined,
        replies: [CastVoteStatus.IN_PROGRESS, CastVoteStatus.VALID],
    },
    {
        name: "new cast discarded after validation",
        initialStatus: undefined,
        replies: [CastVoteStatus.IN_PROGRESS, CastVoteStatus.DISCARDED],
    },
    {
        name: "new cast initially absent from a response",
        initialStatus: undefined,
        replies: [null, CastVoteStatus.IN_PROGRESS, CastVoteStatus.DISCARDED],
    },
    {
        name: "pending cast discarded",
        initialStatus: CastVoteStatus.IN_PROGRESS,
        replies: [CastVoteStatus.DISCARDED],
    },
    {name: "already accepted cast", initialStatus: CastVoteStatus.VALID, replies: []},
]

test.each(cases)("refreshes cast status: $name", async ({initialStatus, replies}) => {
    // ReviewScreen stores the cast action's response, which has no status field.
    store.dispatch(setElection({...election, contests: [], image_document_id: ""}))
    store.dispatch(addCastVotes([{...cast, ...(initialStatus ? {status: initialStatus} : {})}]))
    const operations: string[] = []
    let statusRequests = 0
    const client = new ApolloClient({
        cache: new InMemoryCache(),
        link: new ApolloLink(
            (operation) =>
                new Observable((observer) => {
                    operations.push(operation.operationName || "")
                    let data: any
                    if (operation.operationName === "GetVoterStatus") {
                        data = {
                            get_ballot_files_urls: {
                                event_id: "event",
                                status: event.status,
                                files: [
                                    {
                                        id: "style",
                                        election_id: "election",
                                        version: "v1",
                                        urls,
                                        status: election.status,
                                        num_allowed_revotes: 1,
                                        voting_channels: {online: true},
                                    },
                                ],
                            },
                            sequent_backend_cast_vote:
                                initialStatus === CastVoteStatus.VALID
                                    ? [
                                          {
                                              ...cast,
                                              status: initialStatus,
                                              __typename: "sequent_backend_cast_vote",
                                          },
                                      ]
                                    : [],
                        }
                    } else if (operation.operationName === "GetCastVotes") {
                        const status = replies[Math.min(statusRequests++, replies.length - 1)]
                        data = {
                            sequent_backend_cast_vote: status
                                ? [{...cast, status, __typename: "sequent_backend_cast_vote"}]
                                : [],
                        }
                    } else {
                        observer.error(new Error(`Unexpected operation ${operation.operationName}`))
                        return
                    }
                    const timer = setTimeout(() => {
                        observer.next({data})
                        observer.complete()
                    }, 1)
                    return () => clearTimeout(timer)
                })
        ),
    })
    // The pre-cast bootstrap is already cached on return from review/confirmation.
    await client.query({query: GET_VOTER_STATUS, variables: {electionEventId: "event"}})
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId/election-chooser",
                element: <ElectionSelectionScreen />,
            },
        ],
        {initialEntries: ["/tenant/tenant/event/event/election-chooser"]}
    )
    const view = render(
        <Provider store={store} stabilityCheck="never">
            <ThemeProvider theme={theme}>
                <ApolloProvider client={client}>
                    <RouterProvider router={router} />
                </ApolloProvider>
            </ThemeProvider>
        </Provider>
    )
    try {
        const finalStatus = replies[replies.length - 1] ?? initialStatus
        if (finalStatus === CastVoteStatus.DISCARDED) {
            await waitFor(() => expect(screen.getByRole("button", {name: "Vote"})).toBeEnabled())
            expect(store.getState().castVotes.election).toEqual([])
        } else {
            await waitFor(() =>
                expect(store.getState().castVotes.election?.[0].status).toBe(CastVoteStatus.VALID)
            )
            expect(await screen.findByRole("button", {name: "Already voted"})).toBeDisabled()
        }
        // Terminal votes do not keep polling, and initial settled eligibility
        // continues to use only the bootstrap request.
        await act(async () => {
            await new Promise((resolve) => setTimeout(resolve, 60))
        })
        const settledRequests = statusRequests
        await act(async () => {
            await new Promise((resolve) => setTimeout(resolve, 60))
        })
        expect(statusRequests).toBe(settledRequests)
        expect(statusRequests).toBeGreaterThanOrEqual(replies.length)
        if (!replies.length) expect(operations).toEqual(["GetVoterStatus"])
        expect(operations.filter((name) => name === "GetVoterStatus")).toHaveLength(1)
    } finally {
        view.unmount()
        router.dispose()
        client.stop()
    }
})

test("preview publications still populate the chooser without authenticated requests", async () => {
    const eml = {
        ...ELECTION_WITH_INVALID,
        election_id: "election",
        election_event_id: "event",
        area_id: "area",
    }
    loadPreview(
        {
            election_event: event,
            elections: [election],
            ballot_styles: [eml],
            documents: [],
            support_materials: [],
        } as any,
        "tenant",
        "area",
        store.dispatch
    )
    const style = store.getState().ballotStyles.election
    const requests = jest.fn()
    const client = new ApolloClient({
        cache: new InMemoryCache(),
        link: new ApolloLink(
            () =>
                new Observable((observer) => {
                    requests()
                    observer.error(new Error("Preview must not request voter data"))
                })
        ),
    })
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId/election-chooser",
                element: <ElectionSelectionScreen />,
            },
        ],
        {initialEntries: ["/tenant/tenant/event/event/election-chooser"]}
    )
    sessionStorage.setItem("isDemo", "true")
    const view = render(
        <Provider store={store} stabilityCheck="never">
            <ThemeProvider theme={theme}>
                <SettingsContext.Provider value={{globalSettings: {DISABLE_AUTH: true}} as any}>
                    <ApolloProvider client={client}>
                        <RouterProvider router={router} />
                    </ApolloProvider>
                </SettingsContext.Provider>
            </ThemeProvider>
        </Provider>
    )
    try {
        expect(await screen.findByRole("button", {name: "Vote"})).toBeEnabled()
        expect(store.getState().ballotStyles.election).toBe(style)
        expect(requests).not.toHaveBeenCalled()
        expect(global.fetch).not.toHaveBeenCalled()
    } finally {
        view.unmount()
        router.dispose()
        client.stop()
        sessionStorage.removeItem("isDemo")
    }
})
