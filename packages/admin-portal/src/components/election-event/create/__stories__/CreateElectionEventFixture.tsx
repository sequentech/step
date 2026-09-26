// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The election event creation and import flows. The widgets read the
// application's create provider, whose context is not exported, so stories
// render that provider over boundaries that create or import the event.
import React, {type PropsWithChildren} from "react"
import {fn, userEvent} from "storybook/test"
import type {RaRecord} from "react-admin"
import {json} from "@sequentech/ui-test-kit/mocks/http"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {eventRecord, storyId, tenantRecord} from "@/__stories__/fixtures"
import {storyFetch} from "@/__stories__/storyNetwork"
import {CreateElectionEventProvider} from "@/providers/CreateElectionEventContextProvider"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import type {EStoryPermissions, EStoryTenant} from "../../../../../ui-essentials/.storybook/globals"

export const UPLOAD_URL = "https://files.admin-story.invalid/upload/council-event.json"
export const DOCUMENT_ID = storyId(6, 1)
export const IMPORTED_EVENT_ID = storyId(2, 9)
export const CHECKSUM = "0f".repeat(32)
export const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/

export interface CreateFlowOptions {
    /** The tenant's existing election events. */
    events?: RaRecord[]
    /** What reading the election events does. */
    reads?: ReadState
    /** Whether the creation service fails. */
    createFailure?: boolean
    /** The import check's error for the uploaded file, if it rejects it. */
    importError?: string
}

export interface CreateFlow {
    data: ReturnType<typeof resourceBoundary>
    graphql: ReturnType<typeof graphqlBoundary>
    uploads: ReturnType<typeof storyFetch>
    /** The provider's report of each created or imported election event. */
    created: ReturnType<typeof fn>
}

/** Boundaries of the flows; call it in the story's beforeEach and await `graphql.ready`. */
export function createFlow({
    events = [],
    reads = "records",
    createFailure = false,
    importError,
}: CreateFlowOptions = {}): CreateFlow {
    const data = resourceBoundary(
        {sequent_backend_tenant: [tenantRecord], sequent_backend_election_event: events},
        {reads: {sequent_backend_election_event: reads}}
    )
    const graphql = graphqlBoundary(
        {
            CreateElectionEvent: ({variables}) => {
                if (createFailure) throw new Error("Synthetic creation service unavailable")
                const {id} = variables.electionEvent as {id: string}
                // The created event becomes readable, as Hasura's row does.
                data.records.sequent_backend_election_event.push(eventRecord(undefined, {id}))
                return {
                    data: {
                        insertElectionEvent: {id, message: null, error: null, task_execution: null},
                    },
                }
            },
            GetUploadUrl: () => ({
                data: {get_upload_url: {url: UPLOAD_URL, document_id: DOCUMENT_ID}},
            }),
            ImportElectionEvent: ({variables}) =>
                variables.checkOnly && importError
                    ? {
                          data: {
                              import_election_event: {
                                  id: null,
                                  message: null,
                                  error: importError,
                                  task_execution: null,
                              },
                          },
                      }
                    : {
                          data: {
                              import_election_event: {
                                  id: variables.checkOnly ? null : IMPORTED_EVENT_ID,
                                  message: null,
                                  error: null,
                                  task_execution: null,
                              },
                          },
                      },
            // The provider refreshes the event tree after a creation.
            election_events_tree: () => ({data: {sequent_backend_election_event: []}}),
        },
        {schema: true}
    )
    const uploads = storyFetch({[UPLOAD_URL]: () => json(200, {})})
    return {data, graphql, uploads, created: fn()}
}

/** The signed-in administrator's tenant with the application's create provider. */
export function CreateFlowStory({
    flow,
    role,
    tenant,
    children,
}: PropsWithChildren<{flow: CreateFlow; role: EStoryPermissions; tenant?: EStoryTenant}>) {
    return (
        <AdminStoryProvider
            boundary={flow.graphql}
            dataProvider={flow.data.provider}
            role={role}
            tenant={tenant}
        >
            <NewResourceContext.Provider
                value={{lastCreatedResource: null, setLastCreatedResource: flow.created}}
            >
                <CreateElectionEventProvider>{children}</CreateElectionEventProvider>
            </NewResourceContext.Provider>
        </AdminStoryProvider>
    )
}

/** Selects an exported election event file in the import drop zone under `root`. */
export async function chooseImportFile(root: HTMLElement, name = "council-event.json") {
    const input = root.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("The import file input is missing")
    await userEvent.upload(input, new File(['{"elections":[]}'], name, {type: "application/json"}))
}
