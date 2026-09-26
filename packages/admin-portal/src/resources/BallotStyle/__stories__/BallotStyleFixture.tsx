// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type PropsWithChildren} from "react"
import {ResourceContextProvider, ResourceDefinitionContextProvider} from "react-admin"
import {Outlet} from "react-router"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {
    FIXED_TIME,
    STORY_IDS,
    areaRecords,
    electionRecord,
    eventRecord,
    storyId,
    tenantRecord,
    type StoryRecord,
} from "@/__stories__/fixtures"
import type {Sequent_Backend_Ballot_Style} from "@/gql/graphql"

/** What the ballot style screens' reads do. */
export interface BallotStyleServices {
    /** Reading the ballot styles. */
    reads: ReadState
    /** Whether the tenant has ballot styles. */
    empty: boolean
    /** A save rejects with this message. */
    writeError?: string
}

const RESOURCE = "sequent_backend_ballot_style"

/** The published ballot of the council election for one area. */
const ballotStyle = (id: string, areaId: string): StoryRecord<Sequent_Backend_Ballot_Style> => ({
    id,
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    election_id: STORY_IDS.election,
    area_id: areaId,
    ballot_publication_id: storyId(9, 1),
    ballot_eml: JSON.stringify({id, election_id: STORY_IDS.election, contests: []}),
    ballot_signature: null,
    status: "PUBLISHED",
    created_at: FIXED_TIME,
    last_updated_at: FIXED_TIME,
    deleted_at: null,
    annotations: {},
    labels: {},
})

export const northBallot = ballotStyle(storyId(9, 2), STORY_IDS.area)
export const southBallot = ballotStyle(storyId(9, 3), STORY_IDS.secondArea)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

export async function setUpBallotStyles({reads, empty, writeError}: BallotStyleServices) {
    data = resourceBoundary(
        {
            [RESOURCE]: empty ? [] : [northBallot, southBallot],
            sequent_backend_area: areaRecords(),
            sequent_backend_election: [electionRecord()],
            sequent_backend_election_event: [eventRecord()],
            sequent_backend_tenant: [tenantRecord],
        },
        {reads: {[RESOURCE]: reads}, writeError}
    )
    graphql = graphqlBoundary({}, {schema: true})
    await graphql.ready
}

/** The screens as the ballot style resource routes render them, registered as App.tsx does. */
export function BallotStyleFixture({children}: PropsWithChildren) {
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
                        options: {label: "Ballot Styles"},
                    },
                }}
            >
                <ResourceContextProvider value={RESOURCE}>{children}</ResourceContextProvider>
            </ResourceDefinitionContextProvider>
        </AdminStoryProvider>
    )
}

/**
 * The fixture as the route layout, so that its notifications outlive the
 * redirect that follows a save, as the application's layout does.
 */
export function BallotStyleLayout() {
    return (
        <BallotStyleFixture>
            <Outlet />
        </BallotStyleFixture>
    )
}

export const dataWrites = () => data.writes
/** Reads of one resource by one data provider method. */
export const reads = (method: string, resource: string) =>
    data.calls.filter((call) => call.method === method && call.args[0] === resource)
