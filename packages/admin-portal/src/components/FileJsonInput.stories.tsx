// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, ResourceContextProvider, SimpleForm} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, electionRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import FileJsonInput from "./FileJsonInput"

interface Scenario {
    /** The form's submit handler, called with the edited record. */
    onSubmit: (values: Record<string, unknown>) => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const election = {
    ...electionRecord(),
    configuration: {allow_early_voting: false, grace_period_secs: 60},
}

function Fixture({onSubmit}: Scenario) {
    return (
        <AdminStoryProvider boundary={boundary}>
            <ResourceContextProvider value="sequent_backend_election">
                <RecordContextProvider value={election}>
                    <SimpleForm record={election} onSubmit={onSubmit}>
                        <FileJsonInput
                            parsedValue={election}
                            fileSource="configuration"
                            jsonSource="presentation"
                        />
                    </SimpleForm>
                </RecordContextProvider>
            </ResourceContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Components/FileJsonInput",
    component: FileJsonInput,
    args: {onSubmit: fn()},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const ShowRecordJson: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getAllByText("Preview")[0]).toBeVisible()
        // The collapsed view of the election's presentation: i18n, language_conf and the report policy.
        await expect(await canvas.findByText("3 items")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(args.onSubmit).not.toHaveBeenCalled()
    },
}

export const ImportJsonConfiguration: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const input = canvas.getByLabelText("Preview", {selector: "input"})
        const file = new File(
            ['{"grace_period_secs": 120, "allow_revotes": true}'],
            "configuration.json",
            {type: "application/json"}
        )
        await userEvent.upload(input, file)
        const save = canvas.getByRole("button", {name: "Save"})
        await waitFor(() => expect(save).toBeEnabled())
        await userEvent.click(save)
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        // The file's settings are merged over the election's current configuration.
        expect(args.onSubmit).toHaveBeenCalledWith(
            expect.objectContaining({
                id: STORY_IDS.election,
                configuration: {
                    allow_early_voting: false,
                    grace_period_secs: 120,
                    allow_revotes: true,
                },
            }),
            expect.anything()
        )
    },
}
