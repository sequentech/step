// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EditBase} from "react-admin"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {CandidateDataForm} from "./CandidateDataForm"
import {
    CandidateScreen,
    IMAGE_ID,
    aliceRecord,
    dataWrites,
    graphqlCalls,
    setUpCandidates,
    type CandidateServices,
} from "./__stories__/CandidateFixture"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"

interface Scenario extends CandidateServices {
    /** Whether the upload service issues an upload address. */
    uploadAvailable: boolean
}

const UPLOAD_URL = "https://s3.admin-story.invalid/upload/"
const NEW_IMAGE_ID = "55555555-5555-4555-8555-555555555558"

let uploads: ReturnType<typeof storyFetch>

const meta = {
    title: "Admin/Candidate/CandidateDataForm",
    component: CandidateDataForm,
    args: {
        reads: "records",
        empty: false,
        withImage: false,
        uploadAvailable: true,
    },
    beforeEach: async ({args}) => {
        await setUpCandidates(args, {
            GetUploadUrl: () => ({
                data: {
                    get_upload_url: args.uploadAvailable
                        ? {url: `${UPLOAD_URL}${NEW_IMAGE_ID}`, document_id: NEW_IMAGE_ID}
                        : null,
                },
            }),
        })
        uploads = storyFetch({[UPLOAD_URL]: () => ({status: 200})})
    },
    render: ({withImage}) => (
        <CandidateScreen>
            <EditBase id={STORY_IDS.candidate} mutationMode="pessimistic" redirect={false}>
                <CandidateDataForm record={aliceRecord(withImage)} />
            </EditBase>
        </CandidateScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const otherSection = {
    expectedFailure: {
        reason: "The form's accordion regions have no name that tells them apart.",
        a11y: ["landmark-unique"],
    },
}

const pictureSection = {
    expectedFailure: {
        reason:
            "The accordion regions have no distinguishing name, and the picture's delete " +
            "button has no name and sits inside the section's summary button.",
        a11y: ["button-name", "landmark-unique", "nested-interactive"],
    },
}

const englishName = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {name: "Name"})

async function openSection(canvasElement: HTMLElement, name: string) {
    await userEvent.click(await within(canvasElement).findByRole("button", {name}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(async () =>
            expect(await englishName(canvasElement)).toHaveValue("Alice Example")
        )
        // The election's languages decide the tabs.
        expect(canvas.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
            "English",
            "Spanish",
        ])
        await expect(canvas.getByRole("button", {name: "Save"})).toBeVisible()
    },
}

export const ReadOnlyWithoutCandidateWrite: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    play: async ({canvasElement}) => {
        await waitFor(async () =>
            expect(await englishName(canvasElement)).toHaveValue("Alice Example")
        )
        expect(within(canvasElement).queryByRole("button", {name: "Save"})).toBeNull()
    },
}

export const SaveTheSpanishName: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await englishName(canvasElement)
        await userEvent.click(canvas.getByRole("tab", {name: "Spanish"}))
        const spanish = await canvas.findByRole("textbox", {name: "Name"})
        await userEvent.type(spanish, "Alicia Ejemplo")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0]).toEqual({
            method: "update",
            resource: "sequent_backend_candidate",
            params: expect.objectContaining({
                id: STORY_IDS.candidate,
                data: expect.objectContaining({
                    presentation: expect.objectContaining({
                        i18n: expect.objectContaining({
                            en: expect.objectContaining({name: "Alice Example"}),
                            es: expect.objectContaining({name: "Alicia Ejemplo"}),
                        }),
                    }),
                }),
            }),
        })
    },
}

export const MarkAsWriteIn: Story = {
    parameters: otherSection,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await englishName(canvasElement)
        await openSection(canvasElement, "Type")
        await userEvent.click(await canvas.findByRole("switch", {name: "Write-in"}))
        await userEvent.click(canvas.getByRole("combobox", {name: /Invalid Vote Position/}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Top"}))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0].params).toMatchObject({
            data: {presentation: {is_write_in: true, invalid_vote_position: "top"}},
        })
    },
}

export const UploadAPicture: Story = {
    parameters: pictureSection,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await englishName(canvasElement)
        await openSection(canvasElement, "Image")
        const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
        if (!input) throw new Error("No file input")
        await userEvent.upload(input, new File(["png"], "Carol photo.png", {type: "image/png"}))
        const message = await within(document.body).findByText("File loaded")
        await waitFor(() => expect(message).toBeVisible())
        expect(graphqlCalls().find(({name}) => name === "GetUploadUrl")?.variables).toEqual({
            name: "Carolphoto.png",
            media_type: "image/png",
            size: 3,
            is_public: true,
        })
        expect(uploads.calls).toEqual([
            expect.objectContaining({method: "PUT", url: `${UPLOAD_URL}${NEW_IMAGE_ID}`}),
        ])
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0].params).toMatchObject({
            id: STORY_IDS.candidate,
            data: {
                image_document_id: NEW_IMAGE_ID,
                presentation: {
                    urls: [
                        {
                            url: `tenant-${aliceRecord().tenant_id}/document-${NEW_IMAGE_ID}/Carolphoto.png`,
                            is_image: true,
                        },
                    ],
                },
            },
        })
        await expect(canvas.getByText("Carol photo.png")).toBeVisible()
    },
}

export const UploadUnavailable: Story = {
    parameters: otherSection,
    args: {uploadAvailable: false},
    play: async ({canvasElement}) => {
        await englishName(canvasElement)
        await openSection(canvasElement, "Image")
        const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
        if (!input) throw new Error("No file input")
        await userEvent.upload(input, new File(["png"], "carol.png", {type: "image/png"}))
        const message = await within(document.body).findByText("Error uploading file")
        await waitFor(() => expect(message).toBeVisible())
        expect(uploads.calls).toEqual([])
        expect(dataWrites()).toEqual([])
    },
}

export const RemoveThePicture: Story = {
    parameters: otherSection,
    args: {withImage: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await englishName(canvasElement)
        await openSection(canvasElement, "Image")
        const picture = await canvas.findByRole("img")
        expect(picture).toHaveAttribute(
            "src",
            `${globalThis.location.origin}/story-bucket/tenant-${aliceRecord().tenant_id}/document-${IMAGE_ID}/alice.png`
        )
        const summary = canvas.getByRole("button", {name: "Image"})
        const buttons = within(summary).getAllByRole("button")
        await userEvent.click(buttons[0])
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0].params).toMatchObject({
            id: STORY_IDS.candidate,
            data: {image_document_id: null, presentation: {urls: []}},
        })
    },
}
