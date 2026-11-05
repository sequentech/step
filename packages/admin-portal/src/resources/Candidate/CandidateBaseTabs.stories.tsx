// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {CandidateBaseTabs} from "./CandidateBaseTabs"
import {
    CandidateLayout,
    reads,
    setUpCandidates,
    tallyFlags,
    type CandidateServices,
} from "./__stories__/CandidateFixture"

const meta = {
    title: "Admin/Candidate/CandidateBaseTabs",
    component: CandidateBaseTabs,
    args: {reads: "records", empty: false, withImage: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_candidate/:id",
            initialEntries: [`/sequent_backend_candidate/${STORY_IDS.candidate}`],
            layout: CandidateLayout,
        },
    },
    beforeEach: ({args}) => setUpCandidates(args),
    render: () => <CandidateBaseTabs />,
} satisfies WidgetMeta<CandidateServices>
export default meta
type Story = StoryObj<CandidateServices>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Alice")).toBeVisible()
        await waitFor(async () =>
            expect(await canvas.findByRole("textbox", {name: "Name"})).toHaveValue("Alice Example")
        )
        expect(
            reads("getOne", "sequent_backend_candidate").map(({args}) => args[1])
        ).toContainEqual(expect.objectContaining({id: STORY_IDS.candidate}))
        expect(tallyFlags.candidate).toHaveBeenCalledWith(STORY_IDS.candidate)
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_candidate")).toHaveLength(1))
        const canvas = within(canvasElement)
        // Only the header shows until the candidate arrives.
        await expect(canvas.getByText("Candidate configuration.")).toBeVisible()
        expect(canvas.queryByRole("tab")).toBeNull()
        expect(tallyFlags.candidate).not.toHaveBeenCalled()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(message).toBeVisible())
        await expect(
            within(canvasElement).getByRole("status", {name: "Current location"})
        ).toHaveTextContent(/^\/sequent_backend_candidate$/)
    },
}
