// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {gql} from "@apollo/client"
import {useApolloClient} from "@apollo/client/react"
import {act, render, screen, waitFor} from "@testing-library/react"
import {Provider, useSelector} from "react-redux"
import {ApolloWrapper} from "./ApolloContextProvider"
import {AuthContext} from "./AuthContextProvider"
import {store, RootState, clearVoterSession} from "../store/store"
import {addCastVotes, CastVoteStatus} from "../store/castVotes/castVotesSlice"

jest.mock("./AuthContextProvider", () => ({AuthContext: require("react").createContext({})}))
jest.mock("./SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {DISABLE_AUTH: false, HASURA_URL: "http://localhost/graphql"},
    }),
}))

function token(area: string, exp: number) {
    return `header.${btoa(JSON.stringify({"sub": "voter", exp, "https://hasura.io/jwt/claims": {"x-hasura-area-id": area}}))}.signature`
}

function Participation() {
    const count = useSelector((state: RootState) => state.castVotes.election?.length || 0)
    const [page, setPage] = React.useState(1)
    return (
        <>
            <div data-testid="participation">{count}</div>
            <button onClick={() => setPage(2)}>Page {page}</button>
        </>
    )
}

let requestClient: ReturnType<typeof useApolloClient>
const RequestClient = () => {
    requestClient = useApolloClient()
    return null
}
function tree(accessToken?: string, requestToken = async () => accessToken) {
    const authContext: React.ContextType<typeof AuthContext> = {
        isAuthContextInitialized: true,
        isAuthenticated: Boolean(accessToken),
        userId: "voter",
        username: "voter",
        email: "",
        firstName: "",
        logout: jest.fn(),
        hasRole: () => false,
        isKiosk: () => false,
        getExpiry: () => undefined,
        keycloakAccessToken: accessToken,
        getAccessTokenForRequest: requestToken,
        setTenantEvent: jest.fn(),
        openProfileLink: jest.fn().mockResolvedValue(undefined),
        isGoldUser: () => false,
        reauthWithGold: jest.fn().mockResolvedValue(undefined),
    }
    return (
        <Provider store={store}>
            <AuthContext.Provider value={authContext}>
                <ApolloWrapper>
                    <Participation />
                    <RequestClient />
                </ApolloWrapper>
            </AuthContext.Provider>
        </Provider>
    )
}

test("refresh retains ballot state, while scope changes and logout clear it", async () => {
    store.dispatch(clearVoterSession())
    const mounted = render(tree(token("first-area", 100)))
    await screen.findByTestId("participation")
    act(() => {
        store.dispatch(
            addCastVotes([
                {
                    id: "cast",
                    tenant_id: "tenant",
                    election_event_id: "event",
                    election_id: "election",
                    status: CastVoteStatus.VALID,
                },
            ])
        )
    })
    expect(screen.getByTestId("participation")).toHaveTextContent("1")
    act(() => screen.getByRole("button", {name: "Page 1"}).click())
    mounted.rerender(tree(token("first-area", 200)))
    expect(screen.getByTestId("participation")).toHaveTextContent("1")
    expect(screen.getByRole("button", {name: "Page 2"})).toBeVisible()
    mounted.rerender(tree(token("second-area", 300)))
    await waitFor(() => expect(screen.getByTestId("participation")).toHaveTextContent("0"))
    expect(screen.getByRole("button", {name: "Page 1"})).toBeVisible()
    act(() => {
        store.dispatch(
            addCastVotes([
                {
                    id: "another-cast",
                    tenant_id: "tenant",
                    election_event_id: "event",
                    election_id: "election",
                    status: CastVoteStatus.VALID,
                },
            ])
        )
    })
    mounted.rerender(tree())
    expect(screen.queryByTestId("participation")).toBeNull()
    expect(store.getState().castVotes).toEqual({})
    mounted.unmount()
})

test("checks current authentication at request dispatch before calling HTTP", async () => {
    const fetch = jest.spyOn(globalThis, "fetch").mockRejectedValue(new Error("transport boundary"))
    const requestToken = jest.fn<Promise<string | undefined>, []>().mockResolvedValue(undefined)
    const mounted = render(tree(token("first-area", 100), requestToken))
    await screen.findByTestId("participation")
    const query = gql`
        query IdleGuardProbe {
            __typename
        }
    `
    await expect(requestClient.query({query, fetchPolicy: "network-only"})).rejects.toThrow(
        "No active authentication token"
    )
    expect(fetch).not.toHaveBeenCalled()
    requestToken.mockResolvedValue("fresh-token")
    await expect(requestClient.query({query, fetchPolicy: "network-only"})).rejects.toThrow(
        "transport boundary"
    )
    expect(fetch).toHaveBeenCalledTimes(1)
    expect(new Headers(fetch.mock.calls[0][1]?.headers).get("authorization")).toBe(
        "Bearer fresh-token"
    )
    mounted.unmount()
    fetch.mockRestore()
})
