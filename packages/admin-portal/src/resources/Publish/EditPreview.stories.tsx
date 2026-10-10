// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, spyOn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, areaRecords, storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    startedTask,
    taskHandler,
    taskWidgetDefects,
} from "@/components/tally/__stories__/DownloadFixture"
import type {GetBallotPublicationChangesOutput} from "@/gql/graphql"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {EditPreview} from "./EditPreview"

interface Scenario {
    /** Whether the preview service rejects the request. */
    preparationFails: boolean
    /** What writing to the clipboard does. */
    clipboard: "granted" | "denied"
    close: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let opened: unknown[][]
let copied: string[]

const PUBLICATION_ID = storyId(8, 2)
const DOCUMENT_ID = storyId(8, 6)
const TASK_ID = storyId(8, 7)
const PREVIEW_URL = `https://voting.admin-story.invalid/preview/${TENANT_ID}/${DOCUMENT_ID}/${STORY_IDS.area}/${PUBLICATION_ID}`

/** The publication has a ballot style for the north district only. */
const BALLOT_DATA = {
    previous: null,
    current: {
        ballot_publication_id: PUBLICATION_ID,
        ballot_styles: [{id: storyId(9, 2), area_id: STORY_IDS.area}],
    },
} as GetBallotPublicationChangesOutput

const meta = {
    title: "Admin/Publish/EditPreview",
    component: EditPreview,
    args: {preparationFails: false, clipboard: "granted", close: fn()},
    argTypes: {
        clipboard: {control: "inline-radio", options: ["granted", "denied"]},
        close: {table: {disable: true}},
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                sequent_backend_area: () => ({
                    data: {
                        sequent_backend_area: areaRecords().map(({id, name}) => ({id, name})),
                    },
                }),
                PrepareBallotPublicationPreview: () =>
                    args.preparationFails
                        ? {errors: [new GraphQLError("Synthetic preview unavailable")]}
                        : {
                              data: {
                                  prepare_ballot_publication_preview: {
                                      error_msg: null,
                                      document_id: DOCUMENT_ID,
                                      task_execution: startedTask(
                                          TASK_ID,
                                          "PREPARE_PUBLICATION_PREVIEW"
                                      ),
                                  },
                              },
                          },
                ...taskHandler("SUCCESS", "PREPARE_PUBLICATION_PREVIEW"),
            },
            {schema: true}
        )
        await graphql.ready
        opened = []
        copied = []
        const open = spyOn(window, "open").mockImplementation((...call) => {
            opened.push(call)
            return null
        })
        const clipboard = spyOn(navigator.clipboard, "writeText").mockImplementation(
            async (text) => {
                if (args.clipboard === "denied") throw new DOMException("Denied", "NotAllowedError")
                copied.push(text)
            }
        )
        return () => {
            open.mockRestore()
            clipboard.mockRestore()
        }
    },
    render: ({close}) => (
        <AdminStoryProvider boundary={graphql}>
            <WidgetsContextProvider>
                <EditPreview
                    publicationId={PUBLICATION_ID}
                    electionEventId={EVENT_ID}
                    ballotData={BALLOT_DATA}
                    close={close}
                />
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function chooseNorth(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await waitFor(() =>
        expect(graphql.calls.map(({name}) => name)).toContain("sequent_backend_area")
    )
    await userEvent.click(canvas.getByRole("combobox", {name: "Select Area for Preview"}))
    await userEvent.click(
        await within(document.body).findByRole("option", {name: "North district"})
    )
    await waitFor(() => expect(canvas.getByRole("button", {name: "Preview"})).toBeEnabled())
    return canvas
}

const notice = (text: string) =>
    waitFor(() => expect(within(document.body).getByText(text)).toBeVisible())

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: "Preview"})).toBeDisabled()
        await expect(canvas.getByRole("button", {name: "Copy link"})).toBeDisabled()
        await userEvent.click(canvas.getByRole("combobox", {name: "Select Area for Preview"}))
        const listbox = await within(document.body).findByRole("listbox")
        // Only the areas with a ballot style in the publication.
        await waitFor(() =>
            expect(
                within(listbox)
                    .getAllByRole("option")
                    .map((option) => option.textContent)
            ).toEqual(["North district"])
        )
        expect(graphql.calls).toEqual([
            expect.objectContaining({
                name: "sequent_backend_area",
                variables: {electionEventId: EVENT_ID},
            }),
        ])
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(listbox).not.toBeInTheDocument())
    },
}

export const OpenAPreview: Story = {
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement, args}) => {
        const canvas = await chooseNorth(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Preview"}))
        await notice("Success opening preview")
        expect(opened).toEqual([[PREVIEW_URL, "_blank"]])
        expect(args.close).toHaveBeenCalledOnce()
        expect(
            graphql.calls.find(({name}) => name === "PrepareBallotPublicationPreview")
        ).toMatchObject({
            variables: {electionEventId: EVENT_ID, ballotPublicationId: PUBLICATION_ID},
        })
        expect(copied).toEqual([])
    },
}

export const CopyThePreviewLink: Story = {
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement, args}) => {
        const canvas = await chooseNorth(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Copy link"}))
        await notice("Success copying preview link")
        expect(copied).toEqual([PREVIEW_URL])
        expect(args.close).toHaveBeenCalledOnce()
        expect(opened).toEqual([])
    },
}

export const CopyDenied: Story = {
    args: {clipboard: "denied"},
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement, args}) => {
        const canvas = await chooseNorth(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Copy link"}))
        await notice("Failed to copy preview link")
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const PreparationFails: Story = {
    args: {preparationFails: true},
    parameters: taskWidgetDefects("FAILED"),
    play: async ({canvasElement, args}) => {
        const canvas = await chooseNorth(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Preview"}))
        await notice("Error previewing publication")
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(opened).toEqual([])
        expect(args.close).not.toHaveBeenCalled()
    },
}
