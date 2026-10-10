// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    type RecordedDownload,
} from "@/__stories__/downloads"
import {storyId} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    startedTask,
    taskHandler,
    taskWidgetDefects,
} from "@/components/tally/__stories__/DownloadFixture"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {IPermissions} from "@/types/keycloak"
import {SettingsTrustees} from "./SettingsTrustees"
import {
    SECOND_TRUSTEE_ID,
    TENANT_RESOURCE,
    TRUSTEE_RESOURCE,
    settingsTab,
    settingsTenant,
    trusteeRecords,
} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the trustees does. */
    reads: ReadState
    /** Whether the tenant has trustees. */
    populated: boolean
    /** Whether the export service fails. */
    exportFailure: boolean
    /** The signed-in user's roles. */
    roles: string[]
}

const Tab = settingsTab(SettingsTrustees)
const EXPORT_ID = storyId(4, 9)
const EXPORT_TASK_ID = storyId(4, 10)
const ALL_ROLES = [
    IPermissions.TRUSTEE_READ,
    IPermissions.TRUSTEE_WRITE,
    IPermissions.TRUSTEES_EXPORT,
]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let downloads: RecordedDownload[]

const gridDefects = {
    reason: "The grid's row checkboxes are unlabelled, its actions column has an empty header, and the rows' edit and delete actions are unnamed icon buttons.",
    a11y: ["aria-prohibited-attr", "button-name", "empty-table-header", "label"],
}

const meta = {
    title: "Admin/Settings/SettingsTrustees",
    component: SettingsTrustees,
    args: {reads: "records", populated: true, exportFailure: false, roles: ALL_ROLES},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {expectedFailure: gridDefects},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {
                [TRUSTEE_RESOURCE]: args.populated ? trusteeRecords() : [],
                [TENANT_RESOURCE]: [settingsTenant()],
            },
            {reads: {[TRUSTEE_RESOURCE]: args.reads}}
        )
        graphql = graphqlBoundary(
            {
                ExportTrustees: () => {
                    if (args.exportFailure) throw new Error("Synthetic export service unavailable")
                    return {
                        data: {
                            exportTrustees: {
                                document_id: EXPORT_ID,
                                task_execution: startedTask(EXPORT_TASK_ID, "EXPORT_TRUSTEES"),
                            },
                        },
                    }
                },
                ...taskHandler("SUCCESS", "EXPORT_TRUSTEES"),
                ...documentHandlers({[EXPORT_ID]: {name: "trustees-export.ezip"}}),
            },
            {schema: true}
        )
        await graphql.ready
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={roles}
            auth={{tenantId: TENANT_ID}}
        >
            <WidgetsContextProvider>
                <ResourceContextProvider value={TRUSTEE_RESOURCE}>
                    <Tab />
                </ResourceContextProvider>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const trusteeRow = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

const button = (canvasElement: HTMLElement, label: string) =>
    within(canvasElement).queryByRole("button", {name: i18n.t(label)})

/** The topmost open drawer. */
async function drawer() {
    const drawers = await within(document.body).findAllByRole("presentation")
    const element = drawers[drawers.length - 1]
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

async function drawersClosed() {
    await waitFor(() => expect(within(document.body).queryByRole("presentation")).toBeNull())
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const row = within(await trusteeRow(canvasElement, "trustee1"))
        // The public key column is hidden until chosen.
        expect(row.queryByText("Q2VydGlmaWVkIGtleSBvbmU")).toBeNull()
        await expect(await trusteeRow(canvasElement, "trustee2")).toBeVisible()
        await expect(button(canvasElement, "common.label.add")).toBeVisible()
        await expect(button(canvasElement, "common.label.export")).toBeEnabled()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getList", TRUSTEE_RESOURCE],
        ])
        expect(graphql.calls).toEqual([])
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.TRUSTEE_READ]},
    play: async ({canvasElement}) => {
        await trusteeRow(canvasElement, "trustee1")
        expect(button(canvasElement, "common.label.add")).toBeNull()
        expect(button(canvasElement, "common.label.export")).toBeNull()
    },
}

export const WithoutTrusteeReadPermission: Story = {
    args: {roles: [IPermissions.TRUSTEE_WRITE]},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                i18n.t("trusteesSettingsScreen.common.emptyHeader")
            )
        ).toBeVisible()
        await expect(button(canvasElement, "trusteesSettingsScreen.common.createNew")).toBeVisible()
        expect(data.calls).toEqual([])
    },
}

export const Empty: Story = {
    args: {populated: false},
    parameters: {
        expectedFailure: {
            reason: "The create drawer has no accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                i18n.t("trusteesSettingsScreen.common.emptyHeader")
            )
        ).toBeVisible()
        await userEvent.click(
            button(canvasElement, "trusteesSettingsScreen.common.createNew") as HTMLElement
        )
        const form = await drawer()
        await expect(form.getByText(i18n.t("trusteesSettingsScreen.create.title"))).toBeVisible()
    },
}

