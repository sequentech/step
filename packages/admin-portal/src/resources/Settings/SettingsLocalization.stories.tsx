// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import SettingsLocalization from "./SettingsLocalization"
import {TENANT_RESOURCE, settingsTenant} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the tenant does. */
    reads: ReadState
    /** Whether saving the tenant fails. */
    failure: boolean
}

const WELCOME_KEY = "adminPortal:header.welcome"

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsLocalization",
    component: SettingsLocalization,
    args: {reads: "records", failure: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "The override rows' edit and delete actions are unnamed icon buttons.",
            a11y: ["button-name"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[TENANT_RESOURCE]: [settingsTenant()]},
            {
                reads: args.reads,
                writeError: args.failure ? "Synthetic tenant service failure" : undefined,
            }
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <ResourceContextProvider value={TENANT_RESOURCE}>
                <SettingsLocalization />
            </ResourceContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (name: string) => i18n.t(`electionEventScreen.localization.labels.${name}`)
const notification = (name: string) => i18n.t(`electionEventScreen.localization.notify.${name}`)

const welcomeRow = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("row", {name: /Welcome, council/})

/** The English overrides the last update saved. */
const savedOverrides = () =>
    (
        data.writes.at(-1)?.params.data as {
            settings: {i18n: Record<string, Record<string, string>>}
        }
    ).settings.i18n.en

async function drawer() {
    const element = await within(document.body).findByRole("presentation")
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

async function expectNotification(text: string) {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const row = within(await welcomeRow(canvasElement))
        await expect(row.getByText("header.welcome")).toBeVisible()
        await expect(row.getByText("Admin portal")).toBeVisible()
        await expect(
            within(canvasElement).getByRole("combobox", {
                name: i18n.t("electionEventScreen.localization.selectLanguage"),
            })
        ).toHaveTextContent(i18n.t("common.language.en"))
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", TENANT_RESOURCE],
        ])
    },
}

async function addOverride(canvasElement: HTMLElement, key: string, value: string) {
    await welcomeRow(canvasElement)
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Add"}))
    const form = await drawer()
    await userEvent.type(form.getByRole("textbox", {name: label("key")}), key)
    await userEvent.type(form.getByRole("textbox", {name: label("value")}), value)
    await userEvent.click(form.getByRole("button", {name: "Save"}))
}

export const AddAnOverride: Story = {
    play: async ({canvasElement}) => {
        await addOverride(canvasElement, "footer.help", "Ask the council office")
        await expectNotification(notification("success"))
        expect(data.writes.map(({method, resource}) => [method, resource])).toEqual([
            ["update", TENANT_RESOURCE],
        ])
        expect(data.writes[0].params).toMatchObject({id: TENANT_ID})
        expect(savedOverrides()).toEqual({
            [WELCOME_KEY]: "Welcome, council",
            "adminPortal:footer.help": "Ask the council office",
        })
        await expect(
            await within(canvasElement).findByRole("row", {name: /Ask the council office/})
        ).toBeVisible()
    },
}

export const RejectADuplicateOverride: Story = {
    parameters: {
        expectedFailure: {
            reason: "The override drawer, left open, has no accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        await addOverride(canvasElement, "header.welcome", "Hello")
        await expectNotification(notification("duplicateKey"))
        expect(data.writes).toEqual([])
    },
}

export const EditAnOverride: Story = {
    play: async ({canvasElement}) => {
        const [edit] = within(await welcomeRow(canvasElement)).getAllByRole("button")
        await userEvent.click(edit)
        const form = await drawer()
        // The key identifies the override and cannot change.
        const key = form.getByRole("textbox", {name: label("key")})
        await expect(key).toHaveValue("header.welcome")
        expect(key).toHaveAttribute("readonly")
        const value = form.getByRole("textbox", {name: label("value")})
        await userEvent.clear(value)
        await userEvent.type(value, "Welcome back")
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await expectNotification(notification("success"))
        expect(savedOverrides()).toEqual({[WELCOME_KEY]: "Welcome back"})
    },
}

export const DeleteAnOverride: Story = {
    // The only row, with its unnamed icon buttons, is gone.
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const buttons = within(await welcomeRow(canvasElement)).getAllByRole("button")
        await userEvent.click(buttons[buttons.length - 1])
        const dialog = within(await within(document.body).findByRole("dialog"))
        expect(data.writes).toEqual([])
        await userEvent.click(dialog.getByRole("button", {name: "Delete"}))
        await expectNotification(notification("success"))
        expect(savedOverrides()).toEqual({})
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("row", {name: /Welcome, council/})).toBeNull()
        )
    },
}

export const SaveFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement}) => {
        await addOverride(canvasElement, "footer.help", "Ask the council office")
        await expectNotification(notification("error"))
        expect(data.writes.map(({method}) => method)).toEqual(["update"])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
        expect(within(canvasElement).queryByRole("row", {name: /Welcome, council/})).toBeNull()
    },
}
