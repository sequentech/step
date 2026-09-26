// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {RecordContextProvider} from "react-admin"
import {ETaskExecutionStatus, i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, eventRecord, storyId, type StoryRecord} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {DOCUMENT_ID, taskRecord, widgetDefects} from "@/components/__stories__/WidgetFixture"
import type {Sequent_Backend_Certificate_Authority} from "@/gql/graphql"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {EditElectionEventCAs} from "./EditElectionEventCAs"
import {answerOrPending} from "./__stories__/ElectionEventFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_certificate_authority"
const ROOT_ID = storyId(12, 1)
const INTERMEDIATE_ID = storyId(12, 2)
const ROOT_SUBJECT = "CN=Council Root CA,O=Example Council"
const FINGERPRINT = "3f9a0c1d2e4b5a6978c0d1e2f3a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6"
const PEM = "-----BEGIN CERTIFICATE-----\nc3ludGhldGljIHJvb3Q=\n-----END CERTIFICATE-----\n"

const certificate = (
    overrides: Partial<StoryRecord<Sequent_Backend_Certificate_Authority>>
): StoryRecord<Sequent_Backend_Certificate_Authority> => ({
    id: ROOT_ID,
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    common_name: "Council Root CA",
    subject: ROOT_SUBJECT,
    issuer: ROOT_SUBJECT,
    issuer_common_name: "Council Root CA",
    serial_number: "01",
    fingerprint_sha256: FINGERPRINT,
    not_before: "2025-01-01T00:00:00Z",
    not_after: "2099-01-01T00:00:00Z",
    pem: PEM,
    created_at: FIXED_TIME,
    ...overrides,
})

const CERTIFICATES = [
    certificate({}),
    certificate({
        id: INTERMEDIATE_ID,
        common_name: "Council Voters CA",
        subject: "CN=Council Voters CA,O=Example Council",
        serial_number: "02",
        fingerprint_sha256: undefined,
        not_after: "2020-01-01T00:00:00Z",
        not_before: "2019-01-01T00:00:00Z",
    }),
]

type Reply = "ok" | "error"

interface Scenario {
    /** What reading the certificate authorities does. */
    reads: ReadState
    empty: boolean
    /** Whether the delete, export and import mutations succeed. */
    mutations: Reply
    /** Replaces the group's roles, for a story about one permission. */
    roles?: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

// The event tabs render the section inside the event's record.
function Fixture({roles}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={roles ? undefined : permissions}
            roles={roles}
            tenant={tenant}
        >
            <WidgetsContextProvider>
                <RecordContextProvider value={eventRecord()}>
                    <EditElectionEventCAs />
                </RecordContextProvider>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    )
}

const LIST_DEFECTS =
    "React-admin row selection labels a MUI 7 span instead of its checkbox and the row actions are unnamed icon buttons."

/** The list's defects, and those of whatever else the story's last state shows. */
const defects = (extra = "", rules: string[] = []) => ({
    expectedFailure: {
        reason: extra ? `${LIST_DEFECTS} ${extra}` : LIST_DEFECTS,
        a11y: ["aria-prohibited-attr", "button-name", "label", ...rules],
    },
})

