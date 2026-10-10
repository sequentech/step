// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {StoryObj} from "@storybook/react"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {expect, spyOn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, type RaRecord} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ListUsers} from "./ListUsers"

const USER_ID = "33333333-3333-4333-8333-333333333333"
const DOCUMENT_ID = "44444444-4444-4444-8444-444444444444"
const UPLOAD_URL = "https://admin-story.invalid/upload/users"
const CHECKSUM = "ab".repeat(32)
const event = {id: EVENT_ID, tenant_id: TENANT_ID, name: "Council", presentation: {}}
const user = {
    id: USER_ID,
    username: "alice",
    enabled: true,
    email_verified: false,
    attributes: {},
    votes_info: [],
}
interface Scenario {
    permissions: string[]
    eventScope: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>
function Fixture({permissions, eventScope}: Scenario) {
    const auth = useContext(AuthContext)
    const settings = useContext(SettingsContext)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <AuthContext.Provider
                value={{
                    ...auth,
                    isAuthorized: (_super, _tenant, permission) =>
                        permissions.includes(String(permission)),
                }}
            >
                <SettingsContext.Provider
                    value={{
                        ...settings,
                        globalSettings: {
                            ...settings.globalSettings,
                            QUERY_POLL_INTERVAL_MS: 3600000,
                        },
                    }}
                >
                    <RecordContextProvider value={eventScope ? event : undefined}>
                        <ListUsers electionEventId={eventScope ? EVENT_ID : undefined} />
                    </RecordContextProvider>
                </SettingsContext.Provider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/User/ListUsers",
    component: ListUsers,
    args: {
        permissions: [
            "voter-create",
            "voter-write",
            "voter-delete",
            "voter-import",
            "voter-export",
        ],
        eventScope: true,
    },
    beforeEach: () => {
        boundary = graphqlBoundary({
            GetUserProfileConfiguration: () => ({
                data: {
                    get_user_profile_configuration: {
                        attributes: [
                            {
                                name: "username",
                                display_name: "Username",
                                multivalued: false,
                                annotations: {},
                                validations: {},
                                group: null,
                                required: null,
                                permissions: null,
                                selector: null,
                            },
                        ],
                        groups: [],
                    },
                },
            }),
            GetUploadUrl: () => ({
                data: {get_upload_url: {url: UPLOAD_URL, document_id: DOCUMENT_ID}},
            }),
            ImportUsers: () => ({
                data: {
                    import_users: {
                        task_execution: {
                            id: "task-import",
                            name: "import users",
                            execution_status: "IN_PROGRESS",
                            created_at: null,
                            start_at: null,
                            end_at: null,
                            logs: null,
                            annotations: null,
                            labels: null,
                            executed_by_user: null,
                            tenant_id: TENANT_ID,
                            election_event_id: EVENT_ID,
                            type: "IMPORT_USERS",
                        },
                    },
                },
            }),
        })
        data = dataBoundary({
            getList: async <RecordType extends RaRecord>(resource: string) => {
                if (resource === "user") return {data: [user] as unknown as RecordType[], total: 1}
                if (resource === "role") return {data: [] as RecordType[], total: 0}
                data.unexpected.push(resource)
                throw new Error("Unexpected list resource")
            },
            getOne: async <RecordType extends RaRecord>(
                resource: string,
                {id}: {id: string | number}
            ) => {
                if (resource !== "sequent_backend_election_event" || id !== EVENT_ID) {
                    data.unexpected.push(`${resource}/${id}`)
                    throw new Error("Unexpected event read")
                }
                return {data: event as unknown as RecordType}
            },
        })
        const uploads: string[] = []
        const fetch = spyOn(window, "fetch").mockImplementation(async (url, init) => {
            if (url !== UPLOAD_URL || init?.method !== "PUT") {
                uploads.push(`${init?.method} ${url}`)
                throw new Error("Unexpected upload")
            }
            return new Response(null, {status: 200})
        })
        const services = boundary
        const records = data
        return () => {
            try {
                expect(services.unexpected).toEqual([])
                expect(records.unexpected).toEqual([])
                expect(uploads).toEqual([])
            } finally {
                fetch.mockRestore()
            }
        }
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>
async function loaded(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await canvas.findByText("alice")
    expect(data.calls.find((call) => call.args[0] === "user")?.args[1]).toMatchObject({
        filter: {tenant_id: TENANT_ID},
    })
    return canvas
}
export const PermittedActionsAreVisible: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        for (const name of ["Add", "Import", "Export"])
            expect(canvas.getByRole("button", {name})).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Actions"}))
        const menu = within(await within(document.body).findByRole("menu"))
        await waitFor(() => expect(menu.getByRole("menuitem", {name: "Edit"})).toBeVisible())
        expect(menu.getByRole("menuitem", {name: "Delete"})).toBeVisible()
        expect(menu.queryByRole("menuitem", {name: "Send Communication"})).not.toBeInTheDocument()
        await userEvent.keyboard("{Escape}")
        await waitFor(() =>
            expect(within(document.body).queryByRole("menu")).not.toBeInTheDocument()
        )
    },
}
export const RestrictedActionsAreHidden: Story = {
    args: {permissions: []},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        for (const name of ["Add", "Import", "Export"])
            expect(canvas.queryByRole("button", {name})).not.toBeInTheDocument()
        expect(canvas.queryByRole("button", {name: "Actions"})).not.toBeInTheDocument()
        expect(boundary.calls.map((call) => call.name)).toEqual(["GetUserProfileConfiguration"])
    },
}
export const UserImportDoesNotGrantVoterImport: Story = {
    args: {permissions: ["user-import"]},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        expect(canvas.queryByRole("button", {name: "Import"})).not.toBeInTheDocument()
    },
}
export const VoterImportDoesNotGrantTenantImport: Story = {
    args: {permissions: ["voter-import"], eventScope: false},
    parameters: {router: {initialEntries: ["/user-roles"]}},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        expect(canvas.queryByRole("button", {name: "Import"})).not.toBeInTheDocument()
    },
}
async function importFile(canvasElement: HTMLElement, eventScope: boolean) {
    const canvas = await loaded(canvasElement)
    await userEvent.click(canvas.getByRole("button", {name: "Import"}))
    const body = within(document.body)
    await userEvent.type(
        await body.findByRole("textbox", {name: "Integrity Check (SHA-256)"}),
        CHECKSUM
    )
    const input = document.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("Import drawer file picker is missing")
    await userEvent.upload(
        input,
        new File(['[{"username":"alice"}]'], "users.json", {type: "application/json"})
    )
    await waitFor(() => expect(body.getByRole("button", {name: "Import"})).toBeEnabled())
    await userEvent.click(body.getByRole("button", {name: "Import"}))
    await waitFor(() =>
        expect(boundary.calls.filter((call) => call.name === "ImportUsers")).toHaveLength(1)
    )
    expect(boundary.calls.find((call) => call.name === "ImportUsers")?.variables).toEqual({
        tenantId: TENANT_ID,
        documentId: DOCUMENT_ID,
        electionEventId: eventScope ? EVENT_ID : undefined,
        sha256: CHECKSUM,
    })
    await waitFor(() =>
        expect(
            body.queryByRole("textbox", {name: "Integrity Check (SHA-256)"})
        ).not.toBeInTheDocument()
    )
}
export const VoterImportIncludesEventScope: Story = {
    args: {permissions: ["voter-import"]},
    play: ({canvasElement}) => importFile(canvasElement, true),
}
export const TenantUserImportHasNoEventScope: Story = {
    args: {permissions: ["user-import"], eventScope: false},
    parameters: {router: {initialEntries: ["/user-roles"]}},
    play: ({canvasElement}) => importFile(canvasElement, false),
}
