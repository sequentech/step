// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {eventRecord} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {ListSupportMaterials} from "./ListSuportMaterial"
import {
    DOCUMENT_RESOURCE,
    EVENT_RESOURCE,
    MATERIAL_ID,
    MATERIAL_RESOURCE,
    UPLOADED_DOCUMENT_ID,
    UPLOAD_URL,
    documentRecords,
    materialEvent,
    materialRecords,
} from "./__stories__/SupportMaterialFixture"

interface Scenario {
    /** What reading the materials does. */
    reads: ReadState
    /** Whether the event has support materials. */
    populated: boolean
    /** The signed-in user's roles. */
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let uploads: ReturnType<typeof storyFetch>

const meta = {
    title: "Admin/Support materials/ListSupportMaterials",
    component: ListSupportMaterials,
    args: {
        reads: "records",
        populated: true,
        roles: [IPermissions.SUPPORT_MATERIAL_READ, IPermissions.SUPPORT_MATERIAL_WRITE],
    },
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: `/${EVENT_RESOURCE}/:id`,
            initialEntries: [`/${EVENT_RESOURCE}/${EVENT_ID}`],
        },
        expectedFailure: {
            reason: "The grid's row checkboxes are unlabelled and the rows' edit and delete actions are unnamed icon buttons.",
            a11y: ["aria-prohibited-attr", "button-name", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {
                [EVENT_RESOURCE]: [eventRecord()],
                [MATERIAL_RESOURCE]: args.populated ? materialRecords() : [],
                [DOCUMENT_RESOURCE]: documentRecords(),
            },
            {reads: {[MATERIAL_RESOURCE]: args.reads}}
        )
        graphql = graphqlBoundary(
            {
                GetUploadUrl: () => ({
                    data: {get_upload_url: {url: UPLOAD_URL, document_id: UPLOADED_DOCUMENT_ID}},
                }),
            },
            {schema: true}
        )
        await graphql.ready
        uploads = storyFetch({[UPLOAD_URL]: () => ({status: 200})})
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={roles}
            auth={{tenantId: TENANT_ID}}
        >
            {/* As the event's support material tab renders the list. */}
            <RecordContextProvider value={materialEvent()}>
                <ListSupportMaterials />
            </RecordContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const guideRow = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("row", {name: /Voting guide/})

const listCalls = () =>
    data.calls.filter(({method, args}) => method === "getList" && args[0] === MATERIAL_RESOURCE)

/** The topmost open drawer. */
async function drawer() {
    const drawers = await within(document.body).findAllByRole("presentation")
    const element = drawers[drawers.length - 1]
    await waitFor(() => expect(element).toBeVisible())
    return element
}

const titleLabel = () => i18n.t("electionEventScreen.field.materialTitle")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const row = within(await guideRow(canvasElement))
        await expect(row.getByText("How to vote online")).toBeVisible()
        await expect(
            within(canvasElement).getByRole("button", {name: i18n.t("common.label.add")})
        ).toBeVisible()
        expect(listCalls()[0].args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID, election_event_id: EVENT_ID},
        })
        expect(data.writes).toEqual([])
    },
}

export const Empty: Story = {
    args: {populated: false},
    parameters: {
        expectedFailure: {
            reason: "The empty state's create button nests an icon button.",
            a11y: ["nested-interactive"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(i18n.t("materials.empty.header"))).toBeVisible()
        await expect(
            canvas.getByRole("button", {name: i18n.t("materials.empty.action").trim()})
        ).toBeVisible()
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.SUPPORT_MATERIAL_READ]},
    parameters: {
        expectedFailure: {
            reason: "The grid's row checkboxes are unlabelled.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    play: async ({canvasElement}) => {
        const row = within(await guideRow(canvasElement))
        expect(row.queryAllByRole("button")).toEqual([])
        expect(
            within(canvasElement).queryByRole("button", {name: i18n.t("common.label.add")})
        ).toBeNull()
    },
}

export const CreateAMaterial: Story = {
    play: async ({canvasElement}) => {
        await guideRow(canvasElement)
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: i18n.t("common.label.add")})
        )
        const element = await drawer()
        const form = within(element)
        await userEvent.type(
            await form.findByRole("textbox", {name: titleLabel()}),
            "Accessibility guide"
        )
        const input = element.querySelector<HTMLInputElement>('input[type="file"]')
        if (!input) throw new Error("The material file input is missing")
        await userEvent.upload(
            input,
            new File(["%PDF-1.7"], "accessibility.pdf", {type: "application/pdf"})
        )
        await waitFor(() => expect(uploads.calls.map(({method}) => method)).toEqual(["PUT"]))
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "create",
                    resource: MATERIAL_RESOURCE,
                    params: {
                        data: expect.objectContaining({
                            data: {title_i18n: {en: "Accessibility guide"}, subtitle_i18n: {}},
                            document_id: UPLOADED_DOCUMENT_ID,
                            election_event_id: EVENT_ID,
                            tenant_id: TENANT_ID,
                        }),
                    },
                },
            ])
        )
        await waitFor(() => expect(within(document.body).queryByRole("presentation")).toBeNull())
        await expect(
            await within(canvasElement).findByRole("row", {name: /Accessibility guide/})
        ).toBeVisible()
    },
}

export const EditAMaterial: Story = {
    play: async ({canvasElement}) => {
        const [edit] = within(await guideRow(canvasElement)).getAllByRole("button")
        await userEvent.click(edit)
        const form = within(await drawer())
        await waitFor(() =>
            expect(form.getByRole("textbox", {name: titleLabel()})).toHaveValue("Voting guide")
        )
        await userEvent.type(form.getByRole("textbox", {name: titleLabel()}), " 2026")
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(
                data.writes.map(({method, resource, params}) => [method, resource, params.id])
            ).toEqual([["update", MATERIAL_RESOURCE, MATERIAL_ID]])
        )
        await waitFor(() =>
            expect(
                within(document.body).getByText(i18n.t("materials.updateMaterialSuccess"))
            ).toBeVisible()
        )
        await waitFor(() => expect(document.body.querySelector(".MuiDrawer-root")).toBeNull())
        await expect(
            await within(canvasElement).findByRole("row", {name: /Voting guide 2026/})
        ).toBeVisible()
    },
}

export const DeleteAMaterial: Story = {
    parameters: {
        expectedFailure: {
            reason: "The empty state left after the deletion nests an icon button in its create button.",
            a11y: ["nested-interactive"],
        },
    },
    play: async ({canvasElement}) => {
        const buttons = within(await guideRow(canvasElement)).getAllByRole("button")
        await userEvent.click(buttons[buttons.length - 1])
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(dialog.getByText(i18n.t("common.message.delete"))).toBeVisible()
        expect(data.writes).toEqual([])
        await userEvent.click(dialog.getByRole("button", {name: i18n.t("common.label.delete")}))
        await waitFor(() =>
            expect(
                data.writes.map(({method, resource, params}) => [method, resource, params.id])
            ).toEqual([["delete", MATERIAL_RESOURCE, MATERIAL_ID]])
        )
        await expect(
            await within(canvasElement).findByText(i18n.t("materials.empty.header"))
        ).toBeVisible()
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(listCalls()).toHaveLength(1))
        expect(within(canvasElement).queryByRole("row", {name: /Voting guide/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(listCalls()).toHaveLength(1)
        expect(within(canvasElement).queryByRole("row", {name: /Voting guide/})).toBeNull()
    },
}
