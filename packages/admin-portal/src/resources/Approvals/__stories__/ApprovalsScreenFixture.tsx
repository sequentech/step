// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type PropsWithChildren} from "react"
import type {DataProvider, GetListParams} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {documentHandlers} from "@/__stories__/downloads"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {areaRecords, electionRecord, storyId} from "@/__stories__/fixtures"
import {IApplicationsStatus} from "@/types/applications"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"
import {
    APPLICATION_ID,
    APPROVAL_ATTRIBUTES,
    MATCHING_VOTERS,
    SECOND_APPLICATION_ID,
    applicationRecord,
} from "./ApprovalsFixture"

/** What the approval screens' services do. */
export interface ApprovalServices {
    /** Reading the applications. */
    reads: ReadState
    /** Whether the event has applications. */
    empty: boolean
    /** The status filter the administrator last chose on this browser. */
    storedStatus?: string
}

type Handlers = Parameters<typeof graphqlBoundary>[0]

export const EXPORT_DOCUMENT_ID = storyId(9, 7)

/** Alice's pending application and Bob's accepted one. */
export const applications = () => [
    applicationRecord(),
    applicationRecord({
        id: SECOND_APPLICATION_ID,
        applicant_id: "applicant-0002",
        applicant_data: {
            firstName: "Bob",
            lastName: "Example",
            email: "bob@example.test",
            dateOfBirth: "1985-02-03",
            embassy: "Lisbon",
        },
        annotations: {
            "search-attributes": "firstName,lastName",
            "verified_by": "admin",
            "decision": {
                matrix_version: 1,
                matrix_source: "BUILT_IN",
                rule: 4,
                conditions: {differing: "exactly_1", fields: {embassy: "DIFFERS"}},
                decision: IApplicationsStatus.ACCEPTED,
                reason: null,
                inputs: {
                    identity: null,
                    voter_found: true,
                    already_enrolled: false,
                    valid_id: null,
                    fields: {firstName: "MATCHES", lastName: "MATCHES", embassy: "DIFFERS"},
                    differing: 1,
                },
                candidates: 1,
                accepted_candidates: 1,
            },
        },
        status: IApplicationsStatus.ACCEPTED,
    }),
]

// ra-data-hasura compares String columns with a case-insensitive `_ilike`.
const STRING_COLUMNS = ["status", "verification_type", "applicant_id"]

function withStringFilters(provider: DataProvider): DataProvider {
    return {
        ...provider,
        getList: (resource: string, params: GetListParams) =>
            provider.getList(resource, {
                ...params,
                filter: Object.fromEntries(
                    Object.entries(params.filter ?? {}).map(([key, value]) =>
                        STRING_COLUMNS.includes(key) ? [`${key}@_ilike`, value] : [key, value]
                    )
                ),
            }),
    }
}

const STATUS_FILTER_KEY = "approvals_status_filter"

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

export async function setUpApprovals(
    {reads, empty, storedStatus}: ApprovalServices,
    operations: Handlers = {}
) {
    if (storedStatus) localStorage.setItem(STATUS_FILTER_KEY, storedStatus)
    else localStorage.removeItem(STATUS_FILTER_KEY)
    data = resourceBoundary(
        {
            sequent_backend_applications: empty ? [] : applications(),
            sequent_backend_election: [electionRecord()],
            sequent_backend_area: areaRecords(),
            user: MATCHING_VOTERS,
        },
        {reads: {sequent_backend_applications: reads}}
    )
    graphql = graphqlBoundary(
        {
            getUserProfileAttributes: () => ({
                data: {get_user_profile_attributes: APPROVAL_ATTRIBUTES},
            }),
            ...documentHandlers({[EXPORT_DOCUMENT_ID]: {name: "export-applications.csv"}}),
            ...operations,
        },
        {schema: true}
    )
    await graphql.ready
}

/** The approval screens inside an election event, as the signed-in administrator sees them. */
export function ApprovalsScreen({children}: PropsWithChildren) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={withStringFilters(data.provider)}
            role={permissions}
            tenant={tenant}
        >
            {children}
        </AdminStoryProvider>
    )
}

export {APPLICATION_ID, SECOND_APPLICATION_ID}
export const graphqlCalls = () => graphql.calls
/** Calls of one data provider method for one resource. */
export const reads = (method: string, resource: string) =>
    data.calls.filter((call) => call.method === method && call.args[0] === resource)
/** The filter of each list read of a resource. */
export const listFilters = (resource: string) =>
    reads("getList", resource).map((call) => (call.args[1] as GetListParams).filter)
