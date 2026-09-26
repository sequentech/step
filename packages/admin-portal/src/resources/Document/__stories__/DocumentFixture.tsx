// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type PropsWithChildren} from "react"
import {Outlet} from "react-router"
import {EVENT_ID, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {documentHandlers} from "@/__stories__/downloads"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {ResourceScreen} from "@/__stories__/resourceScreen"
import {
    FIXED_TIME,
    eventRecord,
    storyId,
    tenantRecord,
    type StoryRecord,
} from "@/__stories__/fixtures"
import type {Sequent_Backend_Document} from "@/gql/graphql"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

/** What the document screens' services do. */
export interface DocumentServices {
    /** Reading the documents. */
    reads: ReadState
    /** Whether the tenant has documents. */
    empty: boolean
    /** A save rejects with this message. */
    writeError?: string
    /** Whether Harvest issues download addresses for the documents. */
    downloadable: boolean
}

export const RESOURCE = "sequent_backend_document"
export const RESULTS_ID = storyId(9, 1)
export const ROLL_ID = storyId(9, 2)

function documentRecord(
    overrides: Partial<StoryRecord<Sequent_Backend_Document>>
): StoryRecord<Sequent_Backend_Document> {
    return {
        id: RESULTS_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: "results.pdf",
        media_type: "application/pdf",
        size: 48213,
        is_public: false,
        labels: {},
        annotations: {},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        ...overrides,
    }
}

/** The event's results report and its voter roll. */
export const documents = () => [
    documentRecord({labels: {kind: "results"}}),
    documentRecord({
        id: ROLL_ID,
        name: "voter-roll.csv",
        media_type: "text/csv",
        size: 912,
    }),
]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

export async function setUpDocuments({reads, empty, writeError, downloadable}: DocumentServices) {
    data = resourceBoundary(
        {
            [RESOURCE]: empty ? [] : documents(),
            sequent_backend_election_event: [eventRecord()],
            sequent_backend_tenant: [tenantRecord],
        },
        {reads: {[RESOURCE]: reads}, writeError}
    )
    const {FetchDocument} = documentHandlers(
        downloadable
            ? {[RESULTS_ID]: {name: "results.pdf"}, [ROLL_ID]: {name: "voter-roll.csv"}}
            : {}
    )
    graphql = graphqlBoundary({FetchDocument}, {schema: true})
    await graphql.ready
}

/** The document screens as the signed-in administrator sees them. */
export function DocumentScreen({children}: PropsWithChildren) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <ResourceScreen
            resource={RESOURCE}
            label="Documents"
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            {children}
        </ResourceScreen>
    )
}

/** The screen as the route layout, so that notifications outlive a redirect. */
export function DocumentLayout() {
    return (
        <DocumentScreen>
            <Outlet />
        </DocumentScreen>
    )
}

export const graphqlCalls = () => graphql.calls
export const dataWrites = () => data.writes
/** Calls of one data provider method for one resource. */
export const reads = (method: string, resource: string) =>
    data.calls.filter((call) => call.method === method && call.args[0] === resource)