const meta = {
    title: "Admin/Election event/EditElectionEventCAs",
    component: EditElectionEventCAs,
    args: {reads: "records", empty: false, mutations: "ok"},
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        mutations: {control: "inline-radio", options: ["ok", "error"]},
    },
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: defects(),
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[RESOURCE]: args.empty ? [] : CERTIFICATES.map((row) => ({...row}))},
            {reads: args.reads}
        )
        const failed = () => ({errors: [new GraphQLError("Synthetic certificate service failure")]})
        graphql = graphqlBoundary(
            answerOrPending({
                DeleteCertificateAuthority: ({variables}) => {
                    if (args.mutations === "error") return failed()
                    const rows = data.records[RESOURCE]
                    const kept = rows.filter(({id}) => !variables.ids.includes(id))
                    rows.splice(0, rows.length, ...kept)
                    return {
                        data: {delete_certificate_authority: {deleted_count: variables.ids.length}},
                    }
                },
                ExportCertificateAuthority: () =>
                    args.mutations === "error"
                        ? failed()
                        : {
                              data: {
                                  export_certificate_authority: {
                                      document_id: DOCUMENT_ID,
                                      task_execution: taskRecord(ETaskExecutionStatus.IN_PROGRESS, {
                                          name: "Export certificate authorities",
                                          type: ETasksExecution.EXPORT_CERTIFICATE_AUTHORITIES,
                                      }),
                                  },
                              },
                          },
                ImportCertificateAuthority: () =>
                    args.mutations === "error"
                        ? failed()
                        : {
                              data: {
                                  import_certificate_authority: {
                                      inserted_count: 1,
                                      skipped_count: 1,
                                      errors: [],
                                  },
                              },
                          },
            }),
            {schema: true}
        )
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const ca = (key: string, options?: Record<string, unknown>) =>
    i18n.t(`certificateAuthorities.${key}`, options)
// The intermediate certificate's issuer is the root, so the root's row is the one of its type.
const ROOT_ROW = /Council Root CA\s*Root/
const rootRow = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("row", {name: ROOT_ROW})
const operations = (name: string) => graphql.calls.filter((call) => call.name === name)

async function notified(message: string) {
    const notice = await within(document.body).findByText(message, {
        selector: ".MuiSnackbarContent-message",
    })
    await waitFor(() => expect(notice).toBeVisible())
}

async function confirm(button: string, message: string) {
    const dialog = await within(document.body).findByRole("dialog")
    expect(dialog).toHaveTextContent(message)
    await userEvent.click(within(dialog).getByRole("button", {name: button}))
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
}

/** The list's own button; the hidden bulk toolbar has buttons of the same names. */
const shownButton = async (canvasElement: HTMLElement, name: string) => {
    const buttons = await within(canvasElement).findAllByRole("button", {name})
    const shown = buttons.find((button) => button.checkVisibility({visibilityProperty: true}))
    if (!shown) throw new Error(`No ${name} button is shown`)
    return shown
}

async function selectRoot(canvasElement: HTMLElement) {
    const row = await rootRow(canvasElement)
    await userEvent.click(within(row).getByRole("checkbox"))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const root = await rootRow(canvasElement)
        expect(root).toHaveTextContent(ca("type.root"))
        expect(root).toHaveTextContent(ca("expiry.valid"))
        expect(root).toHaveTextContent(FINGERPRINT.slice(0, 24))
        const intermediate = canvas.getByRole("row", {name: /Council Voters CA/})
        expect(intermediate).toHaveTextContent(ca("type.intermediate"))
        expect(intermediate).toHaveTextContent(ca("expiry.expired"))
        await expect(
            canvas.getByRole("button", {name: i18n.t("common.label.import")})
        ).toBeVisible()
        await expect(await shownButton(canvasElement, i18n.t("common.label.export"))).toBeVisible()
        expect(within(root).getAllByRole("button")).toHaveLength(2)
        expect(data.calls.find(({method}) => method === "getList")?.args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID, election_event_id: EVENT_ID},
        })
        expect(graphql.calls).toEqual([])
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(ca("emptyHeader"))).toBeVisible()
        await expect(canvas.getByRole("button", {name: ca("importButton")})).toBeVisible()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(ca("title"))).toBeVisible()
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getList"))
        expect(canvas.queryByRole("row", {name: ROOT_ROW})).toBeNull()
        expect(canvas.queryByText(ca("emptyHeader"))).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await notified("Synthetic service unavailable")
        expect(within(canvasElement).queryByRole("row", {name: ROOT_ROW})).toBeNull()
    },
}

