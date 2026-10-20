// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Provider} from "react-redux"
import {ApolloProvider} from "@apollo/client/react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {ECastVoteGoldLevelPolicy, EConsolidatedReportPolicy} from "@sequentech/ui-core"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {ScenarioChannel} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {atViewport} from "../../../../test-support/storybook/viewport"
import {AuthContext} from "../../providers/AuthContextProvider"
import {createPreviewApolloClient, previewAuth} from "../../preview/context"
import {store, clearVoterSession} from "../../store/store"
import {setElection} from "../../store/elections/electionsSlice"
import {
    BALLOT_DATA_KEY,
    BALLOT_DATA_EXPIRATION_KEY,
    clearSessionStorageBallotData,
} from "../../store/castVotes/sessionBallotData"
import ReviewScreen from "../ReviewScreen"

const electionPath = `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}`
const reauthenticate = fn(async () => {
    throw new Error("Synthetic authentication outage")
})
const client = createPreviewApolloClient()
const meta = {
    title: "Voting/Cast recovery",
    parameters: {
        router: {
            path: "/tenant/:tenantId/event/:eventId/election/:electionId/review",
            initialEntries: [`${electionPath}/review?lang=en`],
        },
    },
    beforeEach: () => {
        store.dispatch(clearVoterSession())
        store.dispatch(
            setElection({
                id: IDS.election,
                tenant_id: IDS.tenant,
                election_event_id: IDS.event,
                name: "Synthetic recovery election",
                image_document_id: "",
                contests: [],
                presentation: {
                    consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
                    cast_vote_gold_level: ECastVoteGoldLevelPolicy.GOLD_LEVEL,
                },
            })
        )
        reauthenticate.mockClear()
        sessionStorage.setItem(
            BALLOT_DATA_KEY,
            JSON.stringify({
                owner: {tenantId: IDS.tenant, eventId: IDS.event, userId: "synthetic-voter"},
                ballotId: "FTBE-MHRX",
                electionId: IDS.election,
                ballot: "{}",
                isDemo: false,
            })
        )
        sessionStorage.setItem(BALLOT_DATA_EXPIRATION_KEY, String(Date.now() + 300_000))
        return () => {
            clearSessionStorageBallotData()
        }
    },
    render: () => (
        <ApolloProvider client={client}>
            <Provider store={store}>
                <AuthContext.Provider
                    value={{
                        ...previewAuth(ScenarioChannel.ONLINE, () => undefined),
                        userId: "synthetic-voter",
                        reauthWithGold: reauthenticate,
                    }}
                >
                    <main>
                        <ReviewScreen />
                    </main>
                </AuthContext.Provider>
            </Provider>
        </ApolloProvider>
    ),
} satisfies Meta
export default meta
type Story = StoryObj<typeof meta>

export const AuthenticationUnavailable: Story = {
    ...atViewport(1440),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            "The cast result has not been confirmed"
        )
        const saved = sessionStorage.getItem(BALLOT_DATA_KEY)
        const expiry = sessionStorage.getItem(BALLOT_DATA_EXPIRATION_KEY)
        await userEvent.click(canvas.getByRole("button", {name: "Try again"}))
        await expect(reauthenticate).toHaveBeenCalledTimes(2)
        expect(sessionStorage.getItem(BALLOT_DATA_KEY)).toBe(saved)
        expect(sessionStorage.getItem(BALLOT_DATA_EXPIRATION_KEY)).toBe(expiry)
        expect(canvas.getByRole("link", {name: "Check ballot"})).toHaveAttribute(
            "href",
            `${electionPath}/ballot-locator/FTBE-MHRX?lang=en`
        )
        expect(canvasElement.scrollWidth).toBeLessThanOrEqual(canvasElement.clientWidth + 1)
        await userEvent.click(canvas.getByRole("button", {name: "Edit ballot"}))
        expect(sessionStorage.getItem(BALLOT_DATA_KEY)).toBeNull()
    },
}
export const AuthenticationUnavailablePhone: Story = {
    ...AuthenticationUnavailable,
    ...atViewport(390),
}
export const AuthenticationUnavailableTablet: Story = {
    ...AuthenticationUnavailable,
    ...atViewport(820),
}
