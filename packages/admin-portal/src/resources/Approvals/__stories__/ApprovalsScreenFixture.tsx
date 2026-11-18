// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type PropsWithChildren} from "react"
import {memoryStore} from "react-admin"
import type {DataProvider, GetListParams, RaRecord, Store} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {getPreferenceKey} from "@/lib/helpers"
import {documentHandlers} from "@/__stories__/downloads"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {areaRecords, electionRecord, storyId} from "@/__stories__/fixtures"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"
import {
    APPLICATION_ID,
    APPROVAL_ATTRIBUTES,
    MANUAL_APPLICATION_ID,
    REGISTRY_VOTERS,
    REJECTED_APPLICATION_ID,
    SECOND_APPLICATION_ID,
    applications,
} from "./ApprovalsFixture"

/** What the approval screens' services do. */
export interface ApprovalServices {
    /** Reading the applications. */
    reads: ReadState
    /** Whether the event has applications. */
    empty: boolean
    /** The status filter the administrator last chose on this browser. */
    storedStatus?: string
    /** Looking voters up in the registry; default "records". */
    registry?: ReadState
}

type Handlers = Parameters<typeof graphqlBoundary>[0]

export const EXPORT_DOCUMENT_ID = storyId(9, 7)

// ra-data-hasura compares String columns with a case-insensitive `_ilike`.
const STRING_COLUMNS = ["status", "verification_type", "applicant_id"]

const isLike = (value: unknown): value is {IsLike: string} =>
    typeof value === "object" && value !== null && "IsLike" in value

/**
 * Whether a registry voter passes a lookup as the users service answers it:
 * `{IsLike}` finds the text inside a column, and `attributes` compares each
 * attribute's value.
 */
function voterMatches(voter: RaRecord, filter: Record<string, unknown> = {}) {
    return Object.entries(filter).every(([key, expected]) => {
        if (isLike(expected)) {
            return String(voter[key] ?? "")
                .toLowerCase()
                .includes(expected.IsLike.toLowerCase())
        }
        if (key === "attributes" && typeof expected === "object" && expected !== null) {
            return Object.entries(expected).every(([name, value]) =>
                [voter.attributes?.[name]].flat().includes(value)
            )
        }
        return true
    })
}

/** The filters the story's records can't answer the way the services do. */
export function withServiceFilters(provider: DataProvider): DataProvider {
    return {
        ...provider,
        getList: async (resource: string, params: GetListParams) => {
            if (resource === "user") {
                const page = await provider.getList(resource, params)
                const data = page.data.filter((voter) => voterMatches(voter, params.filter))
                return {data, total: data.length}
            }
            const page = await provider.getList(resource, {
                ...params,
                filter: Object.fromEntries(
                    Object.entries(params.filter ?? {}).map(([key, value]) =>
                        STRING_COLUMNS.includes(key) ? [`${key}@_ilike`, value] : [key, value]
                    )
                ),
            })
            // The queue's search looks for the text anywhere in the applicant's answers.
            const search = String(params.filter?.q ?? "")
                .trim()
                .toLowerCase()
            if (resource !== "sequent_backend_applications" || !search) return page
            const data = page.data.filter((application) =>
                JSON.stringify(application.applicant_data).toLowerCase().includes(search)
            )
            return {data, total: data.length}
        },
    }
}

/** The election event's page, where the Approvals tab shows the queue. */
export const EVENT_PATH = `/sequent_backend_election_event/${EVENT_ID}`

/** The signed-in administrator's Keycloak user ID, and another administrator's. */
export const ADMIN_ID = storyId(8, 1)
export const OTHER_ADMIN_ID = storyId(8, 2)

/** Where react-admin keeps an administrator's list parameters of the queue at `pathname`. */
export const queueStoreKey = (pathname: string, userId = ADMIN_ID) =>
    `${getPreferenceKey(pathname, "approvals_queue")}.${userId}`

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let preferences: Store

export async function setUpApprovals(
    {reads, empty, storedStatus, registry = "records"}: ApprovalServices,
    operations: Handlers = {}
) {
    preferences = memoryStore(
        storedStatus ? {[queueStoreKey(EVENT_PATH)]: {filter: {status: storedStatus}}} : {}
    )
    data = resourceBoundary(
        {
            sequent_backend_applications: empty ? [] : applications(),
            sequent_backend_election: [electionRecord()],
            sequent_backend_area: areaRecords(),
            user: REGISTRY_VOTERS,
        },
        {reads: {sequent_backend_applications: reads, user: registry}}
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

/**
 * The approval screens inside an election event, as the signed-in administrator
 * (`userId`, empty while the profile loads) sees them. `store` replaces the
 * preferences `storedStatus` prepared.
 */
export function ApprovalsScreen({
    children,
    store,
    userId = ADMIN_ID,
}: PropsWithChildren<{store?: Store; userId?: string}>) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={withServiceFilters(data.provider)}
            role={permissions}
            tenant={tenant}
            store={store ?? preferences}
            auth={{userId}}
        >
            {children}
        </AdminStoryProvider>
    )
}

export {APPLICATION_ID, MANUAL_APPLICATION_ID, REJECTED_APPLICATION_ID, SECOND_APPLICATION_ID}
export const graphqlCalls = (name?: string) =>
    name ? graphql.calls.filter((call) => call.name === name) : graphql.calls
/** Calls of one data provider method for one resource. */
export const reads = (method: string, resource: string) =>
    data.calls.filter((call) => call.method === method && call.args[0] === resource)
/** The filter of each list read of a resource. */
export const listFilters = (resource: string) =>
    reads("getList", resource).map((call) => (call.args[1] as GetListParams).filter)