export const ReadOnly: Story = {
    args: {roles: ["ca-read"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const root = await rootRow(canvasElement)
        expect(within(root).getAllByRole("button")).toHaveLength(1)
        expect(canvas.queryByRole("button", {name: i18n.t("common.label.import")})).toBeNull()
        await expect(await shownButton(canvasElement, i18n.t("common.label.export"))).toBeVisible()
    },
}

export const EmptyWithoutWritePermission: Story = {
    args: {empty: true, roles: ["ca-read"]},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(ca("emptyHeader"))).toBeVisible()
        expect(canvas.queryByRole("button", {name: ca("importButton")})).toBeNull()
    },
}

export const ViewACertificate: Story = {
    parameters: {widgets: ["ViewCAContent", "LabelValue"]},
    play: async ({canvasElement}) => {
        const root = await rootRow(canvasElement)
        await userEvent.click(within(root).getAllByRole("button")[0])
        const body = within(document.body)
        const title = await body.findByText(ca("viewDialog.title"))
        await waitFor(() => expect(title).toBeVisible())
        const details = within(title.closest(".MuiDrawer-paper") as HTMLElement)
        await expect(details.getAllByText(ROOT_SUBJECT)).toHaveLength(2)
        await expect(details.getByText(FINGERPRINT)).toBeVisible()
        await expect(details.getByText(ca("viewDialog.serialNumber"))).toBeVisible()
        expect(details.getByText(/BEGIN CERTIFICATE/)).toHaveTextContent("c3ludGhldGljIHJvb3Q=")
        expect(data.calls.filter(({method}) => method === "getOne")[0]?.args).toEqual([
            RESOURCE,
            expect.objectContaining({id: ROOT_ID}),
        ])
        await userEvent.click(details.getByRole("button", {name: i18n.t("common.label.close")}))
        await waitFor(() => expect(body.queryByText(ca("viewDialog.title"))).toBeNull())
    },
}

export const ViewACertificateWithoutFingerprint: Story = {
    parameters: {
        widgets: ["ViewCAContent", "LabelValue"],
        expectedFailure: {
            reason: "The details drawer is a modal dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        const row = await within(canvasElement).findByRole("row", {name: /Council Voters CA/})
        await userEvent.click(within(row).getAllByRole("button")[0])
        const title = await within(document.body).findByText(ca("viewDialog.title"))
        await waitFor(() => expect(title).toBeVisible())
        const details = within(title.closest(".MuiDrawer-paper") as HTMLElement)
        const label = details.getByText(ca("columns.fingerprint"))
        expect(label.nextElementSibling).toHaveTextContent("—")
        await expect(details.getByText(ca("expiry.expired"))).toBeVisible()
    },
}

export const DeleteACertificate: Story = {
    play: async ({canvasElement}) => {
        const root = await rootRow(canvasElement)
        await userEvent.click(within(root).getAllByRole("button")[1])
        await confirm(i18n.t("common.label.delete"), i18n.t("common.message.delete"))
        await waitFor(() =>
            expect(operations("DeleteCertificateAuthority")[0]?.variables).toEqual({
                ids: [ROOT_ID],
                electionEventId: EVENT_ID,
            })
        )
        // The list is read again without the deleted certificate.
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("row", {name: ROOT_ROW})).toBeNull()
        )
    },
}

export const DeleteFails: Story = {
    args: {mutations: "error"},
    play: async ({canvasElement}) => {
        const root = await rootRow(canvasElement)
        await userEvent.click(within(root).getAllByRole("button")[1])
        await confirm(i18n.t("common.label.delete"), i18n.t("common.message.delete"))
        await notified(ca("notify.deleteError", {error: ""}))
        await expect(await rootRow(canvasElement)).toBeVisible()
    },
}

export const DeleteTheSelection: Story = {
    play: async ({canvasElement}) => {
        await selectRoot(canvasElement)
        const canvas = within(canvasElement)
        await userEvent.click(await shownButton(canvasElement, i18n.t("common.label.delete")))
        await confirm(i18n.t("common.label.delete"), ca("deleteDialog.description", {count: 1}))
        await waitFor(() =>
            expect(operations("DeleteCertificateAuthority")[0]?.variables).toEqual({
                ids: [ROOT_ID],
                electionEventId: EVENT_ID,
            })
        )
        await waitFor(() => expect(canvas.queryByRole("row", {name: ROOT_ROW})).toBeNull())
    },
}

