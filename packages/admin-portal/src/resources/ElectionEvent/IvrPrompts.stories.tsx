// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, ResourceContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IvrPrompts} from "./IvrPrompts"
import {
    IVR_CONFIG,
    IVR_PHONE,
    IVR_PROMPTS,
    iconButton,
    ivrEvent,
    jsonEditorDefects,
} from "./__stories__/IvrFixture"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the event has an IVR flow and stored prompts. */
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
            {/* As in the event's edit view, whose resource the prompt list inherits. */}
            <ResourceContextProvider value="sequent_backend_election_event">
                <RecordContextProvider value={ivrEvent({configured})}>
                    <IvrPrompts />
                </RecordContextProvider>
            </ResourceContextProvider>
        </AdminStoryProvider>
    )
}

const listDefects = {
    expectedFailure: {
        reason:
            "The row actions are unnamed icon buttons, and json-edit-react's default theme " +
            "renders item counts and string values with insufficient contrast.",
        a11y: ["button-name", "color-contrast"],
    },
}

const meta = {
    title: "Admin/Election event/IvrPrompts",
    component: IvrPrompts,
    args: {configured: true, saveFails: false},
    parameters: {...listDefects, widgets: ["PromptsList"]},
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

const promptRows = async (canvasElement: HTMLElement) => {
    const table = await within(canvasElement).findByRole("table")
    return within(table)
        .getAllByRole("row")
        .slice(1)
        .map((row) => row.querySelector(".column-id")?.textContent)
}

const promptRow = (canvasElement: HTMLElement, key: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(key)})

/** The prompts the page's Save stores, as the saved annotation holds them. */
const savedPrompts = () => {
    const [write] = data.writes
    const params = write?.params as {data?: {annotations?: Record<string, string>}} | undefined
    return JSON.parse(params?.data?.annotations?.["ivr:prompts"] ?? "null")
}

const savePage = async (canvasElement: HTMLElement) => {
    const save = within(canvasElement).getAllByRole("button", {name: "Save"}).at(-1)
    if (!save) throw new Error("The page's Save button is missing")
    await userEvent.click(save)
    await waitFor(() => expect(data.writes).toHaveLength(1))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        // The prompts the flow announces come first and cannot be deleted.
        await waitFor(async () =>
            expect(await promptRows(canvasElement)).toEqual([
                "goodbye_prompt",
                "welcome_prompt",
                "menu_hint",
            ])
        )
        const welcome = await promptRow(canvasElement, "welcome_prompt")
        await expect(within(welcome).getByText("Welcome to the council vote")).toBeVisible()
        expect(iconButton(welcome, "DeleteIcon")).toBeNull()
        expect(iconButton(welcome, "EditIcon")).not.toBeNull()
        expect(iconButton(await promptRow(canvasElement, "menu_hint"), "DeleteIcon")).not.toBeNull()
        await expect(within(canvasElement).getByText("ivr:prompts")).toBeVisible()
        expect(data.writes).toEqual([])
    },
}

