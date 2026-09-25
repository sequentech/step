// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {Meta, StoryObj} from "@storybook/react"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, ResourceContextProvider, type RaRecord} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {CreateElectionList} from "./CreateElectionEvent"

interface Scenario {
    settings: Record<string, unknown>
    canWrite: boolean
    failure: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>
const created = fn()
function Fixture({canWrite}: Scenario) {
    const auth = useContext(AuthContext)
    const settings = useContext(SettingsContext)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <AuthContext.Provider
                value={{
                    ...auth,
                    isAuthorized: (_super, _tenant, permission) =>
                        canWrite && permission === "election-event-write",
                }}
            >
                <SettingsContext.Provider
                    value={{
                        ...settings,
                        globalSettings: {...settings.globalSettings, QUERY_POLL_INTERVAL_MS: 0},
                    }}
                >
                    <NewResourceContext.Provider
                        value={{lastCreatedResource: null, setLastCreatedResource: created}}
                    >
                        <ResourceContextProvider value="sequent_backend_election_event">
                            <RecordContextProvider value={{id: EVENT_ID}}>
                                <CreateElectionList />
                            </RecordContextProvider>
                        </ResourceContextProvider>
                    </NewResourceContext.Provider>
                </SettingsContext.Provider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/Create event language policy",
    component: Fixture,
    args: {
        settings: {
            language_conf: {enabled_language_codes: ["es", "en"], default_language_code: "es"},
        },
        canWrite: true,
        failure: false,
    },
    beforeEach: ({args}) => {
        created.mockClear()
        const tenant = {id: TENANT_ID, slug: "synthetic", settings: args.settings}
        boundary = graphqlBoundary({
            election_events_tree: () => ({data: {sequent_backend_election_event: []}}),
            CreateElectionEvent: () => {
                if (args.failure) throw new Error("Synthetic creation unavailable")
                return {
                    data: {
                        insertElectionEvent: {
                            id: EVENT_ID,
                            message: null,
                            error: null,
                            task_execution: null,
                        },
                    },
                }
            },
        })
        data = dataBoundary({
            getOne: async <RecordType extends RaRecord>(
                resource: string,
                {id}: {id: string | number}
            ) => {
                if (resource !== "sequent_backend_tenant" || id !== TENANT_ID) {
                    data.unexpected.push(`${resource}/${id}`)
                    throw new Error("Unexpected tenant read")
                }
                return {data: tenant as unknown as RecordType}
            },
            getMany: async <RecordType extends RaRecord>(
                resource: string,
                {ids}: {ids: (string | number)[]}
            ) => {
                if (resource !== "sequent_backend_tenant" || ids.some((id) => id !== TENANT_ID)) {
                    data.unexpected.push(resource)
                    throw new Error("Unexpected tenant read")
                }
                return {data: [tenant] as unknown as RecordType[]}
            },
            getList: async <RecordType extends RaRecord>(resource: string) => {
                if (resource === "sequent_backend_tenant")
                    return {data: [tenant] as unknown as RecordType[], total: 1}
                if (resource === "sequent_backend_election_event")
                    return {data: [{id: EVENT_ID}] as RecordType[], total: 1}
                data.unexpected.push(resource)
                throw new Error("Unexpected record list")
            },
        })
        const services = boundary
        const records = data
        return () => {
            expect(services.unexpected).toEqual([])
            expect(records.unexpected).toEqual([])
        }
    },
    render: (args) => <Fixture {...args} />,
} satisfies Meta<Scenario>
export default meta
type Story = StoryObj<typeof meta>
async function submit(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.type(await canvas.findByRole("textbox", {name: "Name"}), "Council event")
    await userEvent.type(
        canvas.getByRole("textbox", {name: "Description"}),
        "Annual council election"
    )
    await userEvent.click(canvas.getByRole("button", {name: "Save"}))
    await waitFor(() =>
        expect(boundary.calls.filter((call) => call.name === "CreateElectionEvent")).toHaveLength(1)
    )
}
function assertLanguage(language_conf: Record<string, unknown>) {
    const call = boundary.calls.find((call) => call.name === "CreateElectionEvent")
    expect(call?.variables.electionEvent).toEqual(
        expect.objectContaining({
            id: EVENT_ID,
            tenant_id: TENANT_ID,
            name: "Council event",
            description: "Annual council election",
            encryption_protocol: "RSA256",
            is_archived: false,
            presentation: expect.objectContaining({
                language_conf,
                i18n: expect.objectContaining({
                    en: expect.objectContaining({name: "Council event"}),
                }),
            }),
        })
    )
    expect(created).toHaveBeenCalledWith({id: EVENT_ID, type: "sequent_backend_election_event"})
}
export const TenantLanguageConfiguration: Story = {
    play: async ({canvasElement}) => {
        await submit(canvasElement)
        assertLanguage({enabled_language_codes: ["es", "en"], default_language_code: "es"})
        expect(
            boundary.calls.find((call) => call.name === "election_events_tree")?.variables
        ).toEqual({tenantId: TENANT_ID, isArchived: false})
    },
}
export const LegacyTenantLanguages: Story = {
    args: {settings: {languages: ["en", "fr"]}},
    play: async ({canvasElement}) => {
        await submit(canvasElement)
        assertLanguage({enabled_language_codes: ["en", "fr"], default_language_code: "en"})
    },
}
export const DefaultLanguageWhenNotConfigured: Story = {
    args: {settings: {}},
    play: async ({canvasElement}) => {
        await submit(canvasElement)
        assertLanguage({enabled_language_codes: ["en"], default_language_code: "en"})
    },
}
export const CreationFailureKeepsFormAvailable: Story = {
    args: {failure: true},
    play: async ({canvasElement}) => {
        await submit(canvasElement)
        await waitFor(() =>
            expect(within(document.body).getByText("Error creating election event")).toBeVisible()
        )
        expect(created).not.toHaveBeenCalled()
        expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeEnabled()
    },
}
export const CreationWithoutWritePermission: Story = {
    args: {canWrite: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("textbox", {name: "Name"})
        expect(canvas.queryByRole("button", {name: "Save"})).not.toBeInTheDocument()
        expect(boundary.calls.filter((call) => call.name === "CreateElectionEvent")).toEqual([])
        expect(created).not.toHaveBeenCalled()
    },
}
