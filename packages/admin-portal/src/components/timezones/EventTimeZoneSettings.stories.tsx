// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {i18n, timeZoneOption, timeZonePrimaryOptionLabel, zoneLabel} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EventTimeZoneSettings} from "./EventTimeZoneSettings"
import {MyTimeZoneProvider, adminDateTimeFormat} from "./timeZoneService"
import {
    MY_TIME_ZONE,
    madridConfiguration,
    overseasConfiguration,
    type ITimeZoneConfiguration,
} from "./__fixtures__/configurations"

interface Scenario {
    configuration: "overseas" | "madrid"
    /** The form's submit handler, which receives the event's values. */
    onSubmit: (values: Record<string, unknown>) => void
}

const CONFIGURATIONS: Record<Scenario["configuration"], () => ITimeZoneConfiguration> = {
    overseas: overseasConfiguration,
    madrid: madridConfiguration,
}

const TEXT = {t: i18n.t, lang: "en", formatDateTime: adminDateTimeFormat("en")}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Timezones/EventTimeZoneSettings",
    component: EventTimeZoneSettings,
    args: {configuration: "overseas", onSubmit: fn()},
    argTypes: {configuration: {control: "inline-radio", options: ["overseas", "madrid"]}},
    beforeEach: async () => {
        boundary = graphqlBoundary({})
        await boundary.ready
    },
    render: ({configuration, onSubmit}) => {
        const config = CONFIGURATIONS[configuration]()
        return (
            <AdminStoryProvider boundary={boundary}>
                <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                    <SimpleForm
                        record={{id: "event", presentation: config.presentation}}
                        onSubmit={onSubmit}
                        toolbar={
                            <Toolbar>
                                <SaveButton />
                            </Toolbar>
                        }
                    >
                        <EventTimeZoneSettings
                            elections={config.elections.map(({id, name, timezone}) => ({
                                id,
                                name,
                                zone: timezone,
                            }))}
                        />
                    </SimpleForm>
                </MyTimeZoneProvider>
            </AdminStoryProvider>
        )
    },
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const settings = (key: string, options?: Record<string, unknown>) =>
    i18n.t(`lifecycle.settings.${key}`, options)

/** tz-settings: 80 configured zones, the primary marked, the rest collapsed into "+N". */
export const OverseasPreset: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const {timezones} = overseasConfiguration().presentation
        await expect(
            await canvas.findByText(
                settings("configuredHelp", {count: timezones!.configured.length})
            )
        ).toBeVisible()
        const primary = timeZoneOption(timezones!.primary, TEXT)
        await expect(canvas.getByText(timeZonePrimaryOptionLabel(primary, TEXT))).toBeVisible()
        await expect(
            canvas.getByText(settings("moreZones", {count: timezones!.configured.length - 8}))
        ).toBeVisible()
        await expect(
            canvas.getByRole("radio", {
                name: settings("logsPrimary", {abbr: zoneLabel(timezones!.primary, TEXT)}),
            })
        ).toBeChecked()
    },
}

/** The Madrid association: two zones, logs per election. */
export const MadridAssociation: Story = {
    args: {configuration: "madrid"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const {timezones} = madridConfiguration().presentation
        for (const zone of timezones!.configured) {
            const option = timeZoneOption(zone, TEXT)
            const label =
                zone === timezones!.primary
                    ? timeZonePrimaryOptionLabel(option, TEXT)
                    : option.label
            await expect(await canvas.findByText(label)).toBeVisible()
        }
        await expect(canvas.getByRole("radio", {name: settings("logsElection")})).toBeChecked()
    },
}

/** A zone an election uses can't be removed: the field says which elections use it. */
export const ZoneInUse: Story = {
    args: {configuration: "madrid"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const canary = madridConfiguration().elections.find(({timezone}) => timezone)!
        const chip = (await canvas.findByText(timeZoneOption(canary.timezone!, TEXT).label))
            .parentElement!
        await userEvent.click(within(chip).getByTestId("CancelIcon"))
        await expect(
            await canvas.findByText(
                settings("inUse", {
                    zone: timeZoneOption(canary.timezone!, TEXT).label,
                    names: canary.name,
                })
            )
        ).toBeVisible()
        // Nothing was removed: the chip stays and the form has nothing to save.
        await expect(canvas.getByText(timeZoneOption(canary.timezone!, TEXT).label)).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(args.onSubmit).not.toHaveBeenCalled()
    },
}

/** Choosing another primary and the logs policy saves both. */
export const ChangeThePrimary: Story = {
    args: {configuration: "madrid"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const primary = await canvas.findByRole("combobox", {name: settings("primary")})
        await userEvent.click(primary)
        const canary = timeZoneOption("Atlantic/Canary", TEXT)
        await userEvent.click(
            await within(document.body).findByRole("option", {
                name: new RegExp(canary.label.replace(/[()+]/g, "\\$&")),
            })
        )
        await userEvent.click(canvas.getByRole("radio", {name: /Primary timezone/}))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(args.onSubmit).toHaveBeenCalledWith(
                expect.objectContaining({
                    presentation: expect.objectContaining({
                        timezones: {
                            configured: ["Europe/Madrid", "Atlantic/Canary"],
                            primary: "Atlantic/Canary",
                            logs: "primary",
                        },
                    }),
                }),
                expect.anything()
            )
        )
    },
}
