// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
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
    return <div data-testid="participation">{count}</div>
}

function tree(accessToken?: string) {
    return (
        <Provider store={store}>
            <AuthContext.Provider
                value={{keycloakAccessToken: accessToken, isAuthContextInitialized: true} as any}
            >
                <ApolloWrapper>
                    <Participation />
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
    mounted.rerender(tree(token("first-area", 200)))
    expect(screen.getByTestId("participation")).toHaveTextContent("1")
    mounted.rerender(tree(token("second-area", 300)))
    await waitFor(() => expect(screen.getByTestId("participation")).toHaveTextContent("0"))
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
