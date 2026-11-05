// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsLanguageSelector} from "./SettingsLanguageSelector"

interface Scenario {
    languageSettings: string[]
    canEdit: boolean
    /** The form's submit handler, which receives the edited election values. */
    onSubmit: (values: Record<string, unknown>) => void
}

/** An election presentation as ElectionDataForm edits it. */
const election = {
    id: "33333333-3333-4333-8333-333333333333",
    enabled_languages: {en: true, es: false},
    presentation: {language_conf: {default_language_code: "en"}},
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Components/SettingsLanguageSelector",
    component: SettingsLanguageSelector,
    args: {languageSettings: ["en", "es"], canEdit: true, onSubmit: fn()},
    parameters: {
        expectedFailure: {
            reason: "The default-language radio buttons have no accessible name.",
            a11y: ["label"],
        },
    },
    beforeEach: async () => {
        boundary = graphqlBoundary({}, {schema: true})
        await boundary.ready
    },
    render: ({onSubmit, ...args}) => (
        <AdminStoryProvider boundary={boundary}>
            <SimpleForm
                record={election}
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton />
                    </Toolbar>
                }
            >
                <SettingsLanguageSelector {...args} />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const language = (lang: string) => i18n.t(`common.language.${lang}`)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("switch", {name: language("en")})).toBeChecked()
        expect(canvas.getByRole("switch", {name: language("es")})).not.toBeChecked()
        const [english, spanish] = canvas.getAllByRole("radio")
        expect(english).toBeChecked()
        expect(spanish).not.toBeChecked()
    },
}

export const EnableLanguageAndMakeItDefault: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("switch", {name: language("es")}))
        await userEvent.click(canvas.getAllByRole("radio")[1])
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        expect(args.onSubmit).toHaveBeenCalledWith(
            expect.objectContaining({
                enabled_languages: {en: true, es: true},
                presentation: {language_conf: {default_language_code: "es"}},
            }),
            expect.anything()
        )
    },
}

export const ReadOnly: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("switch", {name: language("en")})).toBeDisabled()
        expect(canvas.getByRole("switch", {name: language("es")})).toBeDisabled()
        for (const radio of canvas.getAllByRole("radio")) expect(radio).toBeDisabled()
    },
}