export const CreateATrustee: Story = {
    play: async ({canvasElement}) => {
        await trusteeRow(canvasElement, "trustee2")
        await userEvent.click(button(canvasElement, "common.label.add") as HTMLElement)
        const form = await drawer()
        await expect(form.getByText(i18n.t("trusteesSettingsScreen.create.title"))).toBeVisible()
        // Only one create form opens.
        expect(
            within(document.body).getAllByText(i18n.t("trusteesSettingsScreen.create.title"))
        ).toHaveLength(1)
        await userEvent.type(form.getByRole("textbox", {name: "Name"}), "trustee3")
        await userEvent.type(form.getByRole("textbox", {name: "Public key"}), "a2V5IHRocmVl")
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "create",
                    resource: TRUSTEE_RESOURCE,
                    params: {
                        data: {name: "trustee3", public_key: "a2V5IHRocmVl", tenant_id: TENANT_ID},
                    },
                },
            ])
        )
        await expect(await trusteeRow(canvasElement, "trustee3")).toBeVisible()
        await drawersClosed()
    },
}

export const RenameATrustee: Story = {
    play: async ({canvasElement}) => {
        const [edit] = within(await trusteeRow(canvasElement, "trustee2")).getAllByRole("button")
        await userEvent.click(edit)
        const form = await drawer()
        await expect(
            await form.findByText(i18n.t("trusteesSettingsScreen.edit.title"))
        ).toBeVisible()
        const name = form.getByRole("textbox", {name: "Name"})
        await waitFor(() => expect(name).toHaveValue("trustee2"))
        await userEvent.clear(name)
        await userEvent.type(name, "trustee-two")
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "update",
                    resource: TRUSTEE_RESOURCE,
                    params: expect.objectContaining({
                        id: SECOND_TRUSTEE_ID,
                        data: expect.objectContaining({name: "trustee-two"}),
                    }),
                },
            ])
        )
        await expect(await trusteeRow(canvasElement, "trustee-two")).toBeVisible()
        await drawersClosed()
    },
}

export const DeleteATrustee: Story = {
    play: async ({canvasElement}) => {
        const buttons = within(await trusteeRow(canvasElement, "trustee2")).getAllByRole("button")
        await userEvent.click(buttons[buttons.length - 1])
        const dialog = within(await within(document.body).findByRole("dialog"))
        expect(data.writes).toEqual([])
        await userEvent.click(dialog.getByRole("button", {name: i18n.t("common.label.delete")}))
        await waitFor(() =>
            expect(
                data.writes.map(({method, resource, params}) => [method, resource, params.id])
            ).toEqual([["delete", TRUSTEE_RESOURCE, SECOND_TRUSTEE_ID]])
        )
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("row", {name: /trustee2/})).toBeNull()
        )
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const ExportTheTrustees: Story = {
    parameters: {
        expectedFailure: {
            reason: `${gridDefects.reason} ${taskWidgetDefects("SUCCESS").expectedFailure.reason}`,
            a11y: [...gridDefects.a11y, "color-contrast", "nested-interactive"],
        },
    },
    play: async ({canvasElement}) => {
        await trusteeRow(canvasElement, "trustee1")
        await userEvent.click(button(canvasElement, "common.label.export") as HTMLElement)
        const dialog = within(
            await within(document.body).findByRole("dialog", {
                name: i18n.t("electionEventScreen.export.passwordTitle"),
            })
        )
        const password = (dialog.getByRole("textbox") as HTMLInputElement).value
        expect(password).not.toBe("")
        expect(graphql.calls[0]).toEqual({
            name: "ExportTrustees",
            variables: {password},
            headers: {"x-hasura-role": IPermissions.TRUSTEES_EXPORT},
        })
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "trustees-export.ezip", href: documentUrl(EXPORT_ID)},
            ])
        )
        // Behind the modal dialog the export stays disabled.
        await expect(
            within(canvasElement).getByRole("button", {
                name: i18n.t("common.label.export"),
                hidden: true,
            })
        ).toBeDisabled()
        await userEvent.click(dialog.getByRole("button", {name: "Ok"}))
        await waitFor(() => expect(button(canvasElement, "common.label.export")).toBeEnabled())
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
    },
}

export const ExportFailure: Story = {
    args: {exportFailure: true},
    parameters: {
        expectedFailure: {
            reason: `${gridDefects.reason} ${taskWidgetDefects("FAILED").expectedFailure.reason}`,
            a11y: [...gridDefects.a11y, "nested-interactive"],
        },
    },
    play: async ({canvasElement}) => {
        await trusteeRow(canvasElement, "trustee1")
        await userEvent.click(button(canvasElement, "common.label.export") as HTMLElement)
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["ExportTrustees"])
        expect(within(document.body).queryByRole("dialog")).toBeNull()
        expect(downloads).toEqual([])
        await expect(button(canvasElement, "common.label.export")).toBeEnabled()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getList"))
        expect(within(canvasElement).queryByRole("row", {name: /trustee1/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /trustee1/})).toBeNull()
    },
}