export const Empty: Story = {
    args: {configured: false},
    parameters: {...jsonEditorDefects, widgets: ["PromptsList", "EmptyPrompts"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No prompts created yet")).toBeVisible()
        expect(canvas.queryByRole("combobox", {name: "Select Language"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Add"}))
        const drawer = within(await within(document.body).findByRole("presentation"))
        await expect(await drawer.findByRole("textbox", {name: "Key"})).toHaveValue(
            "new_prompt_key"
        )
        await userEvent.keyboard("{Escape}")
        expect(data.writes).toEqual([])
    },
}

export const SpanishPrompts: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await promptRow(canvasElement, "menu_hint")
        await userEvent.click(canvas.getByRole("combobox", {name: "Select Language"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Spanish"}))
        const welcome = await promptRow(canvasElement, "welcome_prompt")
        await expect(within(welcome).getByText("Bienvenido a la votación")).toBeVisible()
    },
}

export const AddPromptAndSave: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await promptRow(canvasElement, "menu_hint")
        await userEvent.click(canvas.getByRole("button", {name: "Add"}))
        const drawer = within(await within(document.body).findByRole("presentation"))
        const key = await drawer.findByRole("textbox", {name: "Key"})
        await userEvent.clear(key)
        await userEvent.type(key, "help_hint")
        await userEvent.type(drawer.getByRole("textbox", {name: "Value"}), "Press 9 for help")
        await userEvent.click(drawer.getByRole("button", {name: "Save"}))
        await promptRow(canvasElement, "help_hint")
        expect(data.writes).toEqual([])
        await savePage(canvasElement)
        // A new key is added to every language with the same text.
        expect(savedPrompts()).toEqual({
            en: {...IVR_PROMPTS.en, goodbye_prompt: "", help_hint: "Press 9 for help"},
            es: {...IVR_PROMPTS.es, goodbye_prompt: "", help_hint: "Press 9 for help"},
        })
        expect(data.writes[0]).toMatchObject({
            method: "update",
            resource: "sequent_backend_election_event",
            params: {
                id: EVENT_ID,
                data: {
                    annotations: {
                        "ivr:config": JSON.stringify(IVR_CONFIG),
                        "ivr:phone-number": IVR_PHONE,
                    },
                },
            },
        })
        await expect(await within(document.body).findByText("Saved successfully")).toBeVisible()
    },
}

export const EditPrompt: Story = {
    play: async ({canvasElement}) => {
        const welcome = await promptRow(canvasElement, "welcome_prompt")
        await userEvent.click(iconButton(welcome, "EditIcon") as HTMLElement)
        const drawer = within(await within(document.body).findByRole("presentation"))
        await expect(await drawer.findByRole("textbox", {name: "Key"})).toHaveAttribute("readonly")
        const value = drawer.getByRole("textbox", {name: "Value"})
        await userEvent.clear(value)
        await userEvent.type(value, "Welcome, voter")
        await userEvent.click(drawer.getByRole("button", {name: "Save"}))
        await waitFor(async () =>
            expect(
                within(await promptRow(canvasElement, "welcome_prompt")).getByText("Welcome, voter")
            ).toBeVisible()
        )
        await savePage(canvasElement)
        expect(savedPrompts().en.welcome_prompt).toBe("Welcome, voter")
        expect(savedPrompts().es.welcome_prompt).toBe(IVR_PROMPTS.es.welcome_prompt)
    },
}

export const DeleteOptionalPrompt: Story = {
    play: async ({canvasElement}) => {
        const hint = await promptRow(canvasElement, "menu_hint")
        await userEvent.click(iconButton(hint, "DeleteIcon") as HTMLElement)
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(dialog.getByRole("button", {name: "Delete"}))
        await waitFor(async () =>
            expect(await promptRows(canvasElement)).toEqual(["goodbye_prompt", "welcome_prompt"])
        )
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        await savePage(canvasElement)
        expect(savedPrompts().en).not.toHaveProperty("menu_hint")
        expect(savedPrompts().es).not.toHaveProperty("menu_hint")
    },
}

export const CancelDiscardsEdits: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            iconButton(await promptRow(canvasElement, "menu_hint"), "DeleteIcon") as HTMLElement
        )
        await userEvent.click(
            within(await within(document.body).findByRole("dialog")).getByRole("button", {
                name: "Delete",
            })
        )
        await waitFor(async () => expect(await promptRows(canvasElement)).toHaveLength(2))
        await userEvent.click(canvas.getByRole("button", {name: "Cancel"}))
        await promptRow(canvasElement, "menu_hint")
        await expect(canvas.getByRole("button", {name: "Cancel"})).toBeDisabled()
        expect(data.writes).toEqual([])
    },
}

export const SaveFailure: Story = {
    args: {saveFails: true},
    play: async ({canvasElement}) => {
        await userEvent.click(
            iconButton(await promptRow(canvasElement, "menu_hint"), "DeleteIcon") as HTMLElement
        )
        await userEvent.click(
            within(await within(document.body).findByRole("dialog")).getByRole("button", {
                name: "Delete",
            })
        )
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        await savePage(canvasElement)
        await expect(await within(document.body).findByText("Failed to save")).toBeVisible()
    },
}
