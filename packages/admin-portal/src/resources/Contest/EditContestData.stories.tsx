// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EditContestData} from "./EditContestData"
import {
    ContestLayout,
    dataWrites,
    reads,
    setUpContests,
    type ContestServices,
} from "./__stories__/ContestFixture"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"

const meta = {
    title: "Admin/Contest/EditContestData",
    component: EditContestData,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_contest/:id",
            initialEntries: [`/sequent_backend_contest/${STORY_IDS.contest}`],
            layout: ContestLayout,
        },
    },
    beforeEach: ({args}) => setUpContests(args),
    render: () => <EditContestData />,
} satisfies WidgetMeta<ContestServices>
export default meta
type Story = StoryObj<ContestServices>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByDisplayValue("Council members")).toBeVisible()
        expect(canvas.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
            "English",
            "Spanish",
        ])
        await expect(canvas.getByRole("button", {name: "Save"})).toBeVisible()
        expect(reads("getOne", "sequent_backend_contest")[0].args[1]).toMatchObject({
            id: STORY_IDS.contest,
        })
    },
}

export const WithoutContestWrite: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByDisplayValue("Council members")).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {
        expectedFailure: {
            reason: "The progress indicator shown until the record arrives has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_contest")).toHaveLength(1))
        expect(within(canvasElement).queryByRole("button", {name: "Save"})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(message).toBeVisible())
        await expect(
            within(canvasElement).getByRole("status", {name: "Current location"})
        ).toHaveTextContent(/^\/sequent_backend_contest$/)
    },
}

export const RenameFromThePresentation: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const name = await canvas.findByDisplayValue("Council members")
        await userEvent.clear(name)
        await userEvent.type(name, "Council seats")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        // The update is undoable: it reaches the service once its notification closes.
        const notification = await within(document.body).findByText("Element updated")
        expect(dataWrites()).toEqual([])
        await userEvent.click(canvasElement)
        await waitFor(() => expect(notification).not.toBeInTheDocument())
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        // Since migration 1772358027729 the name and alias live only in the presentation.
        expect(dataWrites()[0].params.data).not.toHaveProperty("name")
        expect(dataWrites()[0].params.data).not.toHaveProperty("alias")
        expect(dataWrites()[0]).toEqual({
            method: "update",
            resource: "sequent_backend_contest",
            params: expect.objectContaining({
                id: STORY_IDS.contest,
                data: expect.objectContaining({
                    description: "Select up to two members",
                    presentation: expect.objectContaining({
                        i18n: expect.objectContaining({
                            en: expect.objectContaining({name: "Council seats"}),
                        }),
                    }),
                }),
            }),
        })
    },
}
