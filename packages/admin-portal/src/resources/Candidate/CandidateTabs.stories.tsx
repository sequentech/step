// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {CandidateTabs} from "./CandidateTabs"
import {
    CandidateLayout,
    aliceRecord,
    setUpCandidates,
    tallyFlags,
    type CandidateServices,
} from "./__stories__/CandidateFixture"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"

const meta = {
    title: "Admin/Candidate/CandidateTabs",
    component: CandidateTabs,
    args: {reads: "records", empty: false, withImage: false},
    parameters: {
        router: {
            path: "/sequent_backend_candidate/:id",
            initialEntries: [`/sequent_backend_candidate/${STORY_IDS.candidate}`],
            layout: CandidateLayout,
        },
    },
    beforeEach: ({args}) => setUpCandidates(args),
    render: () => (
        <RecordContextProvider value={aliceRecord()}>
            <CandidateTabs />
        </RecordContextProvider>
    ),
} satisfies WidgetMeta<CandidateServices>
export default meta
type Story = StoryObj<CandidateServices>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Alice")).toBeVisible()
        await expect(canvas.getByText("Candidate configuration.")).toBeVisible()
        await expect(canvas.getByRole("tab", {name: "Data"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await waitFor(async () =>
            expect(await canvas.findByRole("textbox", {name: "Name"})).toHaveValue("Alice Example")
        )
        await expect(canvas.getByRole("button", {name: "Save"})).toBeVisible()
        // The tally store follows the open candidate.
        expect(tallyFlags.candidate).toHaveBeenCalledWith(STORY_IDS.candidate)
        expect(tallyFlags.contest).toHaveBeenCalledWith(STORY_IDS.contest)
        expect(tallyFlags.event).toHaveBeenCalledWith(EVENT_ID)
    },
}

export const WithoutCandidateWrite: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(async () =>
            expect(await canvas.findByRole("textbox", {name: "Name"})).toHaveValue("Alice Example")
        )
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
    },
}
