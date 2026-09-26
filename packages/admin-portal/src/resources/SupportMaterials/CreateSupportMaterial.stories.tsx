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
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {CreateSupportMaterial} from "./CreateSupportMaterial"
import {
    MATERIAL_RESOURCE,
    UPLOADED_DOCUMENT_ID,
    UPLOAD_URL,
    materialEvent,
} from "./__stories__/SupportMaterialFixture"

interface Scenario {
    /** Whether storing the uploaded file fails. */
    uploadFailure: boolean
    /** Whether creating the material fails. */
    failure: boolean
    close: Mock<() => void>
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let uploads: ReturnType<typeof storyFetch>

const meta = {
    title: "Admin/Support materials/CreateSupportMaterial",
    component: CreateSupportMaterial,
    args: {uploadFailure: false, failure: false, close: fn()},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[MATERIAL_RESOURCE]: []},
            {writeError: args.failure ? "Synthetic material service failure" : undefined}
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
        uploads = storyFetch({
            [UPLOAD_URL]: () => {
                if (args.uploadFailure) throw new Error("Synthetic storage failure")
                return {status: 200}
            },
        })
    },
    render: ({close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <CreateSupportMaterial record={materialEvent()} close={close} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const titleField = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {
        name: i18n.t("electionEventScreen.field.materialTitle"),
    })

async function expectNotification(text: string) {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

async function uploadTheGuide(canvasElement: HTMLElement) {
    const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("The material file input is missing")
    await userEvent.upload(
        input,
        new File(["%PDF-1.7"], "voting-guide.pdf", {type: "application/pdf"})
    )
    await waitFor(() =>
        expect(uploads.calls.map(({method, url}) => [method, url])).toEqual([["PUT", UPLOAD_URL]])
    )
    expect(graphql.calls).toEqual([
        {
            name: "GetUploadUrl",
            variables: {
                name: "voting-guide.pdf",
                media_type: "application/pdf",
                size: 8,
                is_public: true,
                election_event_id: EVENT_ID,
            },
            headers: {},
        },
    ])
}

const save = (canvasElement: HTMLElement) =>
    userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await titleField(canvasElement)).toHaveValue("")
        await expect(canvas.getByText(i18n.t("materials.common.title"))).toBeVisible()
        // One tab for each enabled language of the event.
        expect(canvas.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
            i18n.t("common.language.en"),
            i18n.t("common.language.es"),
        ])
        await expect(
            canvas.getByRole("switch", {name: i18n.t("materials.fields.isHidden")})
        ).not.toBeChecked()
        expect(graphql.calls).toEqual([])
        expect(data.calls).toEqual([])
    },
}

export const RequireATitleAndADocument: Story = {
    parameters: {widgets: ["UploadValidationError"]},
    play: async ({canvasElement, args}) => {
        await titleField(canvasElement)
        await save(canvasElement)
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(i18n.t("materials.error.title"))).toBeVisible()
        await expect(canvas.getByText(i18n.t("materials.error.document"))).toBeVisible()
        expect(data.writes).toEqual([])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const CreateAMaterial: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.type(await titleField(canvasElement), "Voting guide")
        await userEvent.type(
            within(canvasElement).getByRole("textbox", {
                name: i18n.t("electionEventScreen.field.materialSubTitle"),
            }),
            "How to vote online"
        )
        await uploadTheGuide(canvasElement)
        await expectNotification(i18n.t("electionScreen.common.fileLoaded"))
        await save(canvasElement)
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "create",
                    resource: MATERIAL_RESOURCE,
                    params: {
                        data: expect.objectContaining({
                            data: {
                                title_i18n: {en: "Voting guide"},
                                subtitle_i18n: {en: "How to vote online"},
                            },
                            kind: "application/pdf",
                            document_id: UPLOADED_DOCUMENT_ID,
                            is_hidden: false,
                            election_event_id: EVENT_ID,
                            tenant_id: TENANT_ID,
                        }),
                    },
                },
            ])
        )
        await expectNotification(i18n.t("materials.createMaterialSuccess"))
        expect(args.close).toHaveBeenCalledTimes(1)
    },
}

export const UploadFailure: Story = {
    args: {uploadFailure: true},
    play: async ({canvasElement, args}) => {
        await userEvent.type(await titleField(canvasElement), "Voting guide")
        await uploadTheGuide(canvasElement)
        await expectNotification(i18n.t("electionScreen.error.fileError"))
        await save(canvasElement)
        await expect(
            await within(canvasElement).findByText(i18n.t("materials.error.document"))
        ).toBeVisible()
        expect(data.writes).toEqual([])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const CreateFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement, args}) => {
        await userEvent.type(await titleField(canvasElement), "Voting guide")
        await uploadTheGuide(canvasElement)
        await save(canvasElement)
        await expectNotification(i18n.t("materials.createMaterialError"))
        expect(data.writes.map(({method}) => method)).toEqual(["create"])
        // The form closes and the typed material is lost.
        expect(args.close).toHaveBeenCalledTimes(1)
    },
}
