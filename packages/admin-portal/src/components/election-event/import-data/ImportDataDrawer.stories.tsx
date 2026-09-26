// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"
import {ImportDataDrawer} from "./ImportDataDrawer"
import {
    CHECKSUM,
    CreateFlowStory,
    DOCUMENT_ID,
    IMPORTED_EVENT_ID,
    UPLOAD_URL,
    chooseImportFile,
    createFlow,
    type CreateFlow,
} from "../create/__stories__/CreateElectionEventFixture"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /**
     * "provider": the election event list's drawer, opened and handled by the
     * create provider; "props": a page that imports through its own callbacks.
     */
    mode: "provider" | "props"
    /** The provider's import check rejects the uploaded file with this error. */
    importError?: string
    /** An error the page passes to the drawer. */
    errors?: string | null
    doImport: (documentId: string, sha256: string, password?: string) => Promise<void>
    closeDrawer: () => void
}

let flow: CreateFlow

function OpenFromProvider() {
    const {openImportDrawer} = useCreateElectionEventStore()
    useEffect(() => openImportDrawer(), [])
    return null
}

/** A page with its own import drawer state, such as the backup restore settings. */
function PageDrawer({errors, doImport, closeDrawer}: Scenario) {
    const [open, setOpen] = useState(true)
    return (
        <ImportDataDrawer
            open={open}
            closeDrawer={() => {
                closeDrawer()
                setOpen(false)
            }}
            title="settings.backupRestore.restore.title"
            subtitle="settings.backupRestore.restore.subtitle"
            paragraph="settings.backupRestore.restore.paragraph"
            doImport={doImport}
            errors={errors}
        />
    )
}

function Fixture(args: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    if (args.mode === "props") {
        return (
            <AdminStoryProvider
                boundary={flow.graphql}
                dataProvider={flow.data.provider}
                role={permissions}
                tenant={tenant}
            >
                <PageDrawer {...args} />
            </AdminStoryProvider>
        )
    }
    return (
        <CreateFlowStory flow={flow} role={permissions} tenant={tenant}>
            <OpenFromProvider />
            <ImportDataDrawer
                title="electionEventScreen.import.eetitle"
                subtitle="electionEventScreen.import.eesubtitle"
                paragraph="electionEventScreen.import.electionEventParagraph"
            />
        </CreateFlowStory>
    )
}

const drawerDefects = {
    expectedFailure: {
        reason: "The drawer is a modal dialog without an accessible name.",
        a11y: ["aria-dialog-name"],
    },
}

const meta = {
    title: "Admin/Election event/Import data/ImportDataDrawer",
    component: ImportDataDrawer,
    args: {mode: "provider", errors: null, doImport: fn(async () => {}), closeDrawer: fn()},
    argTypes: {mode: {control: "inline-radio", options: ["provider", "props"]}},
    parameters: drawerDefects,
    beforeEach: async ({args}) => {
        flow = createFlow({importError: args.importError})
        await flow.graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const drawer = async (title: string) => {
    const heading = await within(document.body).findByText(title)
    await waitFor(() => expect(heading).toBeVisible())
    const root = heading.closest<HTMLElement>(".MuiDrawer-paper")
    if (!root) throw new Error("The import drawer is missing")
    return root
}

const operations = () => flow.graphql.calls.map(({name}) => name)

export const ElectionEventImport: Story = {
    parameters: {expectedFailure: null},
    play: async () => {
        const root = await drawer("Import Election Event")
        const screen = within(root)
        await expect(screen.getByText("Import Election Events using a JSON file.")).toBeVisible()
        await userEvent.type(
            screen.getByRole("textbox", {name: "Integrity Check (SHA-256)"}),
            CHECKSUM
        )
        await chooseImportFile(root)
        // The provider checks the uploaded file before it can be imported.
        await waitFor(() => expect(screen.getByRole("button", {name: "Import"})).toBeEnabled())
        expect(flow.uploads.calls.map(({method, url}) => `${method} ${url}`)).toEqual([
            `PUT ${UPLOAD_URL}`,
        ])
        expect(flow.graphql.calls.at(-1)).toMatchObject({
            name: "ImportElectionEvent",
            variables: {tenantId: TENANT_ID, documentId: DOCUMENT_ID, checkOnly: true},
        })
        await userEvent.click(screen.getByRole("button", {name: "Import"}))
        await waitFor(() =>
            expect(within(document.body).queryByText("Import Election Event")).toBeNull()
        )
        await waitFor(() =>
            expect(flow.graphql.calls.at(-1)).toMatchObject({
                name: "ImportElectionEvent",
                variables: {tenantId: TENANT_ID, documentId: DOCUMENT_ID, sha256: CHECKSUM},
            })
        )
        await waitFor(() =>
            expect(flow.created).toHaveBeenCalledWith({
                id: IMPORTED_EVENT_ID,
                type: "sequent_backend_election_event",
            })
        )
    },
}

export const RejectedFile: Story = {
    args: {importError: "Unsupported election event file"},
    play: async () => {
        const root = await drawer("Import Election Event")
        const screen = within(root)
        await chooseImportFile(root)
        await expect(await screen.findByText("Unsupported election event file")).toBeVisible()
        await expect(screen.getByRole("button", {name: "Import"})).toBeDisabled()
        expect(operations()).toEqual(["GetUploadUrl", "ImportElectionEvent"])
    },
}

export const CancelClosesTheProviderDrawer: Story = {
    parameters: {expectedFailure: null},
    play: async () => {
        const root = await drawer("Import Election Event")
        await userEvent.click(within(root).getByRole("button", {name: "Cancel"}))
        await waitFor(() =>
            expect(within(document.body).queryByText("Import Election Event")).toBeNull()
        )
        expect(flow.graphql.calls).toEqual([])
    },
}

export const PageImport: Story = {
    args: {mode: "props"},
    play: async ({args}) => {
        const root = await drawer("Import Tenant Configurations")
        const screen = within(root)
        await userEvent.type(
            screen.getByRole("textbox", {name: "Integrity Check (SHA-256)"}),
            CHECKSUM
        )
        await chooseImportFile(root)
        const importButton = screen.getByRole("button", {name: "Import"})
        await waitFor(() => expect(importButton).toBeEnabled())
        // Without its own upload callback, the drawer skips the provider's import check.
        expect(operations()).toEqual(["GetUploadUrl"])
        await userEvent.click(importButton)
        await waitFor(() => expect(args.doImport).toHaveBeenCalledWith(DOCUMENT_ID, CHECKSUM, ""))
        expect(args.closeDrawer).not.toHaveBeenCalled()
    },
}

export const PageError: Story = {
    args: {mode: "props", errors: "The backup could not be restored"},
    play: async () => {
        const root = await drawer("Import Tenant Configurations")
        await expect(within(root).getByText("The backup could not be restored")).toBeVisible()
        expect(flow.graphql.calls).toEqual([])
    },
}

export const PageCancel: Story = {
    args: {mode: "props"},
    parameters: {expectedFailure: null},
    play: async ({args}) => {
        const root = await drawer("Import Tenant Configurations")
        await userEvent.click(within(root).getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(args.closeDrawer).toHaveBeenCalledTimes(1))
        await waitFor(() =>
            expect(within(document.body).queryByText("Import Tenant Configurations")).toBeNull()
        )
        expect(args.doImport).not.toHaveBeenCalled()
    },
}
