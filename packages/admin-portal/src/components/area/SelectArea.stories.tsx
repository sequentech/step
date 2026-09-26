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
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, areaRecords} from "@/__stories__/fixtures"
import SelectArea from "./SelectArea"

interface Scenario {
    /** The parent area the form already has. */
    parentId?: string
    isRequired: boolean
    disabled: boolean
    onSelectArea: Mock<(areaId: string) => void>
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Area/SelectArea",
    component: SelectArea,
    args: {isRequired: false, disabled: false, onSelectArea: fn(), onSubmit: fn()},
    argTypes: {parentId: {control: "select", options: [STORY_IDS.area, STORY_IDS.secondArea]}},
    beforeEach: async () => {
        data = resourceBoundary({sequent_backend_area: areaRecords()})
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({parentId, isRequired, disabled, onSelectArea, onSubmit}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <SimpleForm
                record={parentId ? {parent_id: parentId} : {}}
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton />
                    </Toolbar>
                }
            >
                <SelectArea
                    tenantId={TENANT_ID}
                    electionEventId={EVENT_ID}
                    source="parent_id"
                    label="Parent area"
                    onSelectArea={onSelectArea}
                    isRequired={isRequired}
                    disabled={disabled}
                />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const areaInput = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("combobox", {name: /Parent area/})
const searches = () => data.calls.filter(({method}) => method === "getList")

export const SavedParent: Story = {
    args: {parentId: STORY_IDS.area},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(areaInput(canvasElement)).toHaveValue("North district"))
        expect(data.calls).toContainEqual({
            method: "getMany",
            args: ["sequent_backend_area", expect.objectContaining({ids: [STORY_IDS.area]})],
        })
        // The choices load at once: enableGetChoices reads `q`, which the search never sets.
        expect(searches().map(({args}) => args[1])).toEqual([
            expect.objectContaining({filter: {tenant_id: TENANT_ID, election_event_id: EVENT_ID}}),
        ])
    },
}

export const SearchAndSelect: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.type(areaInput(canvasElement), "South district")
        await waitFor(() =>
            expect(searches().map(({args}) => args[1])).toContainEqual(
                expect.objectContaining({
                    filter: {
                        tenant_id: TENANT_ID,
                        election_event_id: EVENT_ID,
                        name: "South district",
                    },
                    pagination: {page: 1, perPage: 100},
                })
            )
        )
        await waitFor(() =>
            expect(
                within(document.body)
                    .getAllByRole("option")
                    .map((option) => option.textContent)
            ).toEqual(["South district"])
        )
        await userEvent.click(within(document.body).getByRole("option", {name: "South district"}))
        expect(args.onSelectArea).toHaveBeenLastCalledWith(STORY_IDS.secondArea, expect.anything())
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        expect(args.onSubmit.mock.calls[0][0]).toEqual({parent_id: STORY_IDS.secondArea})
    },
}

export const Required: Story = {
    args: {isRequired: true},
    play: async ({canvasElement}) => {
        await expect(areaInput(canvasElement)).toBeRequired()
    },
}

export const Disabled: Story = {
    args: {disabled: true, parentId: STORY_IDS.secondArea},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(areaInput(canvasElement)).toHaveValue("South district"))
        await expect(areaInput(canvasElement)).toBeDisabled()
    },
}