const exportDefects = widgetDefects(
    "button-name",
    "nested-interactive",
    "aria-progressbar-name",
    "color-contrast"
)

export const ExportAll: Story = {
    parameters: defects(exportDefects.expectedFailure.reason, [
        "nested-interactive",
        "aria-progressbar-name",
        "color-contrast",
    ]),
    play: async ({canvasElement}) => {
        await rootRow(canvasElement)
        await userEvent.click(await shownButton(canvasElement, i18n.t("common.label.export")))
        await confirm(
            i18n.t("common.label.export"),
            ca("exportDialog.description", {amount: ca("exportDialog.all")})
        )
        await waitFor(() =>
            expect(operations("ExportCertificateAuthority")[0]?.variables).toEqual({
                ids: [],
                electionEventId: EVENT_ID,
            })
        )
        await expect(
            await within(document.body).findByText(
                i18n.t("tasksScreen.widget.taskTitle", {
                    title: i18n.t("tasksScreen.tasksExecution.EXPORT_CERTIFICATE_AUTHORITIES"),
                })
            )
        ).toBeVisible()
    },
}

export const ExportTheSelection: Story = {
    parameters: defects(exportDefects.expectedFailure.reason, [
        "nested-interactive",
        "aria-progressbar-name",
        "color-contrast",
    ]),
    play: async ({canvasElement}) => {
        await selectRoot(canvasElement)
        await userEvent.click(await shownButton(canvasElement, i18n.t("common.label.export")))
        await confirm(i18n.t("common.label.export"), ca("exportDialog.description", {amount: 1}))
        await waitFor(() =>
            expect(operations("ExportCertificateAuthority")[0]?.variables).toEqual({
                ids: [ROOT_ID],
                electionEventId: EVENT_ID,
            })
        )
    },
}

async function importFile(canvasElement: HTMLElement) {
    await rootRow(canvasElement)
    await userEvent.click(
        within(canvasElement).getByRole("button", {name: i18n.t("common.label.import")})
    )
    const body = within(document.body)
    const title = await body.findByText(ca("importDialog.title"))
    await waitFor(() => expect(title).toBeVisible())
    const drawer = within(title.closest(".MuiDrawer-paper") as HTMLElement)
    const submit = drawer.getByRole("button", {name: ca("importDialog.importButton")})
    expect(submit).toBeDisabled()
    const input = document.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("The import drawer has no file picker")
    await userEvent.upload(
        input,
        new File([PEM], "council-ca.pem", {type: "application/x-pem-file"})
    )
    await expect(
        await drawer.findByText(ca("importDialog.fileLoaded", {bytes: PEM.length}))
    ).toBeVisible()
    await userEvent.click(submit)
    await waitFor(() =>
        expect(operations("ImportCertificateAuthority")[0]?.variables).toEqual({
            electionEventId: EVENT_ID,
            pemContent: PEM,
        })
    )
    return drawer
}

export const ImportCertificates: Story = {
    play: async ({canvasElement}) => {
        await importFile(canvasElement)
        // The skipped certificate is reported in a second notice, after this one hides.
        await notified(ca("notify.importSuccess", {inserted: 1}))
        await waitFor(() =>
            expect(within(document.body).queryByText(ca("importDialog.title"))).toBeNull()
        )
        // The list is read again after the import.
        await waitFor(() =>
            expect(data.calls.filter(({method}) => method === "getList")).toHaveLength(2)
        )
    },
}

export const ImportFails: Story = {
    args: {mutations: "error"},
    parameters: {
        expectedFailure: {
            reason: "The import drawer is a modal dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        const drawer = await importFile(canvasElement)
        await notified(ca("notify.importError", {error: "Synthetic certificate service failure"}))
        await expect(drawer.getByText(ca("importDialog.title"))).toBeVisible()
    },
}
