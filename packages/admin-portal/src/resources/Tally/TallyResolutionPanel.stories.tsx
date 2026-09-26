// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {FIXED_TIME, STORY_IDS, storyId} from "@/__stories__/fixtures"
import {ITallyExecutionStatus} from "@/types/ceremonies"
import {
    COUNCIL_CONTEST,
    COUNCIL_ELECTION,
    DEPUTY_CONTEST,
    DEPUTY_ELECTION,
    TALLY_IDS,
    TallyStoryContext,
    tallyData,
    tallySession,
} from "./__stories__/TallyFixture"
import type {Sequent_Backend_Contest, Sequent_Backend_Election} from "@/gql/graphql"
import {TallyResolutionPanel} from "./TallyResolutionPanel"
import {EStoryWorkflow} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the tally has recorded any ties. */
    ties: boolean
    /** Whether applying the resolutions fails. */
    submitFails: boolean
    onResolutionSubmitted: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const scope = {tenant_id: STORY_IDS.tenant, election_event_id: EVENT_ID}
const PENDING_ID = storyId(5, 1)
const RESOLVED_ID = storyId(5, 2)

// The council contest is tied in its second round; the deputy tie was settled earlier.
const resolutions = [
    {
        ...scope,
        id: PENDING_ID,
        tally_session_id: STORY_IDS.tallySession,
        contest_id: STORY_IDS.contest,
        resolution_type: "irv_tie_break",
        status: "pending",
        resolution_data: {
            round_number: 2,
            tied_candidate_ids: [STORY_IDS.candidate, STORY_IDS.secondCandidate],
            vote_count: 40,
            method_used: "external-procedure",
        },
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    },
    {
        ...scope,
        id: RESOLVED_ID,
        tally_session_id: STORY_IDS.tallySession,
        contest_id: STORY_IDS.secondContest,
        resolution_type: "plurality_tie_break",
        status: "resolved",
        resolution_data: {
            tied_candidate_ids: [TALLY_IDS.deputyCandidate, TALLY_IDS.secondDeputyCandidate],
            vote_count: 19,
            method_used: "external-procedure",
            resolved_by_candidate_id: TALLY_IDS.deputyCandidate,
        },
        resolved_at: FIXED_TIME,
        resolved_by_user: "admin",
        created_at: "2025-01-01T09:00:00.000Z",
        last_updated_at: FIXED_TIME,
    },
]

// The council contest is tallied in both areas, the deputy contest only in the north.
const sessionContests = [
    [STORY_IDS.contest, STORY_IDS.election, STORY_IDS.area],
    [STORY_IDS.contest, STORY_IDS.election, STORY_IDS.secondArea],
    [STORY_IDS.secondContest, STORY_IDS.secondElection, STORY_IDS.area],
].map(([contest_id, election_id, area_id], index) => ({
    ...scope,
    id: storyId(6, index),
    tally_session_id: STORY_IDS.tallySession,
    session_id: index + 1,
    contest_id,
    election_id,
    area_id,
    created_at: FIXED_TIME,
    last_updated_at: FIXED_TIME,
}))

