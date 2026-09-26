// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {ContestsOrder} from "@sequentech/ui-core"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    STORY_IDS,
    contestRecord,
    electionPresentation,
    electionRecord,
} from "@/__stories__/fixtures"
import {EditElectionData} from "./ElectionData"
import {
    ElectionLayout,
    contests,
    dataWrites,
    reads,
    setUpElections,
    type ElectionServices,
} from "./__stories__/ElectionFixture"
import {EStoryPermissions, EStoryWorkflow} from "../../../../ui-essentials/.storybook/globals"

interface Scenario extends ElectionServices {
    /** Whether the ballot lists the contests in a custom order. */
    customOrder: boolean
}

/** The council election listing its contests in a custom order not yet saved. */
const customOrderElection = () =>
    electionRecord(EStoryWorkflow.ENDED, {
        presentation: {
            ...electionPresentation("Council election"),
            contests_order: ContestsOrder.CUSTOM,
        },
    })

const unorderedContests = () =>
    contests().map((contest, index) =>
        index === 0
            ? contestRecord({presentation: {...contest.presentation, sort_order: 5}})
            : contest
    )

const meta = {
    title: "Admin/Election/EditElectionData",
    component: EditElectionData,
    args: {reads: "records", empty: false, customOrder: false},
    parameters: {
        router: {
            path: "/sequent_backend_election/:id",
            initialEntries: [`/sequent_backend_election/${STORY_IDS.election}`],
            layout: ElectionLayout,
        },
    },
    beforeEach: ({args}) =>
        setUpElections(
            args,
            {},
            args.customOrder
                ? {elections: [customOrderElection()], contests: unorderedContests()}
                : {}
        ),
    render: ({customOrder}) => (
        <RecordContextProvider value={customOrder ? customOrderElection() : electionRecord()}>
            <EditElectionData />
        </RecordContextProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const nameInput = (canvasElement: HTMLElement) =>
    within(canvasElement).findByDisplayValue("Council election")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await nameInput(canvasElement)).toBeVisible()
        expect(canvas.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
            "English",
            "Spanish",
        ])
        await expect(canvas.getByRole("button", {name: "Save"})).toBeVisible()
        expect(reads("getOne", "sequent_backend_election")[0].args[1]).toMatchObject({
            id: STORY_IDS.election,
        })
    },
}

export const WithoutElectionWrite: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    play: async ({canvasElement}) => {
        await expect(await nameInput(canvasElement)).toBeVisible()
        expect(within(canvasElement).queryByRole("button", {name: "Save"})).toBeNull()
    },
}

async function saveAndConfirm(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
    // The update is undoable: it reaches the service once its notification closes.
    const notification = await within(document.body).findByText("Element updated")
    await userEvent.click(canvasElement)
    await waitFor(() => expect(notification).not.toBeInTheDocument())
}

const electionUpdates = () =>
    dataWrites().filter(({resource}) => resource === "sequent_backend_election")

export const RenameFromThePresentation: Story = {
    play: async ({canvasElement}) => {
        const name = await nameInput(canvasElement)
        await userEvent.clear(name)
        await userEvent.type(name, "Council vote")
        await saveAndConfirm(canvasElement)
        await waitFor(() => expect(electionUpdates()).toHaveLength(1))
        // Since migration 1772358027729 the name and alias live only in the presentation.
        expect(electionUpdates()[0].params.data).not.toHaveProperty("name")
        expect(electionUpdates()[0].params.data).not.toHaveProperty("alias")
        expect(electionUpdates()[0].params).toMatchObject({
            id: STORY_IDS.election,
            data: {
                description: "Choose the council members",
                presentation: {i18n: {en: {name: "Council vote"}}},
            },
        })
        expect(dataWrites().filter(({resource}) => resource === "sequent_backend_contest")).toEqual(
            []
        )
    },
}

export const CustomOrderRenumbersContests: Story = {
    args: {customOrder: true},
    play: async ({canvasElement}) => {
        const name = await nameInput(canvasElement)
        await userEvent.type(name, " 2026")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        // The contests take their place in the list as their sort order before the
        // election is saved.
        await waitFor(() =>
            expect(
                dataWrites().filter(({resource}) => resource === "sequent_backend_contest")
            ).toHaveLength(2)
        )
        expect(
            dataWrites()
                .filter(({resource}) => resource === "sequent_backend_contest")
                .map(({params}) => params)
        ).toEqual([
            expect.objectContaining({
                id: STORY_IDS.secondContest,
                data: {presentation: expect.objectContaining({sort_order: 0})},
            }),
            expect.objectContaining({
                id: STORY_IDS.contest,
                data: {presentation: expect.objectContaining({sort_order: 1})},
            }),
        ])
        const notification = await within(document.body).findByText("Element updated")
        await waitFor(() => expect(notification).toBeVisible())
    },
}
