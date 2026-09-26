// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {CreateAreaContest} from "./CreateAreaContest"
import {
    AreaContestFixture,
    dataWrites,
    reads,
    setUpAreaContests,
    type AreaContestServices,
} from "./__stories__/AreaContestFixture"

const meta = {
    title: "Admin/Area contest/CreateAreaContest",
    component: CreateAreaContest,
    args: {reads: "records", empty: false},
    parameters: {
        router: {initialEntries: ["/sequent_backend_area_contest/create"]},
        expectedFailure: {
            reason: "The JSON inputs grey their item counts below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    beforeEach: ({args}) => setUpAreaContests(args),
    render: () => (
        <AreaContestFixture>
            <CreateAreaContest />
        </AreaContestFixture>
    ),
} satisfies WidgetMeta<AreaContestServices>
export default meta
type Story = StoryObj<AreaContestServices>

/**
 * Chooses the option with this value: the tenant options show a `username`
 * tenants do not have, so they have no visible name.
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
        await expect(canvas.getByText("Area Contest creation")).toBeVisible()
        await waitFor(() => expect(reads("getList", "sequent_backend_tenant")).toHaveLength(1))
        await expect(canvas.getByRole("button", {name: "Save"})).toBeVisible()
        await userEvent.click(canvas.getByRole("combobox", {name: "Tenant"}))
        const tenant = await within(document.body).findByRole("option", {name: "example-council"})
        await waitFor(() => expect(tenant).toBeVisible())
        expect(dataWrites()).toEqual([])
    },
}

export const CreateForAnAreaAndContest: Story = {
    play: async ({canvasElement}) => {
        await choose(canvasElement, "Tenant", TENANT_ID)
        // Event and contest names live in the presentation since migration 1772358027729.
        await userEvent.click(
            await within(canvasElement).findByRole("combobox", {name: "Election event"})
        )
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "Council event"})
        )
        // The event's contests and areas are offered once the event is chosen.
        await waitFor(() =>
            expect(reads("getList", "sequent_backend_area").at(-1)?.args[1]).toMatchObject({
                filter: {tenant_id: TENANT_ID, election_event_id: EVENT_ID},
            })
        )
        await userEvent.click(await within(canvasElement).findByRole("combobox", {name: "Contest"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "Council members"})
        )
        await userEvent.click(await within(canvasElement).findByRole("combobox", {name: "Area"}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "South district"})
        )
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0]).toEqual({
            method: "create",
            resource: "sequent_backend_area_contest",
            params: expect.objectContaining({
                data: expect.objectContaining({
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    contest_id: STORY_IDS.contest,
                    area_id: STORY_IDS.secondArea,
                }),
            }),
        })
        // React-admin opens the created record.
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent("/sequent_backend_area_contest/created-1")
        )
    },
}
