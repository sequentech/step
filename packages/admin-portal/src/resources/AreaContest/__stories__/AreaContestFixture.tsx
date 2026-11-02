// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type PropsWithChildren} from "react"
import {ResourceContextProvider, ResourceDefinitionContextProvider} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {
    STORY_IDS,
    areaContestRecord,
    areaRecords,
    contestRecord,
    eventRecord,
    storyId,
    tenantRecord,
} from "@/__stories__/fixtures"

/** What the area contest screens' reads do. */
export interface AreaContestServices {
    /** Reading the area contests. */
    reads: ReadState
    /** Whether the tenant has area contests. */
    empty: boolean
}

/** South district's link to the council contest, next to North district's. */
export const southAreaContest = areaContestRecord({
    id: storyId(7, 8),
    area_id: STORY_IDS.secondArea,
})

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

export async function setUpAreaContests({reads, empty}: AreaContestServices) {
    data = resourceBoundary(
        {
            sequent_backend_area_contest: empty ? [] : [areaContestRecord(), southAreaContest],
            sequent_backend_area: areaRecords(),
            sequent_backend_contest: [contestRecord()],
            sequent_backend_election_event: [eventRecord()],
            sequent_backend_tenant: [tenantRecord],
        },
        {reads: {sequent_backend_area_contest: reads}}
    )
    graphql = graphqlBoundary({}, {schema: true})
    await graphql.ready
}

const RESOURCE = "sequent_backend_area_contest"

/** The screens as the area contest resource routes render them, registered as App.tsx does. */
export function AreaContestFixture({children}: PropsWithChildren) {
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <ResourceDefinitionContextProvider
                definitions={{
                    [RESOURCE]: {
                        name: RESOURCE,
                        hasList: true,
                        hasEdit: true,
                        hasCreate: true,
                        hasShow: false,
                        options: {label: "Area Contest"},
                    },
                }}
            >
                <ResourceContextProvider value={RESOURCE}>{children}</ResourceContextProvider>
            </ResourceDefinitionContextProvider>
        </AdminStoryProvider>
    )
}

/** React-admin reads and writes of the story. */
export const dataCalls = () => data.calls
export const dataWrites = () => data.writes
/** Reads of one resource by one data provider method. */
export const reads = (method: string, resource: string) =>
    data.calls.filter((call) => call.method === method && call.args[0] === resource)
