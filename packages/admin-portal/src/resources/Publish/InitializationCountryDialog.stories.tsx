// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, storyId} from "@/__stories__/fixtures"
import type {ILifecycleSnapshotEntry} from "@/queries/Lifecycle"
import {InitializationCountryDialog} from "./InitializationCountryDialog"

const A = storyId(80, 1)
const B = storyId(80, 2)
interface Scenario {
    membership: "retained" | "empty" | "unknown"
    onGenerate: (areaIds?: string[]) => Promise<boolean>
    onClose: () => void
}
let graphql: ReturnType<typeof graphqlBoundary>
const snapshots = (membership: Scenario["membership"]): ILifecycleSnapshotEntry[] => [
    {
        election_id: STORY_IDS.election,
        publication_id: storyId(80, 3),
        published_at: "2026-10-04T00:00:00Z",
        approval_request_id: null,
        approval_code: null,
        signed: false,
        snapshot: {
            schedule: [],
            ...(membership === "unknown"
                ? {}
                : {
                      initialization_countries: {
                          [STORY_IDS.election]: membership === "empty" ? [] : [A, B],
                      },
                  }),
        },
    },
]
const meta = {
    title: "Admin/Publish/InitializationCountryDialog",
    component: InitializationCountryDialog,
    args: {membership: "retained", onGenerate: fn(async () => true), onClose: fn()},
    beforeEach: () => {
        graphql = graphqlBoundary(
            {
                GetInitializationCountries: () => ({
                    data: {
                        sequent_backend_area: [
                            {id: A, name: "Country A", parent_id: null},
                            {id: B, name: "Country B", parent_id: null},
                        ],
                        // B's editable link is gone; its signed publication still retains it.
                        sequent_backend_area_contest: [
                            {area_id: A, contest: {election_id: STORY_IDS.election}},
                        ],
                        sequent_backend_ballot_style: [{area_id: A}, {area_id: B}],
                    },
                }),
            },
            {schema: true}
        )
        const boundary = graphql
        return () => expect(boundary.unexpected).toEqual([])
    },
    render: (args) => (
        <AdminStoryProvider
            boundary={
                args.membership === "retained"
                    ? graphql
                    : graphqlBoundary(
                          {
                              GetInitializationCountries: () => ({
                                  data: {
                                      sequent_backend_area: [],
                                      sequent_backend_area_contest: [],
                                      sequent_backend_ballot_style: [],
                                  },
                              }),
                          },
                          {schema: true}
                      )
            }
        >
            <InitializationCountryDialog
                electionEventId={EVENT_ID}
                electionId={STORY_IDS.election}
                busy={false}
                snapshots={snapshots(args.membership)}
                onGenerate={args.onGenerate}
                onClose={args.onClose}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const RetainedCountry: Story = {
    play: async ({args, canvasElement}) => {
        const body = within(canvasElement.ownerDocument.body)
        const field = await body.findByRole("combobox", {name: "Country"})
        await userEvent.click(field)
        await userEvent.click(await body.findByRole("option", {name: "Country B"}))
        await userEvent.click(body.getByRole("button", {name: "Generate Initialization Report"}))
        await waitFor(() => expect(args.onGenerate).toHaveBeenCalledWith([B]))
        expect(args.onClose).toHaveBeenCalledTimes(1)
    },
}
export const EntirePost: Story = {
    play: async ({args, canvasElement}) => {
        const body = within(canvasElement.ownerDocument.body)
        await body.findByRole("combobox", {name: "Country"})
        await userEvent.click(body.getByRole("button", {name: "Generate Initialization Report"}))
        await waitFor(() => expect(args.onGenerate).toHaveBeenCalledWith(undefined))
    },
}
export const ProvenEmpty: Story = {
    args: {membership: "empty"},
    play: async ({args, canvasElement}) => {
        const body = within(canvasElement.ownerDocument.body)
        await body.findByRole("combobox", {name: "Country"})
        await userEvent.click(body.getByRole("button", {name: "Generate Initialization Report"}))
        await waitFor(() => expect(args.onGenerate).toHaveBeenCalledWith(undefined))
    },
}
export const UnknownLegacy: Story = {
    args: {membership: "unknown"},
    play: async ({args, canvasElement}) => {
        const body = within(canvasElement.ownerDocument.body)
        await body.findByText(/no eligible countries/)
        expect(body.getByRole("button", {name: "Generate Initialization Report"})).toBeDisabled()
        expect(args.onGenerate).not.toHaveBeenCalled()
    },
}
