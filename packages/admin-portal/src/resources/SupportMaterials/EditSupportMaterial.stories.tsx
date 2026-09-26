// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
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
import {EditSupportMaterial} from "./EditSuportMaterial"
import {
    DOCUMENT_RESOURCE,
    EVENT_RESOURCE,
    MATERIAL_DOCUMENT_ID,
    MATERIAL_ID,
    MATERIAL_RESOURCE,
    UPLOADED_DOCUMENT_ID,
    UPLOAD_URL,
    documentRecords,
    materialRecords,
} from "./__stories__/SupportMaterialFixture"

interface Scenario {
    /** What reading the election event does. */
    eventReads: ReadState
    /** What reading the material does. */
    reads: ReadState
    /** Whether saving the material fails. */
    failure: boolean
    close: Mock<() => void>
}

const PUBLIC_URL = `https://public.admin-story.invalid/tenant-${TENANT_ID}/document-${MATERIAL_DOCUMENT_ID}/voting-guide.pdf`

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let uploads: ReturnType<typeof storyFetch>

const meta = {
    title: "Admin/Support materials/EditSupportMaterial",
    component: EditSupportMaterial,
    args: {eventReads: "records", reads: "records", failure: false, close: fn()},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {
                [EVENT_RESOURCE]: [eventRecord()],
                [MATERIAL_RESOURCE]: materialRecords(),
                [DOCUMENT_RESOURCE]: documentRecords(),
            },
            {
                reads: {[EVENT_RESOURCE]: args.eventReads, [MATERIAL_RESOURCE]: args.reads},
                writeError: args.failure ? "Synthetic material service failure" : undefined,
            }
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
    render: ({close}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            auth={{tenantId: TENANT_ID}}
        >
            <EditSupportMaterial id={MATERIAL_ID} electionEventId={EVENT_ID} close={close} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const titleLabel = () => i18n.t("electionEventScreen.field.materialTitle")

/** The title field, once the material has loaded into it. */
async function titleField(canvasElement: HTMLElement) {
    await waitFor(() =>
        expect(within(canvasElement).getByRole("textbox", {name: titleLabel()})).toHaveValue(
            "Voting guide"
        )
    )
    return within(canvasElement).getByRole("textbox", {name: titleLabel()})
}

async function expectNotification(text: string) {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

async function renameTheGuide(canvasElement: HTMLElement) {
    const title = await titleField(canvasElement)
    await userEvent.clear(title)
    await userEvent.type(title, "Voter handbook")
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    parameters: {widgets: ["GetPublicURL"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await titleField(canvasElement)).toBeVisible()
        await expect(
            canvas.getByRole("textbox", {
                name: i18n.t("electionEventScreen.field.materialSubTitle"),
            })
        ).toHaveValue("How to vote online")
        await expect(
            await canvas.findByRole("textbox", {name: i18n.t("materials.fields.publicUrl")})
        ).toHaveValue(PUBLIC_URL)
        await expect(
            canvas.getByRole("switch", {name: i18n.t("materials.fields.isHidden")})
        ).not.toBeChecked()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual(
            expect.arrayContaining([
                ["getOne", EVENT_RESOURCE],
                ["getOne", MATERIAL_RESOURCE],
                ["getList", DOCUMENT_RESOURCE],
            ])
        )
        expect(data.writes).toEqual([])
    },
}

export const RenameAMaterial: Story = {
    play: async ({canvasElement, args}) => {
        await renameTheGuide(canvasElement)
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "update",
                    resource: MATERIAL_RESOURCE,
                    params: expect.objectContaining({
                        id: MATERIAL_ID,
                        data: expect.objectContaining({
                            data: {
                                title_i18n: {en: "Voter handbook", es: "Guía de voto"},
                                subtitle_i18n: {
                                    en: "How to vote online",
                                    es: "Cómo votar en línea",
                                },
                            },
                            document_id: MATERIAL_DOCUMENT_ID,
                            kind: "application/pdf",
                        }),
                    }),
                },
            ])
        )
        await expectNotification(i18n.t("materials.updateMaterialSuccess"))
        expect(args.close).toHaveBeenCalledTimes(1)
    },
}

export const ReplaceTheDocument: Story = {
    play: async ({canvasElement, args}) => {
        await titleField(canvasElement)
        const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
        if (!input) throw new Error("The material file input is missing")
        await userEvent.upload(input, new File(["video"], "voting-guide.mp4", {type: "video/mp4"}))
        await waitFor(() => expect(uploads.calls.map(({method}) => method)).toEqual(["PUT"]))
        expect(graphql.calls.map(({name, variables}) => [name, variables])).toEqual([
            [
                "GetUploadUrl",
                {
                    name: "voting-guide.mp4",
                    media_type: "video/mp4",
                    size: 5,
                    is_public: true,
                    election_event_id: EVENT_ID,
                },
            ],
        ])
        await expectNotification(i18n.t("electionScreen.common.fileLoaded"))
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(data.writes.map(({params}) => params.data)).toEqual([
                expect.objectContaining({document_id: UPLOADED_DOCUMENT_ID, kind: "video/mp4"}),
            ])
        )
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
    },
}

export const RequireATitle: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.clear(await titleField(canvasElement))
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await expect(
            await within(canvasElement).findByText(i18n.t("materials.error.title"))
        ).toBeVisible()
        expect(data.writes).toEqual([])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const UpdateFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement, args}) => {
        await renameTheGuide(canvasElement)
        await expectNotification(i18n.t("materials.updateMaterialError"))
        expect(data.writes.map(({method}) => method)).toEqual(["update"])
        // The form closes and the edit is lost.
        expect(args.close).toHaveBeenCalledTimes(1)
    },
}

export const LoadingTheEvent: Story = {
    args: {eventReads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
                ["getOne", EVENT_RESOURCE],
            ])
        )
        expect(within(canvasElement).queryByRole("textbox")).toBeNull()
    },
}

export const LoadingTheMaterial: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(data.calls.map(({method, args}) => [method, args[0]])).toContainEqual([
                "getOne",
                MATERIAL_RESOURCE,
            ])
        )
        expect(within(canvasElement).queryByDisplayValue("Voting guide")).toBeNull()
    },
}
