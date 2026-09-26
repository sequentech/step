// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {RecordContextProvider, type RaRecord} from "react-admin"
import {FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {STORY_TRUSTEE} from "@/__stories__/storyAuth"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
} from "@/services/KeyCeremony"
import {EditElectionEventKeys} from "./EditElectionEventKeys"
import {
    EStoryPermissions,
    EStoryTenant,
    EStoryWorkflow,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"
import {EStoryDataState, pending} from "../../../../ui-essentials/.storybook/screens"

const KEYS_ID = "44444444-4444-4444-8444-444444444444"
const trustees = [STORY_TRUSTEE, "trustee2", "trustee3"].map((name, index) => ({
    id: `55555555-5555-4555-8555-55555555555${index}`,
    tenant_id: TENANT_ID,
    name,
}))
const event = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    presentation: {i18n: {en: {name: "Council event"}}, ceremonies_policy: "manual-ceremonies"},
}

/**
 * The event's keys ceremony: while it runs, only trustee2 has checked its key,
 * so the story trustee is still invited; afterwards it has completed.
 */
function keysCeremony(workflow: EStoryWorkflow) {
    const generating = workflow === EStoryWorkflow.CREATED
    return {
        id: KEYS_ID,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        trustee_ids: trustees.map(({id}) => id),
        execution_status: generating ? EStatus.IN_PROGRESS : EStatus.SUCCESS,
        status: {
            public_key: generating ? undefined : "synthetic-public-key",
            trustees: trustees.map(({name}, index) => ({
                name,
                status: generating && index !== 1 ? TStatus.WAITING : TStatus.KEY_CHECKED,
            })),
            logs: [],
        },
        labels: {},
        annotations: {},
        threshold: 2,
        name: "Council key",
        settings: {policy: "manual-ceremonies"},
        is_default: true,
        permission_label: null,
    }
}

const ceremonyListDefects = {
    expectedFailure: {
        reason: "React-admin row selection labels a MUI 7 span instead of its checkbox, row actions are unnamed icon buttons and status chips have white text below 4.5 contrast.",
        a11y: ["aria-prohibited-attr", "button-name", "color-contrast", "label"],
    },
}
const emptyStateDefect = {
    expectedFailure: {
        reason: "The empty state's create button contains an icon button.",
        a11y: ["nested-interactive"],
    },
}

interface Scenario {
    data: EStoryDataState
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>

function KeysScreen() {
    const {permissions, tenant} = useStoryGlobals()
    const settings = useContext(SettingsContext)
    return (
        <AdminStoryProvider
            boundary={boundary}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <SettingsContext.Provider
                value={{
                    ...settings,
                    globalSettings: {
                        ...settings.globalSettings,
                        QUERY_FAST_POLL_INTERVAL_MS: 3_600_000,
                    },
                }}
            >
                <RecordContextProvider value={event}>
                    <EditElectionEventKeys />
                </RecordContextProvider>
            </SettingsContext.Provider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Screens/Admin/Keys ceremony",
    component: KeysScreen,
    args: {data: EStoryDataState.POPULATED},
    beforeEach: ({args, globals}) => {
        const ceremonies = {
            [EStoryDataState.LOADING]: [],
            [EStoryDataState.EMPTY]: [],
            [EStoryDataState.POPULATED]: [keysCeremony(readStoryGlobals(globals).workflow)],
            [EStoryDataState.ERROR]: [],
        }[args.data]
        const failure = new Error("Synthetic keys service unavailable")
        boundary = graphqlBoundary({
            ListKeysCeremony: () => {
                if (args.data === EStoryDataState.LOADING) return pending()
                if (args.data === EStoryDataState.ERROR) throw failure
                return {
                    data: {
                        list_keys_ceremony: {
                            items: ceremonies,
                            total: {aggregate: {count: ceremonies.length}},
                        },
                    },
                }
            },
            TrusteeNames: () => ({data: {sequent_backend_trustee: trustees}}),
        })
        data = dataBoundary({
            getList: async <RecordType extends RaRecord>(resource: string) => {
                if (resource !== "sequent_backend_keys_ceremony") {
                    data.unexpected.push(resource)
                    throw new Error(`Unexpected list ${resource}`)
                }
                if (args.data === EStoryDataState.LOADING) return pending()
                if (args.data === EStoryDataState.ERROR) throw failure
                return {data: ceremonies as unknown as RecordType[], total: ceremonies.length}
            },
        })
        const services = boundary
        const records = data
        return () => {
            expect(services.unexpected).toEqual([])
            expect(records.unexpected).toEqual([])
        }
    },
    render: (_args, {globals}) => <KeysScreen key={JSON.stringify(globals)} />,
} satisfies Meta<Scenario>
export default meta
type Story = StoryObj<typeof meta>

export const Loading: Story = {
    args: {data: EStoryDataState.LOADING},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(boundary.calls.map(({name}) => name)).toContain("ListKeysCeremony")
        )
        expect(canvas.queryByRole("row", {name: /Council key/})).not.toBeInTheDocument()
    },
}

export const Empty: Story = {
    args: {data: EStoryDataState.EMPTY},
    parameters: emptyStateDefect,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No Key Ceremony yet.")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Create Key Ceremony"})).toBeEnabled()
    },
}

export const Populated: Story = {
    parameters: ceremonyListDefects,
    play: async ({canvasElement, globals}) => {
        const {workflow, permissions} = readStoryGlobals(globals)
        const canvas = within(canvasElement)
        const row = await canvas.findByRole("row", {name: /Council key/})
        await expect(within(row).getByText(keysCeremony(workflow).execution_status)).toBeVisible()
        expect(boundary.calls[0]).toEqual({
            name: "ListKeysCeremony",
            variables: {tenantId: TENANT_ID, electionEventId: EVENT_ID},
            headers: {
                "x-hasura-role":
                    permissions === EStoryPermissions.TRUSTEE
                        ? "trustee-ceremony"
                        : "admin-ceremony",
            },
        })
    },
}

export const LoadError: Story = {
    args: {data: EStoryDataState.ERROR},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const notification = await within(document.body).findByText(
            "Synthetic keys service unavailable"
        )
        await waitFor(() => expect(notification).toBeVisible())
        expect(canvas.queryByRole("row", {name: /Council key/})).not.toBeInTheDocument()
    },
}

export const TrusteeInvitation: Story = {
    globals: {permissions: EStoryPermissions.TRUSTEE, workflow: EStoryWorkflow.CREATED},
    parameters: ceremonyListDefects,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(/You have been invited to participate in a Key Ceremony/)
        ).toBeVisible()
        const row = await canvas.findByRole("row", {name: /Council key/})
        await expect(within(row).getByText(EStatus.IN_PROGRESS)).toBeVisible()
        expect(boundary.calls[0].headers).toEqual({"x-hasura-role": "trustee-ceremony"})
    },
}

export const CustomBranding: Story = {
    args: {data: EStoryDataState.EMPTY},
    parameters: emptyStateDefect,
    globals: {tenant: EStoryTenant.CUSTOM},
    play: async ({canvasElement}) => {
        const heading = await within(canvasElement).findByText("No Key Ceremony yet.")
        await waitFor(() => expect(getComputedStyle(heading).color).toBe("rgb(11, 79, 58)"))
    },
}
