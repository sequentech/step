// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, eventPresentation, storyId} from "@/__stories__/fixtures"
import type {Sequent_Backend_Election} from "@/gql/graphql"
import {COUNCIL_ELECTION, DEPUTY_ELECTION, tallySession} from "./__stories__/TallyFixture"
import {TallyElectionsList} from "./TallyElectionsList"
import {EStoryWorkflow} from "../../../../ui-essentials/.storybook/globals"

const OTHER_CEREMONY = storyId(4, 9)
const ELECTIONS = [
    COUNCIL_ELECTION,
    {...DEPUTY_ELECTION, keys_ceremony_id: OTHER_CEREMONY},
] as Sequent_Backend_Election[]

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Tally/TallyElectionsList",
    component: TallyElectionsList,
    args: {
        elections: ELECTIONS,
        electionEventId: EVENT_ID,
        electionEventPresentation: eventPresentation,
        keysCeremonyId: null,
        disabled: false,
        update: fn(),
    },
    argTypes: {
        elections: {table: {disable: true}},
        tallySession: {table: {disable: true}},
        electionEventPresentation: {table: {disable: true}},
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <TallyElectionsList {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof TallyElectionsList>
export default meta
type Story = StoryObj<typeof meta>

const selection = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("checkbox", {name})

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        await expect(await selection(canvasElement, "Council")).toBeChecked()
        await expect(await selection(canvasElement, "Deputy")).toBeChecked()
        await waitFor(() =>
            expect(args.update).toHaveBeenLastCalledWith([
                STORY_IDS.election,
                STORY_IDS.secondElection,
            ])
        )
    },
}

export const DeselectAnElection: Story = {
    play: async ({canvasElement, args}) => {
        const deputy = await selection(canvasElement, "Deputy")
        await userEvent.click(deputy)
        await waitFor(() => expect(deputy).not.toBeChecked())
        expect(args.update).toHaveBeenLastCalledWith([STORY_IDS.election])
    },
}

export const ReadOnly: Story = {
    args: {disabled: true},
    play: async ({canvasElement}) => {
        await expect(await selection(canvasElement, "Council")).toBeDisabled()
        await expect(await selection(canvasElement, "Deputy")).toBeDisabled()
    },
}

export const KeysCeremonyLimitsTheElections: Story = {
    args: {keysCeremonyId: STORY_IDS.keysCeremony},
    play: async ({canvasElement, args}) => {
        await expect(await selection(canvasElement, "Council")).toBeChecked()
        expect(within(canvasElement).queryByRole("checkbox", {name: "Deputy"})).toBeNull()
        await waitFor(() => expect(args.update).toHaveBeenLastCalledWith([STORY_IDS.election]))
    },
}

export const ExistingTallyKeepsItsElections: Story = {
    args: {
        keysCeremonyId: STORY_IDS.keysCeremony,
        tallySession: tallySession(EStoryWorkflow.TALLY, {
            election_ids: [STORY_IDS.secondElection],
        }),
    },
    play: async ({canvasElement, args}) => {
        // A created tally lists its own elections, whatever the chosen keys ceremony.
        await expect(await selection(canvasElement, "Deputy")).toBeChecked()
        expect(within(canvasElement).queryByRole("checkbox", {name: "Council"})).toBeNull()
        await waitFor(() =>
            expect(args.update).toHaveBeenLastCalledWith([STORY_IDS.secondElection])
        )
    },
}

export const Empty: Story = {
    args: {elections: []},
    play: async ({canvasElement, args}) => {
        await expect(await within(canvasElement).findByText("No rows")).toBeVisible()
        expect(args.update).toHaveBeenLastCalledWith([])
    },
}
