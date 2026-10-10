// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsTrusteesCreate} from "./SettingsTrusteesCreate"
import {TENANT_RESOURCE, TRUSTEE_RESOURCE, settingsTenant} from "./__stories__/SettingsFixture"

interface Scenario {
    /** Whether creating the trustee fails. */
    failure: boolean
    /** Called when the drawer holding the form should close. */
    close: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsTrusteesCreate",
    component: SettingsTrusteesCreate,
    args: {failure: false, close: fn()},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[TRUSTEE_RESOURCE]: [], [TENANT_RESOURCE]: [settingsTenant()]},
            {writeError: args.failure ? "Synthetic trustee failure" : undefined}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <SettingsTrusteesCreate close={close} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function createTrustee(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await expect(
        await canvas.findByText(i18n.t("trusteesSettingsScreen.create.title"))
    ).toBeVisible()
    await userEvent.type(canvas.getByRole("textbox", {name: "Name"}), "trustee3")
    await userEvent.type(canvas.getByRole("textbox", {name: "Public key"}), "a2V5IHRocmVl")
    await userEvent.click(canvas.getByRole("button", {name: "Save"}))
}

export const Empty: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("textbox", {name: "Name"})).toHaveValue("")
        await expect(canvas.getByRole("textbox", {name: "Public key"})).toHaveValue("")
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(data.writes).toEqual([])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const CreateATrustee: Story = {
    play: async ({canvasElement, args}) => {
        await createTrustee(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledOnce())
        expect(data.writes).toEqual([
            {
                method: "create",
                resource: TRUSTEE_RESOURCE,
                params: {
                    data: {name: "trustee3", public_key: "a2V5IHRocmVl", tenant_id: TENANT_ID},
                },
            },
        ])
    },
}

export const CreateFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement, args}) => {
        await createTrustee(canvasElement)
        const message = await within(document.body).findByText("Synthetic trustee failure")
        await waitFor(() => expect(message).toBeVisible())
        expect(args.close).toHaveBeenCalledOnce()
        expect(data.writes.map(({method}) => method)).toEqual(["create"])
    },
}