const meta = {
    title: "Admin/Tally/TallyResolutionPanel",
    component: TallyResolutionPanel,
    args: {ties: true, submitFails: false, onResolutionSubmitted: fn()},
    argTypes: {onResolutionSubmitted: {table: {disable: true}}},
    parameters: {
        expectedFailure: {
            reason: "The pending resolution chip has white text on orange below 4.5 contrast.",
            a11y: ["color-contrast"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            sequent_backend_tally_session_resolution: args.ties ? resolutions : [],
            sequent_backend_tally_session_contest: sessionContests,
        })
        graphql = graphqlBoundary(
            {
                SubmitTallyResolution: ({variables}) =>
                    args.submitFails
                        ? {errors: [new GraphQLError("Synthetic tally service unavailable")]}
                        : {
                              data: {
                                  submit_tally_resolution: {
                                      success: true,
                                      tally_session_id: STORY_IDS.tallySession,
                                      resolved_count: Array.isArray(variables.resolutions)
                                          ? variables.resolutions.length
                                          : 0,
                                  },
                              },
                          },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({onResolutionSubmitted}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <TallyStoryContext data={tallyData()}>
                <TallyResolutionPanel
                    tallySession={tallySession(EStoryWorkflow.TALLY, {
                        execution_status: ITallyExecutionStatus.AWAITING_INPUT,
                    })}
                    contests={[COUNCIL_CONTEST, DEPUTY_CONTEST] as Sequent_Backend_Contest[]}
                    elections={[COUNCIL_ELECTION, DEPUTY_ELECTION] as Sequent_Backend_Election[]}
                    electionEventId={EVENT_ID}
                    tenantId={STORY_IDS.tenant}
                    onResolutionSubmitted={onResolutionSubmitted}
                />
            </TallyStoryContext>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const selectionDefects = {
    expectedFailure: {
        reason:
            "The candidate picker is an Autocomplete without a label, and the pending " +
            "resolution chip has white text on orange below 4.5 contrast.",
        a11y: ["label", "color-contrast"],
    },
}

const PENDING_TITLE = /^Tie Resolution Required: Council \| Members \| Global \| Round 2/
const RESOLVED_TITLE = /^Tie Resolved: Deputy \| Deputy \| North district/

const item = (canvasElement: HTMLElement, title: RegExp) =>
    within(canvasElement).findByText((_, element) =>
        element?.tagName === "P" ? title.test(element.textContent ?? "") : false
    )

const applyButton = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {name: "Apply Resolutions and Recalculate"})

async function chooseCandidate(canvasElement: HTMLElement, name: string) {
    await userEvent.click(within(canvasElement).getByRole("combobox"))
    await userEvent.click(await within(document.body).findByRole("option", {name}))
}

async function decidePendingTie(canvasElement: HTMLElement) {
    await userEvent.click(await item(canvasElement, PENDING_TITLE))
    await chooseCandidate(canvasElement, "Alice")
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await item(canvasElement, PENDING_TITLE)).toBeVisible()
        await expect(await item(canvasElement, RESOLVED_TITLE)).toBeVisible()
        await expect(canvas.getByText("(1)")).toBeVisible()
        await expect(canvas.getByText("Pending Resolution")).toBeVisible()
        await expect(canvas.getByText("Resolved")).toBeVisible()
        await expect(canvas.getByText("Select an item from the left to view details")).toBeVisible()
        await expect(applyButton(canvasElement)).toBeDisabled()
        expect(data.calls.map(({method, args}) => `${method} ${String(args[0])}`)).toEqual(
            expect.arrayContaining([
                "getList sequent_backend_tally_session_resolution",
                "getList sequent_backend_tally_session_contest",
            ])
        )
        expect(graphql.calls).toEqual([])
    },
}

export const PendingTieDetails: Story = {
    parameters: selectionDefects,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await item(canvasElement, PENDING_TITLE))
        await expect(canvas.getByText("Tally paused due to unresolved tie (Round 2)")).toBeVisible()
        // The council contest counted 90 votes, so the 40 tied votes are 44.4% of them.
        await expect(
            canvas.getByText(
                "Candidates tied (40 votes, 44.4%): Alice, Bob. Manual tie-break required to continue tally."
            )
        ).toBeVisible()
        // Nothing can be saved until a candidate is chosen.
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        await userEvent.click(canvas.getByRole("combobox"))
        const options = within(await within(document.body).findByRole("listbox"))
            .getAllByRole("option")
            .map((option) => option.textContent)
        expect(options).toEqual(["Alice", "Bob"])
    },
}

