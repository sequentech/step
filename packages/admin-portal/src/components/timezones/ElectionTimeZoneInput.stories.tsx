// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {i18n, timeZoneOption, type IElectionEventPresentation} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ElectionTimeZoneInput} from "./ElectionTimeZoneInput"
import {MyTimeZoneProvider, adminDateTimeFormat} from "./timeZoneService"
import {timeZoneContextOf} from "./useTimeZoneContext"
import {
    MY_TIME_ZONE,
    madridConfiguration,
    overseasConfiguration,
    type ITimeZoneConfiguration,
} from "./__fixtures__/configurations"

interface Scenario {
    configuration: "overseas" | "madrid"
    /** Which of the configuration's elections is edited. */
    election: number
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

const CONFIGURATIONS: Record<Scenario["configuration"], () => ITimeZoneConfiguration> = {
    overseas: overseasConfiguration,
    madrid: madridConfiguration,
}

const TEXT = {t: i18n.t, lang: "en", formatDateTime: adminDateTimeFormat("en")}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Timezones/ElectionTimeZoneInput",
    component: ElectionTimeZoneInput,
    args: {configuration: "overseas", election: 0, onSubmit: fn()},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: ({configuration, election, onSubmit}) => {
        const config = CONFIGURATIONS[configuration]()
        const edited = config.elections[election]
        return (
            <AdminStoryProvider boundary={boundary}>
                <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                    <SimpleForm
                        record={{id: edited.id, presentation: {timezone: edited.timezone}}}
                        onSubmit={onSubmit}
                        toolbar={
                            <Toolbar>
                                <SaveButton />
                            </Toolbar>
                        }
                    >
                        <ElectionTimeZoneInput
                            context={timeZoneContextOf(
                                config.presentation as IElectionEventPresentation
                            )}
                        />
                    </SimpleForm>
                </MyTimeZoneProvider>
            </AdminStoryProvider>
        )
    },
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const field = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("combobox", {name: i18n.t("lifecycle.settings.electionZone")})

/** tz-election: Dubai PCG in its own zone, chosen from the configured list. */
export const PostWithItsZone: Story = {
    play: async ({canvasElement}) => {
        const dubai = overseasConfiguration().elections[0]
        await expect(field(canvasElement)).toHaveValue(timeZoneOption(dubai.timezone!, TEXT).label)
    },
}

/** Empty uses the event's primary, which the field names. */
export const UsesThePrimary: Story = {
    args: {configuration: "madrid", election: 0},
    play: async ({canvasElement}) => {
        const primary = madridConfiguration().presentation.timezones!.primary
        await expect(field(canvasElement)).toHaveValue("")
        await expect(field(canvasElement)).toHaveAttribute(
            "placeholder",
            i18n.t("lifecycle.settings.electionPrimary", {
                zone: timeZoneOption(primary, TEXT).label,
            })
        )
    },
}

/** Only the configured zones are offered. */
export const OnlyConfiguredZones: Story = {
    args: {configuration: "madrid", election: 0},
    play: async ({canvasElement, args}) => {
        await userEvent.click(field(canvasElement))
        const options = await within(document.body).findAllByRole("option")
        expect(options).toHaveLength(
            madridConfiguration().presentation.timezones!.configured.length
        )
        const canary = timeZoneOption("Atlantic/Canary", TEXT).label
        await userEvent.click(options.find((option) => option.textContent?.startsWith(canary))!)
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalled())
        expect(args.onSubmit.mock.calls[0][0]).toMatchObject({
            presentation: {timezone: "Atlantic/Canary"},
        })
    },
}
