// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, trusteeRecords} from "@/__stories__/fixtures"
import {COUNCIL_ELECTION, DEPUTY_ELECTION, tallySession} from "./__stories__/TallyFixture"
import {EditTally} from "./EditTally"
import {EStoryWorkflow} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    reads: ReadState
    /** A save fails with this message. */
    writeError?: string
    close: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Tally/EditTally",
    component: EditTally,
    args: {reads: "records", close: fn()},
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        close: {table: {disable: true}},
    },
    beforeEach: ({args}) => {
        graphql = graphqlBoundary({})
        data = resourceBoundary(
            {
                sequent_backend_election: [COUNCIL_ELECTION, DEPUTY_ELECTION],
                sequent_backend_trustee: trusteeRecords,
                sequent_backend_tally_session: [
                    tallySession(EStoryWorkflow.TALLY, {
                        election_ids: [STORY_IDS.election, STORY_IDS.secondElection],
                    }),
                ],
            },
            {reads: args.reads, writeError: args.writeError}
        )
    },
    render: ({close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <EditTally id={STORY_IDS.tallySession} electionEventId={EVENT_ID} close={close} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/**
 * The checkbox of an election. The widget labels each one with the election's
 * `name`, a column elections do not have, so every label reads "undefined".
 */
const election = (canvasElement: HTMLElement, id: string) =>
    waitFor(() => {
        const checkbox = canvasElement.querySelector<HTMLInputElement>(
            `input[type="checkbox"][value="${id}"]`
        )
        if (!checkbox) throw new Error(`No checkbox for election ${id}`)
        return checkbox
    })

/** The tally's elections are checked once its record has loaded into the form. */
async function loadedElection(canvasElement: HTMLElement, id: string) {
    const checkbox = await election(canvasElement, id)
    await waitFor(() => expect(checkbox).toBeChecked())
    return checkbox
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await loadedElection(canvasElement, STORY_IDS.election)
        const deputy = await loadedElection(canvasElement, STORY_IDS.secondElection)
        expect(deputy).toHaveAccessibleName("undefined")
        await expect(within(canvasElement).getByText(i18n.t("tally.common.title"))).toBeVisible()
        expect(data.calls.map(({method, args}) => `${method} ${String(args[0])}`)).toEqual(
            expect.arrayContaining([
                "getList sequent_backend_election",
                "getList sequent_backend_trustee",
                "getOne sequent_backend_tally_session",
            ])
        )
        expect(data.writes).toEqual([])
    },
}

export const SaveChangedElections: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await loadedElection(canvasElement, STORY_IDS.secondElection))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(data.writes).toEqual([
                expect.objectContaining({
                    method: "update",
                    resource: "sequent_backend_tally_session",
                    params: expect.objectContaining({
                        id: STORY_IDS.tallySession,
                        data: expect.objectContaining({election_ids: [STORY_IDS.election]}),
                    }),
                }),
            ])
        )
        await expect(await within(document.body).findByText("Area updated")).toBeVisible()
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
    },
}

export const SaveFailure: Story = {
    args: {writeError: "Synthetic update rejected"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await loadedElection(canvasElement, STORY_IDS.secondElection))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await expect(await within(document.body).findByText("Could not update Area")).toBeVisible()
        expect(data.writes).toHaveLength(1)
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(data.calls.map(({args}) => args[0])).toContain("sequent_backend_election")
        )
        // Nothing is shown until the elections and trustees have loaded.
        expect(within(canvasElement).queryByRole("checkbox")).toBeNull()
        expect(within(canvasElement).queryByRole("button", {name: "Save"})).toBeNull()
    },
}
