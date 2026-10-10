// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IvrConfig} from "./IvrConfig"
import {
    IVR_CONFIG,
    IVR_PHONE,
    IVR_PROMPTS,
    ivrEvent,
    jsonEditorDefects,
} from "./__stories__/IvrFixture"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the event has IVR annotations. */
    configured: boolean
    /** Whether saving the event fails. */
    saveFails: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({configured}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <RecordContextProvider value={ivrEvent({configured})}>
                <IvrConfig />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/IvrConfig",
    component: IvrConfig,
    args: {configured: true, saveFails: false},
    parameters: jsonEditorDefects,
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {sequent_backend_election_event: [ivrEvent({configured: args.configured})]},
            {writeError: args.saveFails ? "Synthetic save failure" : undefined}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const phoneInput = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {name: "Configured phone number"})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await phoneInput(canvasElement)).toHaveValue(IVR_PHONE)
        await expect(canvas.getByText("ivr:config")).toBeVisible()
        await expect(canvas.getByText(/"welcome_prompt"/)).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        await expect(canvas.getByRole("button", {name: "Cancel"})).toBeDisabled()
        expect(data.writes).toEqual([])
    },
}

export const Unconfigured: Story = {
    args: {configured: false},
    play: async ({canvasElement}) => {
        await expect(await phoneInput(canvasElement)).toHaveValue("")
        await expect(within(canvasElement).getByText("ivr:config")).toBeVisible()
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeDisabled()
    },
}

export const SavePhoneNumber: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const phone = await phoneInput(canvasElement)
        await userEvent.clear(phone)
        await userEvent.type(phone, " +34600000002 ")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "update",
                    resource: "sequent_backend_election_event",
                    params: expect.objectContaining({
                        id: EVENT_ID,
                        data: {
                            annotations: {
                                "ivr:config": JSON.stringify(IVR_CONFIG),
                                "ivr:phone-number": "+34600000002",
                                "ivr:prompts": JSON.stringify(IVR_PROMPTS),
                            },
                        },
                    }),
                },
            ])
        )
        await expect(await within(document.body).findByText("Saved successfully")).toBeVisible()
    },
}

export const CancelRestoresTheSavedValues: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const phone = await phoneInput(canvasElement)
        await userEvent.type(phone, "9")
        await expect(canvas.getByRole("button", {name: "Save"})).toBeEnabled()
        await userEvent.click(canvas.getByRole("button", {name: "Cancel"}))
        await expect(phone).toHaveValue(IVR_PHONE)
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(data.writes).toEqual([])
    },
}

export const SaveFailure: Story = {
    args: {saveFails: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.type(await phoneInput(canvasElement), "9")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await expect(await within(document.body).findByText("Failed to save")).toBeVisible()
        expect(data.writes.map(({method, resource}) => `${method} ${resource}`)).toEqual([
            "update sequent_backend_election_event",
        ])
        // The edit stays pending so that it can be saved again.
        await expect(canvas.getByRole("button", {name: "Save"})).toBeEnabled()
    },
}
