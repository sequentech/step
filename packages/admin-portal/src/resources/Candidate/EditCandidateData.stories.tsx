// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EditCandidateData} from "./EditCandidateData"
import {
    CandidateLayout,
    aliceRecord,
    dataWrites,
    reads,
    setUpCandidates,
    type CandidateServices,
} from "./__stories__/CandidateFixture"

interface Scenario extends CandidateServices {
    /** Whether the tabs have the candidate's record yet. */
    withRecord: boolean
}

const meta = {
    title: "Admin/Candidate/EditCandidateData",
    component: EditCandidateData,
    args: {reads: "records", empty: false, withImage: false, withRecord: true},
    parameters: {
        router: {
            path: "/sequent_backend_candidate/:id",
            initialEntries: [`/sequent_backend_candidate/${STORY_IDS.candidate}`],
            layout: CandidateLayout,
        },
    },
    beforeEach: ({args}) => setUpCandidates(args),
    render: ({withRecord}) => <EditCandidateData record={withRecord ? aliceRecord() : undefined} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const englishName = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {name: "Name"})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await waitFor(async () =>
            expect(await englishName(canvasElement)).toHaveValue("Alice Example")
        )
        expect(reads("getOne", "sequent_backend_candidate")[0].args[1]).toMatchObject({
            id: STORY_IDS.candidate,
        })
    },
}

export const WithoutRecord: Story = {
    args: {withRecord: false},
    parameters: {
        expectedFailure: {
            reason: "The progress indicator shown until the record arrives has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
        expect(reads("getOne", "sequent_backend_candidate")).toEqual([])
    },
}

export const RenameFromThePresentation: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const name = await englishName(canvasElement)
        await waitFor(() => expect(name).toHaveValue("Alice Example"))
        await userEvent.clear(name)
        await userEvent.type(name, "Alice Sample")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        // The update is undoable: it reaches the service once its notification closes.
        const notification = await within(document.body).findByText("Element updated")
        expect(dataWrites()).toEqual([])
        await userEvent.click(canvasElement)
        await waitFor(() => expect(notification).not.toBeInTheDocument())
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        // The description follows the English presentation; since migration
        // 1772358027729 the name and alias live only in the presentation.
        expect(dataWrites()[0].params.data).not.toHaveProperty("name")
        expect(dataWrites()[0].params.data).not.toHaveProperty("alias")
        expect(dataWrites()[0].params).toMatchObject({
            id: STORY_IDS.candidate,
            data: {
                description: "Alice Example stands for the council",
                presentation: {
                    i18n: {en: {name: "Alice Sample"}},
                    language_conf: {enabled_language_codes: []},
                },
            },
        })
    },
}
