// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ELanguageDetectionPolicy, i18n, initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {LANGUAGE_CONF} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsLanguages} from "./SettingsLanguages"
import {TENANT_RESOURCE, settingsTab, settingsTenant} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the tenant does. */
    reads: ReadState
}

const Tab = settingsTab(SettingsLanguages)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsLanguages",
    component: SettingsLanguages,
    args: {reads: "records"},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "The language switches are not labelled by the language name beside them.",
            a11y: ["label"],
        },
    },
    beforeEach: async ({args}) => {
        await initCore()
        data = resourceBoundary(
            {
                [TENANT_RESOURCE]: [
                    settingsTenant({
                        language_conf: {
                            ...LANGUAGE_CONF,
                            language_detection_policy: ELanguageDetectionPolicy.BROWSER_DETECT,
                        },
                    }),
                ],
            },
            {reads: args.reads}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <Tab />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const languageName = (code: string) => i18n.t("language", {lng: code})

const languageSwitch = async (canvasElement: HTMLElement, code: string) => {
    const label = await within(canvasElement).findByText(languageName(code), {selector: "span"})
    return within(label.parentElement as HTMLElement).getByRole("switch")
}

async function loaded(canvasElement: HTMLElement) {
    await waitFor(async () => expect(await languageSwitch(canvasElement, "es")).toBeChecked())
}

/** Closes the undo notification so that the undoable change is written. */
async function commit() {
    await within(document.body).findByText("Element updated")
    expect(data.writes).toEqual([])
    await userEvent.keyboard("{Escape}")
}

const savedLanguageConf = () =>
    (data.writes.at(-1)?.params.data as {settings: {language_conf: unknown}}).settings.language_conf

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await loaded(canvasElement)
        await expect(await languageSwitch(canvasElement, "en")).toBeChecked()
        await expect(await languageSwitch(canvasElement, "fr")).not.toBeChecked()
        await expect(
            canvas.getByRole("combobox", {name: i18n.t("settings.languages.default")})
        ).toHaveTextContent(languageName("en"))
        await expect(
            canvas.getByRole("combobox", {
                name: i18n.t("electionEventScreen.field.languageDetectionPolicy.policyLabel"),
            })
        ).toHaveTextContent("Browser Detect")
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", TENANT_RESOURCE],
        ])
    },
}

export const EnableALanguage: Story = {
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await userEvent.click(await languageSwitch(canvasElement, "fr"))
        await commit()
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "update",
                    resource: TENANT_RESOURCE,
                    params: expect.objectContaining({id: TENANT_ID}),
                },
            ])
        )
        expect(savedLanguageConf()).toEqual({
            enabled_language_codes: ["en", "es", "fr"],
            default_language_code: "en",
            language_detection_policy: ELanguageDetectionPolicy.BROWSER_DETECT,
        })
    },
}

export const ChooseTheDefaultLanguage: Story = {
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await userEvent.click(
            within(canvasElement).getByRole("combobox", {
                name: i18n.t("settings.languages.default"),
            })
        )
        const options = within(await within(document.body).findByRole("listbox"))
        // Only the enabled languages can be the default.
        expect(options.getAllByRole("option").map((option) => option.textContent)).toEqual([
            languageName("en"),
            languageName("es"),
        ])
        await userEvent.click(options.getByRole("option", {name: languageName("es")}))
        await commit()
        await waitFor(() => expect(data.writes).toHaveLength(1))
        expect(savedLanguageConf()).toMatchObject({default_language_code: "es"})
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
        expect(within(canvasElement).queryByRole("switch")).toBeNull()
    },
}
