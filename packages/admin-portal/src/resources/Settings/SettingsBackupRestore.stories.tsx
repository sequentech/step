// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    type RecordedDownload,
} from "@/__stories__/downloads"
import {storyId} from "@/__stories__/fixtures"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    startedTask,
    taskHandler,
    taskWidgetDefects,
} from "@/components/tally/__stories__/DownloadFixture"
import {TenantContext} from "@/providers/TenantContextProvider"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {SettingsBackupRestore} from "./SettingsBackupRestore"
import {settingsTab} from "./__stories__/SettingsFixture"

interface Scenario {
    /** Whether the tenant configuration services fail. */
    failure: boolean
    /** Whether a tenant is selected. */
    withTenant: boolean
}

const Tab = settingsTab(SettingsBackupRestore)
const BACKUP_ID = storyId(6, 1)
const BACKUP_TASK_ID = storyId(6, 2)
const RESTORE_ID = storyId(6, 3)
const RESTORE_TASK_ID = storyId(6, 4)
const UPLOAD_URL = "https://s3.admin-story.invalid/upload/tenant-config"
const CHECKSUM = "ab".repeat(32)

let graphql: ReturnType<typeof graphqlBoundary>
let uploads: ReturnType<typeof storyFetch>
let downloads: RecordedDownload[]

const headingOrder = {
    reason: "The backup and restore subtitles are h6 headings below the h4 title.",
    a11y: ["heading-order"],
}

/** The task widget's defects on top of the heading order. */
const withTaskWidget = (status: "SUCCESS" | "FAILED") => {
    const {expectedFailure} = taskWidgetDefects(status)
    return {
        expectedFailure: {
            reason: `${headingOrder.reason} ${expectedFailure.reason}`,
            a11y: [...headingOrder.a11y, ...expectedFailure.a11y],
        },
    }
}

const meta = {
    title: "Admin/Settings/SettingsBackupRestore",
    component: SettingsBackupRestore,
    args: {failure: false, withTenant: true},
    parameters: {
        widgets: ["StyledDivider"],
        expectedFailure: headingOrder,
    },
    beforeEach: async ({args}) => {
        const failure = () => {
            throw new Error("Synthetic tenant configuration service unavailable")
        }
        graphql = graphqlBoundary(
            {
                ExportTenantConfig: () =>
                    args.failure
                        ? failure()
                        : {
                              data: {
                                  export_tenant_config: {
                                      error_msg: null,
                                      document_id: BACKUP_ID,
                                      task_execution: startedTask(
                                          BACKUP_TASK_ID,
                                          "EXPORT_TENANT_CONFIG"
                                      ),
                                  },
                              },
                          },
                ImportTenantConfig: () =>
                    args.failure
                        ? failure()
                        : {
                              data: {
                                  import_tenant_config: {
                                      message: null,
                                      error: null,
                                      task_execution: startedTask(
                                          RESTORE_TASK_ID,
                                          "IMPORT_TENANT_CONFIG"
                                      ),
                                  },
                              },
                          },
                GetUploadUrl: () => ({
                    data: {get_upload_url: {url: UPLOAD_URL, document_id: RESTORE_ID}},
                }),
                ...taskHandler(args.failure ? "FAILED" : "SUCCESS", "EXPORT_TENANT_CONFIG"),
                ...documentHandlers({[BACKUP_ID]: {name: "tenant-config.zip"}}),
            },
            {schema: true}
        )
        await graphql.ready
        uploads = storyFetch({[UPLOAD_URL]: () => ({status: 200})})
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({withTenant}) => (
        <AdminStoryProvider boundary={graphql}>
            <TenantContext.Provider
                value={{
                    tenantId: withTenant ? TENANT_ID : null,
                    setTenantId: () => {},
                    setTenant: () => {},
                }}
            >
                <WidgetsContextProvider>
                    <Tab />
                </WidgetsContextProvider>
            </TenantContext.Provider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const backupButton = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {name: i18n.t("settings.backupRestore.backup.label")})
const restoreButton = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {
        name: i18n.t("settings.backupRestore.restore.label"),
    })

const OPTIONS = ["tenantConfigOption", "keycloakConfigOption", "RolesConfigOption"]

async function chooseOptions(canvasElement: HTMLElement, options: string[]) {
    for (const option of options) {
        await userEvent.click(
            within(canvasElement).getByRole("checkbox", {
                name: i18n.t(`settings.backupRestore.restore.${option}`),
            })
        )
    }
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("settings.backupRestore.title"))
        ).toBeVisible()
        await expect(backupButton(canvasElement)).toBeEnabled()
        await expect(restoreButton(canvasElement)).toBeDisabled()
        expect(graphql.calls).toEqual([])
    },
}

