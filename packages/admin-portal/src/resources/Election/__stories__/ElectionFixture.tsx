// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, type PropsWithChildren} from "react"
import {Outlet} from "react-router"
import {fn} from "storybook/test"
import {initCore} from "@sequentech/ui-core"
import {graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {ResourceScreen} from "@/__stories__/resourceScreen"
import {
    STORY_IDS,
    contestRecord,
    electionPresentation,
    electionRecord,
    eventRecord,
    tenantRecord,
} from "@/__stories__/fixtures"
import {ElectionEventTallyContext} from "@/providers/ElectionEventTallyProvider"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {EStoryWorkflow, useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

/** What the election screens' services do. */
export interface ElectionServices {
    /** Reading the elections. */
    reads: ReadState
    /** Whether the tenant has elections. */
    empty: boolean
    /** A save rejects with this message. */
    writeError?: string
}

type Handlers = Parameters<typeof graphqlBoundary>[0]

export const RESOURCE = "sequent_backend_election"

/** The ended council election and a mayoral election whose voting has not started. */
export const elections = () => [
    electionRecord(),
    electionRecord(EStoryWorkflow.PUBLISHED, {
        id: STORY_IDS.secondElection,
        description: "Choose the mayor",
        presentation: electionPresentation("Mayoral election"),
    }),
]

/** The council election's two contests. */
export const contests = () => [
    contestRecord(),
    contestRecord({
        id: STORY_IDS.secondContest,
        description: "Choose the chair",
        presentation: {i18n: {en: {name: "Council chair", alias: "Chair"}}},
    }),
]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

/** NewResourceContext's record of the last created resource. */
export const lastCreated = fn()
/** The tally store's election. */
export const electionFlag = fn()

export async function setUpElections(
    {reads, empty, writeError}: ElectionServices,
    operations: Handlers = {},
    records: {elections?: ReturnType<typeof elections>; contests?: ReturnType<typeof contests>} = {}
) {
    await initCore()
    lastCreated.mockClear()
    electionFlag.mockClear()
    data = resourceBoundary(
        {
            [RESOURCE]: empty ? [] : (records.elections ?? elections()),
            sequent_backend_contest: records.contests ?? contests(),
            sequent_backend_election_event: [eventRecord()],
            sequent_backend_tenant: [tenantRecord],
            sequent_backend_document: [],
        },
        {reads: {[RESOURCE]: reads}, writeError}
    )
    graphql = graphqlBoundary(
        {
            election_events_tree: () => ({data: {sequent_backend_election_event: []}}),
            ...operations,
        },
        {schema: true}
    )
    await graphql.ready
}

/** The election screens with the tree menu, tally store and new-resource contexts. */
export function ElectionScreen({children}: PropsWithChildren) {
    const {permissions, tenant} = useStoryGlobals()
    const tally = useContext(ElectionEventTallyContext)
    return (
        <ResourceScreen
            resource={RESOURCE}
            label="Elections"
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <NewResourceContext.Provider
                value={{lastCreatedResource: null, setLastCreatedResource: lastCreated}}
            >
                <ElectionEventTallyContext.Provider
                    value={{...tally, setElectionIdFlag: electionFlag}}
                >
                    {children}
                </ElectionEventTallyContext.Provider>
            </NewResourceContext.Provider>
        </ResourceScreen>
    )
}

/** The screen as the route layout, so that notifications outlive a redirect. */
export function ElectionLayout() {
    return (
        <ElectionScreen>
            <Outlet />
        </ElectionScreen>
    )
}

export const graphqlCalls = () => graphql.calls
export const dataWrites = () => data.writes
/** Calls of one data provider method for one resource. */
export const reads = (method: string, resource: string) =>
    data.calls.filter((call) => call.method === method && call.args[0] === resource)
