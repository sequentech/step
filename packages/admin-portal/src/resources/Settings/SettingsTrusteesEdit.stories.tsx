// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsTrusteesEdit} from "./SettingsTrusteesEdit"
import {SECOND_TRUSTEE_ID, TRUSTEE_RESOURCE, trusteeRecords} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the trustee does. */
    reads: ReadState
    /** Whether saving the trustee fails. */
    failure: boolean
    /** Called when the drawer holding the form should close. */
    close: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsTrusteesEdit",
    component: SettingsTrusteesEdit,
    args: {reads: "records", failure: false, close: fn()},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[TRUSTEE_RESOURCE]: trusteeRecords()},
            {
                reads: args.reads,
                writeError: args.failure ? "Synthetic trustee failure" : undefined,
            }
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <SettingsTrusteesEdit id={SECOND_TRUSTEE_ID} close={close} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function keyInput(canvasElement: HTMLElement) {
    const input = await within(canvasElement).findByRole("textbox", {name: "Public key"})
    await waitFor(() => expect(input).toHaveValue("Q2VydGlmaWVkIGtleSB0d28"))
    return input
}

async function replaceTheKey(canvasElement: HTMLElement) {
    const input = await keyInput(canvasElement)
    await userEvent.clear(input)
    await userEvent.type(input, "bmV3IGtleQ")
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await keyInput(canvasElement)
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox", {name: "Name"})).toHaveValue("trustee2")
        await expect(canvas.getByText(i18n.t("trusteesSettingsScreen.edit.title"))).toBeVisible()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", TRUSTEE_RESOURCE],
        ])
    },
}

export const ReplaceThePublicKey: Story = {
    play: async ({canvasElement, args}) => {
        await replaceTheKey(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledOnce())
        expect(data.writes).toEqual([
            {
                method: "update",
                resource: TRUSTEE_RESOURCE,
                params: expect.objectContaining({
                    id: SECOND_TRUSTEE_ID,
                    data: expect.objectContaining({name: "trustee2", public_key: "bmV3IGtleQ"}),
                }),
            },
        ])
    },
}

export const SaveFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement, args}) => {
        await replaceTheKey(canvasElement)
        const message = await within(document.body).findByText("Synthetic trustee failure")
        await waitFor(() => expect(message).toBeVisible())
        expect(data.writes.map(({method}) => method)).toEqual(["update"])
        // The drawer stays open with the new key.
        expect(args.close).not.toHaveBeenCalled()
        await expect(within(canvasElement).getByRole("textbox", {name: "Public key"})).toHaveValue(
            "bmV3IGtleQ"
        )
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
        expect(within(canvasElement).queryByRole("textbox")).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement, args}) => {
        const message = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("textbox")).toBeNull()
        expect(args.close).not.toHaveBeenCalled()
    },
}