export const BackUpTheTenant: Story = {
    parameters: withTaskWidget("SUCCESS"),
    play: async ({canvasElement}) => {
        await userEvent.click(backupButton(canvasElement))
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: `tenant-config-${TENANT_ID}-export.zip`, href: documentUrl(BACKUP_ID)},
            ])
        )
        expect(graphql.calls[0]).toEqual({
            name: "ExportTenantConfig",
            variables: {tenantId: TENANT_ID},
            headers: {"x-hasura-role": "tenant-read"},
        })
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
        await waitFor(() => expect(backupButton(canvasElement)).toBeEnabled())
    },
}

export const BackupFailure: Story = {
    args: {failure: true},
    parameters: withTaskWidget("FAILED"),
    play: async ({canvasElement}) => {
        await userEvent.click(backupButton(canvasElement))
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["ExportTenantConfig"])
        expect(downloads).toEqual([])
        await expect(backupButton(canvasElement)).toBeEnabled()
    },
}

export const RestoreNeedsEveryOption: Story = {
    play: async ({canvasElement}) => {
        await chooseOptions(canvasElement, OPTIONS.slice(0, 2))
        await expect(restoreButton(canvasElement)).toBeDisabled()
        await chooseOptions(canvasElement, OPTIONS.slice(2))
        await expect(restoreButton(canvasElement)).toBeEnabled()
        expect(within(document.body).queryByRole("presentation")).toBeNull()
    },
}

async function restoreFromAFile(canvasElement: HTMLElement) {
    await chooseOptions(canvasElement, OPTIONS)
    await userEvent.click(restoreButton(canvasElement))
    const drawerElement = await within(document.body).findByRole("presentation")
    const drawer = within(drawerElement)
    await expect(
        await drawer.findByText(i18n.t("settings.backupRestore.restore.title"))
    ).toBeVisible()
    await userEvent.type(drawer.getByRole("textbox", {name: "Integrity Check (SHA-256)"}), CHECKSUM)
    const input = drawerElement.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("The import file input is missing")
    await userEvent.upload(input, new File(["{}"], "tenant-config.zip", {type: "application/zip"}))
    await waitFor(() => expect(uploads.calls.map(({method}) => method)).toEqual(["PUT"]))
    const importButton = drawer.getByRole("button", {name: "Import"})
    await waitFor(() => expect(importButton).toBeEnabled())
    await userEvent.click(importButton)
}

export const RestoreTheTenant: Story = {
    parameters: withTaskWidget("SUCCESS"),
    play: async ({canvasElement}) => {
        await restoreFromAFile(canvasElement)
        await waitFor(() =>
            expect(graphql.calls.find(({name}) => name === "ImportTenantConfig")).toEqual({
                name: "ImportTenantConfig",
                variables: {
                    tenantId: TENANT_ID,
                    documentId: RESTORE_ID,
                    importConfigurations: {
                        include_tenant: true,
                        include_keycloak: true,
                        include_roles: true,
                    },
                    sha256: CHECKSUM,
                },
                headers: {"x-hasura-role": "tenant-write"},
            })
        )
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
        await waitFor(() => expect(within(document.body).queryByRole("presentation")).toBeNull())
    },
}

export const RestoreFailure: Story = {
    args: {failure: true},
    parameters: withTaskWidget("FAILED"),
    play: async ({canvasElement}) => {
        await restoreFromAFile(canvasElement)
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toContain("ImportTenantConfig")
        await waitFor(() => expect(restoreButton(canvasElement)).toBeEnabled())
    },
}

export const WithoutATenant: Story = {
    args: {withTenant: false},
    // The empty state has no divider.
    parameters: {widgets: [], expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("electionTypeScreen.common.emptyHeader"))
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("button")).toBeNull()
    },
}
