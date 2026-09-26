// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {CreateBallotStyle} from "./CreateBallotStyle"
import {
    BallotStyleFixture,
    dataWrites,
    reads,
    setUpBallotStyles,
    type BallotStyleServices,
} from "./__stories__/BallotStyleFixture"

const meta = {
    title: "Admin/Ballot style/CreateBallotStyle",
    component: CreateBallotStyle,
    args: {reads: "records", empty: false},
    parameters: {
        router: {initialEntries: ["/sequent_backend_ballot_style/create"]},
        expectedFailure: {
            reason: "The JSON inputs grey their item counts below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    beforeEach: ({args}) => setUpBallotStyles(args),
    render: () => (
        <BallotStyleFixture>
            <CreateBallotStyle />
        </BallotStyleFixture>
    ),
} satisfies WidgetMeta<BallotStyleServices>
export default meta
type Story = StoryObj<BallotStyleServices>

/**
 * Chooses the option with this value: the tenant, event and election options
 * show fields those records no longer have, so they have no visible name.
 */
async function choose(canvasElement: HTMLElement, label: string, value: string) {
    await userEvent.click(await within(canvasElement).findByRole("combobox", {name: label}))
    const listbox = await within(document.body).findByRole("listbox")
    const option = listbox.querySelector<HTMLElement>(`[data-value="${value}"]`)
    if (!option) throw new Error(`No ${label} option ${value}`)
    await userEvent.click(option)
    await waitFor(() => expect(within(document.body).queryByRole("listbox")).toBeNull())
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Ballot Style creation")).toBeVisible()
        await waitFor(() => expect(reads("getList", "sequent_backend_tenant")).toHaveLength(1))
        await expect(canvas.getByRole("textbox", {name: "Ballot eml"})).toHaveValue("")
        expect(dataWrites()).toEqual([])
    },
}

export const CreateForAnArea: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.type(canvas.getByRole("textbox", {name: "Ballot eml"}), "{{}}")
        await choose(canvasElement, "Tenant", TENANT_ID)
        await choose(canvasElement, "Election event", EVENT_ID)
        await choose(canvasElement, "Election", STORY_IDS.election)
        await userEvent.click(await canvas.findByRole("combobox", {name: "Area"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "North district"})
        )
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0]).toEqual({
            method: "create",
            resource: "sequent_backend_ballot_style",
            params: expect.objectContaining({
                data: expect.objectContaining({
                    ballot_eml: "{}",
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    election_id: STORY_IDS.election,
                    area_id: STORY_IDS.area,
                }),
            }),
        })
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Current location"})).toHaveTextContent(
                "/sequent_backend_ballot_style/created-1"
            )
        )
    },
}