export const ResolveTieAndApply: Story = {
    parameters: selectionDefects,
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await decidePendingTie(canvasElement)
        await expect(await canvas.findByText("Pending calculation")).toBeVisible()
        await expect(canvas.getByRole("combobox")).toBeDisabled()
        await userEvent.click(applyButton(canvasElement))
        await expect(
            await within(document.body).findByText("Resolutions submitted. Tally is resuming...")
        ).toBeVisible()
        expect(graphql.calls).toEqual([
            {
                name: "SubmitTallyResolution",
                variables: {
                    election_event_id: EVENT_ID,
                    tally_session_id: STORY_IDS.tallySession,
                    resolutions: [
                        {contest_id: STORY_IDS.contest, selected_candidate_id: STORY_IDS.candidate},
                    ],
                },
                headers: expect.objectContaining({"x-hasura-role": "tally-resolution-submit"}),
            },
        ])
        expect(args.onResolutionSubmitted).toHaveBeenCalledTimes(1)
        await waitFor(() => expect(canvas.queryByText("Pending calculation")).toBeNull())
    },
}

export const UndoADecision: Story = {
    parameters: selectionDefects,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await decidePendingTie(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Undo Resolution"}))
        await expect(canvas.getByRole("combobox")).toBeEnabled()
        await expect(canvas.getByRole("combobox")).toHaveValue("Alice")
        await expect(canvas.getByText("Pending Resolution")).toBeVisible()
        await expect(applyButton(canvasElement)).toBeDisabled()
    },
}

export const ChangeAResolvedTie: Story = {
    parameters: selectionDefects,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await item(canvasElement, RESOLVED_TITLE))
        await expect(canvas.getByText("Tally resumed after resolution applied")).toBeVisible()
        await expect(canvas.getByText(/was resolved on .* by admin$/)).toBeVisible()
        await expect(canvas.getByRole("combobox")).toHaveValue("Carol Example")
        await expect(canvas.getByRole("combobox")).toBeDisabled()
        await userEvent.click(canvas.getByRole("button", {name: "Undo Resolution"}))
        await chooseCandidate(canvasElement, "Dan Example")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await userEvent.click(applyButton(canvasElement))
        await waitFor(() =>
            expect(graphql.calls.map(({variables}) => variables.resolutions)).toEqual([
                [
                    {
                        contest_id: STORY_IDS.secondContest,
                        selected_candidate_id: TALLY_IDS.secondDeputyCandidate,
                    },
                ],
            ])
        )
    },
}

export const SubmitFailure: Story = {
    args: {submitFails: true},
    parameters: selectionDefects,
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await decidePendingTie(canvasElement)
        await userEvent.click(applyButton(canvasElement))
        await expect(
            await within(document.body).findByText(
                "Failed to submit resolutions. Please try again."
            )
        ).toBeVisible()
        expect(args.onResolutionSubmitted).not.toHaveBeenCalled()
        // The decision is kept so it can be applied again.
        await expect(canvas.getByText("Pending calculation")).toBeVisible()
        await expect(applyButton(canvasElement)).toBeEnabled()
    },
}

export const FilterByStatus: Story = {
    // Only the resolved tie is left, and nothing is selected.
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await item(canvasElement, PENDING_TITLE)
        await userEvent.click(canvas.getByRole("button", {name: "Filter"}))
        const popover = within(await within(document.body).findByRole("presentation"))
        await userEvent.click(popover.getByRole("combobox", {name: "Status"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Resolved"}))
        await userEvent.keyboard("{Escape}")
        await userEvent.keyboard("{Escape}")
        await expect(await canvas.findByRole("button", {name: "Filter (1)"})).toBeVisible()
        await waitFor(() => expect(canvas.queryByText(PENDING_TITLE)).toBeNull())
        await expect(await item(canvasElement, RESOLVED_TITLE)).toBeVisible()
    },
}

export const NoTies: Story = {
    args: {ties: false},
    parameters: {widgets: [], expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(data.calls.map(({args}) => args[0])).toContain(
                "sequent_backend_tally_session_resolution"
            )
        )
        // The panel stays hidden while the tally has nothing to resolve.
        expect(within(canvasElement).queryByText("Pending resolutions")).toBeNull()
        expect(canvasElement.querySelector("button")).toBeNull()
    },
}
