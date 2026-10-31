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
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    startedTask,
    taskHandler,
    taskWidgetDefects,
} from "@/components/tally/__stories__/DownloadFixture"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {IPermissions} from "@/types/keycloak"
import {TemplateList} from "./TemplateList"
import {
    RECEIPT_TEMPLATE_ID,
    TEMPLATE_RESOURCE,
    templateRecords,
} from "./__stories__/TemplateFixture"

interface Scenario {
    /** What reading the templates does. */
    reads: ReadState
    /** Whether the tenant has templates. */
    populated: boolean
    /** Whether the export and import services fail. */
    failure: boolean
    /** The signed-in user's roles. */
    roles: string[]
}

const READ_ROLES = [IPermissions.template_READ, IPermissions.TEMPLATES_MENU]
const EXPORT_ID = storyId(7, 9)
const IMPORT_ID = storyId(7, 10)
const TASK_ID = storyId(7, 11)
const UPLOAD_URL = "https://s3.admin-story.invalid/upload/templates"
const CHECKSUM = "cd".repeat(32)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let uploads: ReturnType<typeof storyFetch>
let downloads: RecordedDownload[]

const gridDefects = {
    reason: "The grid's row checkboxes are unlabelled and the rows' edit and delete actions are unnamed icon buttons.",
    a11y: ["aria-prohibited-attr", "button-name", "label"],
}

const withTaskWidget = (status: "SUCCESS" | "FAILED") => {
    const {expectedFailure} = taskWidgetDefects(status)
    return {
        expectedFailure: {
            reason: `${gridDefects.reason} ${expectedFailure.reason}`,
            a11y: Array.from(new Set([...gridDefects.a11y, ...expectedFailure.a11y])),
        },
    }
}

const OPEN_DRAWER = {
    expectedFailure: {
        reason: "The form drawer has no accessible name.",
        a11y: ["aria-dialog-name"],
    },
}

const meta = {
    title: "Admin/Template/TemplateList",
    component: TemplateList,
    args: {
        reads: "records",
        populated: true,
        failure: false,
        roles: [...READ_ROLES, IPermissions.template_WRITE],
    },
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {expectedFailure: gridDefects},
    beforeEach: async ({args}) => {
        const failure = () => {
            throw new Error("Synthetic template service unavailable")
        }
        data = resourceBoundary(
            {[TEMPLATE_RESOURCE]: args.populated ? templateRecords() : []},
            {reads: args.reads}
        )
        graphql = graphqlBoundary(
            {
                ExportTemplate: () =>
                    args.failure
                        ? failure()
                        : {
                              data: {
                                  export_template: {
                                      error_msg: null,
                                      document_id: EXPORT_ID,
                                      task_execution: startedTask(TASK_ID, "EXPORT_TEMPLATES"),
                                  },
                              },
                          },
                ImportTemplates: () =>
                    args.failure
                        ? failure()
                        : {
                              data: {
                                  import_templates: {
                                      error_msg: null,
                                      document_id: IMPORT_ID,
                                      task_execution: startedTask(TASK_ID, "IMPORT_TEMPLATES"),
                                  },
                              },
                          },
                GetUploadUrl: () => ({
                    data: {get_upload_url: {url: UPLOAD_URL, document_id: IMPORT_ID}},
                }),
                ...taskHandler(args.failure ? "FAILED" : "SUCCESS", "EXPORT_TEMPLATES"),
                ...documentHandlers({[EXPORT_ID]: {name: "templates.csv"}}),
            },
            {schema: true}
        )
        await graphql.ready
        uploads = storyFetch({[UPLOAD_URL]: () => ({status: 200})})
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={roles}
            // The create form opens its own GraphQL client for the signed-in session.
            auth={{tenantId: TENANT_ID, isAuthenticated: true, getAccessToken: () => "story-token"}}
        >
            <WidgetsContextProvider>
                <TemplateList />
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const templateRow = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

const button = (canvasElement: HTMLElement, label: string) =>
    within(canvasElement).queryByRole("button", {name: i18n.t(label)})

async function drawer() {
    const drawers = await within(document.body).findAllByRole("presentation")
    const element = drawers[drawers.length - 1]
    await waitFor(() => expect(element).toBeVisible())
    return element
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const row = within(await templateRow(canvasElement, "Voter credentials"))
        await expect(row.getByText("voter-credentials")).toBeVisible()
        await expect(row.getByText("CREDENTIALS")).toBeVisible()
        await expect(await templateRow(canvasElement, "Ballot receipt")).toBeVisible()
        for (const label of ["common.label.add", "common.label.import", "common.label.export"]) {
            await expect(button(canvasElement, label)).toBeVisible()
        }
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getList", TEMPLATE_RESOURCE],
        ])
        expect(graphql.calls).toEqual([])
    },
}

export const ReadOnly: Story = {
    args: {roles: READ_ROLES},
    parameters: {
        expectedFailure: {
            reason: "The grid's row checkboxes are unlabelled.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    play: async ({canvasElement}) => {
        const row = within(await templateRow(canvasElement, "Voter credentials"))
        expect(row.queryAllByRole("button")).toEqual([])
        expect(button(canvasElement, "common.label.add")).toBeNull()
        expect(button(canvasElement, "common.label.import")).toBeNull()
        await expect(button(canvasElement, "common.label.export")).toBeVisible()
    },
}

export const WithoutTemplatePermissions: Story = {
    args: {roles: [IPermissions.template_READ]},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("template.noPermissions"))
        ).toBeVisible()
        expect(data.calls).toEqual([])
    },
}

