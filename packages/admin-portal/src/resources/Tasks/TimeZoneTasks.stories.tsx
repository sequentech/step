// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// VOTE-LIFECYCLE drafts tz-tasks and tz-localization-admin: other Admin
// Portal times as one value with a label, in the event's primary zone, under
// two configurations; a tenant override of a zone label changes them.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n, zoneLabel, type IElectionEventPresentation} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {eventPresentation, eventRecord, storyId} from "@/__stories__/fixtures"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {applyTenantTranslationOverrides} from "@/providers/TenantContextProvider"
import {formatZoned} from "@/lib/timezones/zonedFormat"
import {ListTasks} from "./ListTasks"
import {taskRecord} from "./__stories__/TasksFixture"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

/** "My time" in the drafts: New York. Explicit, so no story depends on the machine's zone. */
const MY_ZONE = "America/New_York"

/** A Manila primary (the COMELEC preset) and a Madrid one (the Madrid association). */
const PRIMARIES = {manila: "Asia/Manila", madrid: "Europe/Madrid"} as const
type Configuration = keyof typeof PRIMARIES

interface Scenario {
    configuration: Configuration
    /** A tenant override of the Manila label (tz-localization-admin). */
    manilaLabel?: string
}

const presentation = (configuration: Configuration): IElectionEventPresentation => ({
    ...eventPresentation,
    timezones: {
        configured: [PRIMARIES[configuration], "Asia/Dubai"],
        primary: PRIMARIES[configuration],
    },
})

const STARTS = ["2028-04-08T19:41:15Z", "2028-04-07T00:02:17Z"]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({configuration}: Scenario) {
    const {permissions} = useStoryGlobals()
    const record = eventRecord(undefined, {
        presentation: presentation(configuration),
    }) as Sequent_Backend_Election_Event
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={permissions}>
            <MyTimeZoneProvider zone={MY_ZONE}>
                <EventTimeZoneProvider event={record}>
                    <ListTasks onViewTask={() => {}} electionEventRecord={record} />
                </EventTimeZoneProvider>
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Screens/Admin/VOTE-LIFECYCLE/Admin times",
    component: ListTasks,
    args: {configuration: "manila"},
    argTypes: {configuration: {control: "inline-radio", options: Object.keys(PRIMARIES)}},
    parameters: {
        expectedFailure: {
            reason: "White status chip labels lack contrast; the row's view action is an icon button without an accessible name.",
            a11y: ["button-name", "color-contrast"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            sequent_backend_tasks_execution: [
                taskRecord({name: "Initialization report: Dubai PCG", start_at: STARTS[0]}),
                taskRecord({id: storyId(5, 9), name: "Export logs (CSV)", start_at: STARTS[1]}),
            ],
        })
        graphql = graphqlBoundary({})
        if (args.manilaLabel) {
            applyTenantTranslationOverrides({
                i18n: {en: {"global:timezones.abbr.Asia/Manila": args.manilaLabel}},
            })
            return () => applyTenantTranslationOverrides({})
        }
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const options = () => ({t: i18n.t.bind(i18n), lang: i18n.language, seconds: true})

async function expectStarts(canvasElement: HTMLElement, configuration: Configuration) {
    const canvas = within(canvasElement)
    for (const start of STARTS) {
        const text = formatZoned(start, PRIMARIES[configuration], options())
        await expect(await canvas.findByText(text)).toBeVisible()
        expect(text).toContain(zoneLabel(PRIMARIES[configuration], options(), new Date(start)))
    }
    // One value: no second line in my time.
    expect(canvas.queryByText(/my time/)).toBeNull()
}

/** tz-tasks: start times in the event's primary zone, with its label. */
export const TzTasks: Story = {
    play: async ({canvasElement}) => expectStarts(canvasElement, "manila"),
}

export const TzTasksMadrid: Story = {
    args: {configuration: "madrid"},
    play: async ({canvasElement}) => expectStarts(canvasElement, "madrid"),
}

/** tz-localization-admin: the tenant sets the Manila label to PHT and the admin times follow. */
export const TzLocalizationAdmin: Story = {
    args: {manilaLabel: "PHT"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expectStarts(canvasElement, "manila")
        expect(canvas.getAllByText(/ PHT$/)).toHaveLength(STARTS.length)
        expect(canvas.queryByText(/PhST/)).toBeNull()
    },
}
