// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {useGetOne} from "react-admin"
import {i18n, browserTimeZone} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {TallyStoryContext, tallySession} from "./__stories__/TallyFixture"
import {TallyStartDate} from "./TallyStartDate"
import {EStoryWorkflow} from "../../../../ui-essentials/.storybook/globals"
import {formatZoned} from "@/lib/timezones/zonedFormat"

/** A time as the screen shows it outside an event's screens: labelled, in the viewer's zone. */
const shownTime = (value: string, seconds = false) =>
    formatZoned(value, browserTimeZone(), {t: i18n.t.bind(i18n), lang: i18n.language, seconds})

type Scenario = Record<string, never>

let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

/**
 * The tally screen shows the date on its results page, after it has loaded
 * the tally session that the field reads again.
 */
function LoadedTally() {
    const {data: tally} = useGetOne("sequent_backend_tally_session", {id: STORY_IDS.tallySession})
    return tally ? <TallyStartDate /> : null
}

const meta = {
    title: "Admin/Tally/TallyStartDate",
    component: TallyStartDate,
    beforeEach: () => {
        boundary = graphqlBoundary({})
        data = resourceBoundary({
            sequent_backend_tally_session: [
                tallySession(EStoryWorkflow.RESULTS, {created_at: "2026-01-15T12:00:00Z"}),
            ],
        })
    },
    render: () => (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <TallyStoryContext>
                <LoadedTally />
            </TallyStoryContext>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const field = await within(canvasElement).findByRole("textbox", {
            name: i18n.t("tally.common.date"),
        })
        // When the tally started, with its zone label, read-only.
        expect(field).toHaveValue(shownTime("2026-01-15T12:00:00Z", false))
        expect(field).toBeDisabled()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", "sequent_backend_tally_session"],
        ])
    },
}
