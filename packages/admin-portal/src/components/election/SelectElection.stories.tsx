// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, electionPresentation, electionRecord} from "@/__stories__/fixtures"
import SelectElection from "./SelectElection"

interface Scenario {
    /** The election the form already has, e.g. an existing report's. */
    value?: string
    isRequired: boolean
    disabled: boolean
    /** What reading the event's elections does. */
    reads: ReadState
    onSelectElection: Mock<(electionId: string) => void>
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const elections = [
    electionRecord(),
    electionRecord(undefined, {
        id: STORY_IDS.secondElection,
        presentation: electionPresentation("Deputy election"),
    }),
]

const meta = {
    title: "Admin/Election/SelectElection",
    component: SelectElection,
    args: {
        isRequired: true,
        disabled: false,
        reads: "records",
        onSelectElection: fn(),
        onSubmit: fn(),
    },
    argTypes: {
        value: {control: "select", options: [STORY_IDS.election, STORY_IDS.secondElection]},
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({sequent_backend_election: elections}, {reads: args.reads})
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({value, isRequired, disabled, onSelectElection, onSubmit}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <SimpleForm
                record={value ? {election_id: value} : {}}
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton />
                    </Toolbar>
                }
            >
                <SelectElection
                    tenantId={TENANT_ID}
                    electionEventId={EVENT_ID}
                    source="election_id"
                    label="Election"
                    onSelectElection={onSelectElection}
                    isRequired={isRequired}
                    disabled={disabled}
                    value={value}
                />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const electionInput = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("combobox", {name: /Election/})
const searches = () =>
    data.calls.filter(
        ({method, args}) => method === "getList" && args[0] === "sequent_backend_election"
    )
const eventScope = {tenant_id: TENANT_ID, election_event_id: EVENT_ID}

export const SavedElection: Story = {
    args: {value: STORY_IDS.election},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(electionInput(canvasElement)).toHaveValue("Council"))
        expect(data.calls).toContainEqual({
            method: "getMany",
            args: [
                "sequent_backend_election",
                expect.objectContaining({ids: [STORY_IDS.election]}),
            ],
        })
    },
}

/**
 * The search filters the election's name and alias columns, which migration
 * 1772358027729 moved into `presentation` and renamed to `external_id`, so no
 * election matches a search and none can be picked.
 */
export const SearchFindsNoElection: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.type(electionInput(canvasElement), "Deputy")
        // Every search stays within the tenant's event, 200 elections at a time.
        await waitFor(() =>
            expect(searches().at(-1)?.args[1]).toMatchObject({
                filter: {...eventScope, "name@_ilike,alias@_ilike": "Deputy"},
                pagination: {page: 1, perPage: 200},
            })
        )
        for (const {args: search} of searches()) {
            expect(search[1]).toMatchObject({filter: expect.objectContaining(eventScope)})
        }
        await waitFor(() => expect(within(document.body).queryAllByRole("option")).toHaveLength(0))
        expect(args.onSelectElection).not.toHaveBeenCalled()
    },
}

export const RequiredByDefault: Story = {
    play: async ({canvasElement}) => {
        await expect(electionInput(canvasElement)).toBeRequired()
    },
}

export const Optional: Story = {
    args: {isRequired: false},
    play: async ({canvasElement}) => {
        await expect(electionInput(canvasElement)).not.toBeRequired()
    },
}

export const Disabled: Story = {
    args: {disabled: true, value: STORY_IDS.secondElection},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(electionInput(canvasElement)).toHaveValue("Deputy"))
        await expect(electionInput(canvasElement)).toBeDisabled()
    },
}

export const LoadingElections: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(searches()).toHaveLength(1))
        await userEvent.click(electionInput(canvasElement))
        expect(within(document.body).queryByRole("option")).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(searches()).toHaveLength(1))
        await expect(
            await within(canvasElement).findByText("Synthetic service unavailable")
        ).toBeVisible()
    },
}