export const Empty: Story = {
    args: {populated: false},
    parameters: {
        expectedFailure: {
            reason: "The create button nests an icon button inside it.",
            a11y: ["nested-interactive"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(i18n.t("template.empty.title"))).toBeVisible()
        await expect(canvas.getByRole("button", {name: /Create Template/})).toBeVisible()
        await expect(button(canvasElement, "common.label.import")).toBeVisible()
    },
}

export const OpenTheCreateForm: Story = {
    parameters: OPEN_DRAWER,
    play: async ({canvasElement}) => {
        await templateRow(canvasElement, "Voter credentials")
        await userEvent.click(button(canvasElement, "common.label.add") as HTMLElement)
        const form = within(await drawer())
        await expect(await form.findByText(i18n.t("template.create.title"))).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

export const OpenTheEditForm: Story = {
    parameters: OPEN_DRAWER,
    play: async ({canvasElement}) => {
        const [edit] = within(await templateRow(canvasElement, "Ballot receipt")).getAllByRole(
            "button"
        )
        await userEvent.click(edit)
        const form = within(await drawer())
        await expect(await form.findByText(i18n.t("template.edit.title"))).toBeVisible()
        await waitFor(() =>
            expect(form.getByRole("textbox", {name: /Template Alias/})).toHaveValue(
                "ballot-receipt"
            )
        )
        expect(data.calls).toContainEqual({
            method: "getOne",
            args: [TEMPLATE_RESOURCE, expect.objectContaining({id: RECEIPT_TEMPLATE_ID})],
        })
    },
}

export const DeleteATemplate: Story = {
    play: async ({canvasElement}) => {
        const buttons = within(await templateRow(canvasElement, "Ballot receipt")).getAllByRole(
            "button"
        )
        await userEvent.click(buttons[buttons.length - 1])
        const dialog = within(await within(document.body).findByRole("dialog"))
        expect(data.writes).toEqual([])
        await userEvent.click(dialog.getByRole("button", {name: i18n.t("common.label.delete")}))
        await waitFor(() =>
            expect(
                data.writes.map(({method, resource, params}) => [method, resource, params.id])
            ).toEqual([["delete", TEMPLATE_RESOURCE, RECEIPT_TEMPLATE_ID]])
        )
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("row", {name: /Ballot receipt/})).toBeNull()
        )
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

async function confirmExport(canvasElement: HTMLElement) {
    await templateRow(canvasElement, "Voter credentials")
    await userEvent.click(button(canvasElement, "common.label.export") as HTMLElement)
    const dialog = within(await within(document.body).findByRole("dialog"))
    await expect(dialog.getByText(i18n.t("common.export"))).toBeVisible()
    expect(graphql.calls).toEqual([])
    await userEvent.click(dialog.getByRole("button", {name: i18n.t("common.label.export")}))
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
}

export const ExportTheTemplates: Story = {
    parameters: withTaskWidget("SUCCESS"),
    play: async ({canvasElement}) => {
        await confirmExport(canvasElement)
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "templates-export.csv", href: documentUrl(EXPORT_ID)},
            ])
        )
        expect(graphql.calls[0]).toEqual({
            name: "ExportTemplate",
            variables: {tenantId: TENANT_ID},
            headers: {},
        })
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
    },
}

export const ExportFailure: Story = {
    args: {failure: true},
    parameters: withTaskWidget("FAILED"),
    play: async ({canvasElement}) => {
        await confirmExport(canvasElement)
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["ExportTemplate"])
        expect(downloads).toEqual([])
    },
}

async function importAFile(canvasElement: HTMLElement) {
    await templateRow(canvasElement, "Voter credentials")
    await userEvent.click(button(canvasElement, "common.label.import") as HTMLElement)
    const element = await drawer()
    const form = within(element)
    await expect(await form.findByText(i18n.t("template.import.title"))).toBeVisible()
    await userEvent.type(form.getByRole("textbox", {name: "Integrity Check (SHA-256)"}), CHECKSUM)
    const input = element.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("The import file input is missing")
    await userEvent.upload(input, new File(["alias,name"], "templates.csv", {type: "text/csv"}))
    await waitFor(() => expect(uploads.calls.map(({method}) => method)).toEqual(["PUT"]))
    const importButton = form.getByRole("button", {name: "Import"})
    await waitFor(() => expect(importButton).toBeEnabled())
    await userEvent.click(importButton)
}

export const ImportTemplates: Story = {
    parameters: withTaskWidget("SUCCESS"),
    play: async ({canvasElement}) => {
        await importAFile(canvasElement)
        await waitFor(() =>
            expect(graphql.calls.find(({name}) => name === "ImportTemplates")).toEqual({
                name: "ImportTemplates",
                variables: {tenantId: TENANT_ID, documentId: IMPORT_ID, sha256: CHECKSUM},
                headers: {},
            })
        )
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
        // The list is read again for the imported templates.
        await waitFor(() =>
            expect(data.calls.filter(({method}) => method === "getList").length).toBeGreaterThan(1)
        )
    },
}

export const ImportFailure: Story = {
    args: {failure: true},
    parameters: withTaskWidget("FAILED"),
    play: async ({canvasElement}) => {
        await importAFile(canvasElement)
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toContain("ImportTemplates")
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getList"))
        expect(within(canvasElement).queryByRole("row", {name: /Voter credentials/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /Voter credentials/})).toBeNull()
    },
}
