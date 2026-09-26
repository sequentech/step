// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, type PropsWithChildren} from "react"
import {Outlet} from "react-router"
import {fn} from "storybook/test"
import {initCore} from "@sequentech/ui-core"
import type {DataProvider} from "react-admin"
import {graphqlBoundary, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {ResourceScreen, withMissingRecords} from "@/__stories__/resourceScreen"
import {
    STORY_IDS,
    candidateRecords,
    contestRecord,
    electionRecord,
    eventRecord,
    tenantRecord,
} from "@/__stories__/fixtures"
import {ElectionEventTallyContext} from "@/providers/ElectionEventTallyProvider"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

/** What the contest screens' services do. */
export interface ContestServices {
    /** Reading the contests. */
    reads: ReadState
    /** Whether the tenant has contests. */
    empty: boolean
    /** A save rejects with this message. */
    writeError?: string
}

export const RESOURCE = "sequent_backend_contest"

/** The council contest and a second, single-seat contest of the same election. */
export const contests = () => [
    contestRecord(),
    contestRecord({
        id: STORY_IDS.secondContest,
        description: "Choose the chair",
        max_votes: 1,
        winning_candidates_num: 1,
        presentation: {i18n: {en: {name: "Council chair", alias: "Chair"}}},
    }),
]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let provider: DataProvider

/** NewResourceContext's record of the last created resource. */
export const lastCreated = fn()
/** The tally store's contest. */
export const contestFlag = fn()

export async function setUpContests({reads, empty, writeError}: ContestServices) {
    await initCore()
    lastCreated.mockClear()
    contestFlag.mockClear()
    data = resourceBoundary(
        {
            [RESOURCE]: empty ? [] : contests(),
            sequent_backend_candidate: candidateRecords(),
            sequent_backend_election: [electionRecord()],
            sequent_backend_election_event: [eventRecord()],
            sequent_backend_tenant: [tenantRecord],
            sequent_backend_document: [],
        },
        {reads: {[RESOURCE]: reads}, writeError}
    )
    // Without a picture, ContestDataForm reads the document whose ID is the
    // tenant's, which Hasura does not find.
    provider = withMissingRecords(data.provider, [["sequent_backend_document", TENANT_ID]])
    graphql = graphqlBoundary(
        {election_events_tree: () => ({data: {sequent_backend_election_event: []}})},
        {schema: true}
    )
    await graphql.ready
}

/** The contest screens with the tree menu, tally store and new-resource contexts. */
export function ContestScreen({children}: PropsWithChildren) {
    const {permissions, tenant} = useStoryGlobals()
    const tally = useContext(ElectionEventTallyContext)
    return (
        <ResourceScreen
            resource={RESOURCE}
            label="Contests"
            boundary={graphql}
            dataProvider={provider}
            role={permissions}
            tenant={tenant}
        >
            <NewResourceContext.Provider
                value={{lastCreatedResource: null, setLastCreatedResource: lastCreated}}
            >
                <ElectionEventTallyContext.Provider
                    value={{...tally, setContestIdFlag: contestFlag}}
                >
                    {children}
                </ElectionEventTallyContext.Provider>
            </NewResourceContext.Provider>
        </ResourceScreen>
    )
}

/** The screen as the route layout, so that notifications outlive a redirect. */
export function ContestLayout() {
    return (
        <ContestScreen>
            <Outlet />
        </ContestScreen>
    )
}

export const graphqlCalls = () => graphql.calls
export const dataWrites = () => data.writes
/** Calls of one data provider method for one resource. */
export const reads = (method: string, resource: string) =>
    data.calls.filter((call) => call.method === method && call.args[0] === resource)
