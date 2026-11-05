// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, ResourceContextProvider, SaveContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {eventRecord} from "@/__stories__/fixtures"
import {EditElectionEventAreasList} from "./EditElectionEventAreasList"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** The save of the surrounding edit view. */
    save: (values: Record<string, unknown>) => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>

// The election event has no name or alias columns, so those inputs start empty.
const record = eventRecord()

function Fixture({save}: Scenario) {
    const {tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} tenant={tenant}>
            <SaveContextProvider value={{save, saving: false, mutationMode: "pessimistic"}}>
                <ResourceContextProvider value="sequent_backend_election_event">
                    <RecordContextProvider value={record}>
                        <EditElectionEventAreasList />
                    </RecordContextProvider>
                </ResourceContextProvider>
            </SaveContextProvider>
        </AdminStoryProvider>
    )
}

// The form reads nothing: every value comes from the record in context.
const meta = {
    title: "Admin/Election event/EditElectionEventAreasList",
    component: EditElectionEventAreasList,
    args: {save: fn()},
    parameters: {
        expectedFailure: {
            reason: "The section titles are h5 headings inside the h3 of their accordion summaries.",
            a11y: ["heading-order"],
        },
    },
    beforeEach: async () => {
        data = dataBoundary({})
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const field = (key: string) => i18n.t(`electionEventScreen.field.${key}`)
const section = (key: string) => i18n.t(`electionEventScreen.edit.${key}`)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("tab", {name: "English", selected: true})).toBeVisible()
        await expect(canvas.getByRole("textbox", {name: field("description")})).toHaveValue(
            "Synthetic election event for stories"
        )
        await expect(canvas.getByRole("textbox", {name: field("name")})).toHaveValue("")
        await expect(canvas.getByRole("textbox", {name: field("alias")})).toHaveValue("")
        // Only the general section starts expanded.
        expect(canvas.getByRole("button", {name: section("general")})).toHaveAttribute(
            "aria-expanded",
            "true"
        )
        expect(canvas.getByRole("button", {name: section("dates")})).toHaveAttribute(
            "aria-expanded",
            "false"
        )
        expect(data.calls).toEqual([])
        expect(graphql.calls).toEqual([])
    },
}

export const SpanishTab: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("tab", {name: "Spanish"}))
        await expect(canvas.getByRole("tab", {name: "Spanish", selected: true})).toBeVisible()
        // Both languages edit the same record fields.
        await expect(canvas.getByRole("textbox", {name: field("description")})).toHaveValue(
            "Synthetic election event for stories"
        )
    },
}

export const OpenTheLanguageSection: Story = {
    parameters: {
        expectedFailure: {
            reason: "The section titles are h5 headings inside h3 summaries, and the collapsed general section keeps an unnamed region beside the expanded one.",
            a11y: ["heading-order", "landmark-unique"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: section("language")}))
        await waitFor(() =>
            expect(canvas.getByRole("button", {name: section("general")})).toHaveAttribute(
                "aria-expanded",
                "false"
            )
        )
        await expect(await canvas.findByRole("switch", {name: "English"})).toBeChecked()
        await expect(canvas.getByRole("switch", {name: "Spanish"})).not.toBeChecked()
    },
}

export const SaveTheForm: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.type(canvas.getByRole("textbox", {name: field("name")}), "District areas")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.save).toHaveBeenCalledTimes(1))
        // The unopened sections submit their default switches.
        expect(args.save).toHaveBeenCalledWith(
            expect.objectContaining({
                name: "District areas",
                description: "Synthetic election event for stories",
                language: {english: true, spanish: false},
                allowed: {one: true, two: true},
            }),
            expect.anything()
        )
        expect(data.calls).toEqual([])
    },
}
