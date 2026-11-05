// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// VOTE-FREEZE: the Ballot boxes card under a Post's monitoring dashboards, with
// the switch that hides it, remembered per viewer in the browser.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EBallotBoxSealPolicy} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {
    BALLOT_BOXES_VISIBLE_KEY,
    BallotBoxesSection,
    EBallotBoxesVisibility,
} from "./BallotBoxesSection"
import {
    BALLOT_BOXES_FIXTURES,
    EBallotBoxesScenario,
    sealingEventPresentation,
} from "./__stories__/BallotBoxesCard.fixtures"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

interface Scenario {
    policy: EBallotBoxSealPolicy
    /** What this browser remembers from an earlier visit; nothing by default. */
    stored?: EBallotBoxesVisibility
}

const FIXTURE = BALLOT_BOXES_FIXTURES[EBallotBoxesScenario.SEALED]

let graphql: ReturnType<typeof graphqlBoundary>

function Fixture({policy}: Scenario) {
    const {permissions} = useStoryGlobals()
    const event = {id: EVENT_ID, presentation: sealingEventPresentation(FIXTURE.zone, policy)}
    return (
        <AdminStoryProvider boundary={graphql} role={permissions}>
            <MyTimeZoneProvider zone={FIXTURE.zone}>
                <EventTimeZoneProvider event={event}>
                    <BallotBoxesSection
                        electionEventId={EVENT_ID}
                        electionId={STORY_IDS.election}
                        election={FIXTURE.election}
                        now={new Date(FIXTURE.now)}
                    />
                </EventTimeZoneProvider>
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Dashboard/Election/BallotBoxesSection",
    component: BallotBoxesSection,
    args: {policy: EBallotBoxSealPolicy.SEAL_AT_CLOSE},
    argTypes: {
        policy: {control: "inline-radio", options: Object.values(EBallotBoxSealPolicy)},
        stored: {control: "inline-radio", options: Object.values(EBallotBoxesVisibility)},
    },
    beforeEach: async ({args}) => {
        try {
            window.localStorage.removeItem(BALLOT_BOXES_VISIBLE_KEY)
            if (args.stored) window.localStorage.setItem(BALLOT_BOXES_VISIBLE_KEY, args.stored)
        } catch {
            // A blocked storage shows the ballot boxes, as in the portal.
        }
        graphql = graphqlBoundary(
            {
                GetBallotBoxSeals: () => ({
                    data: {sequent_backend_ballot_box_seal: FIXTURE.seals},
                }),
                GetBallotBoxAreas: () => ({
                    data: {
                        sequent_backend_ballot_style: FIXTURE.areas.map((area) => ({
                            area_id: area.id,
                        })),
                    },
                }),
                GetBallotBoxAreaNames: () => ({data: {sequent_backend_area: FIXTURE.areas}}),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const toggle = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("switch", {name: "Show ballot boxes"})

/** Shown by default, and hidden by switching it off. */
export const ShownByDefault: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const shown = await toggle(canvasElement)
        await expect(shown).toBeChecked()
        await expect(await canvas.findByRole("cell", {name: "Spain"})).toBeVisible()
        await userEvent.click(shown)
        await expect(shown).not.toBeChecked()
        await waitFor(() => expect(canvas.queryByRole("cell", {name: "Spain"})).toBeNull())
        expect(window.localStorage.getItem(BALLOT_BOXES_VISIBLE_KEY)).toBe(
            EBallotBoxesVisibility.HIDDEN
        )
    },
}

/** A viewer who hid the ballot boxes on an earlier visit finds them hidden. */
export const HiddenOnAnEarlierVisit: Story = {
    args: {stored: EBallotBoxesVisibility.HIDDEN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await toggle(canvasElement)).not.toBeChecked()
        expect(canvas.queryByRole("cell", {name: "Spain"})).toBeNull()
        await userEvent.click(await toggle(canvasElement))
        await expect(await canvas.findByRole("cell", {name: "Spain"})).toBeVisible()
    },
}

/** An event that doesn't seal its ballot boxes shows neither the switch nor the card. */
export const NotSealing: Story = {
    args: {policy: EBallotBoxSealPolicy.DO_NOT_SEAL},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(canvas.queryByRole("switch")).toBeNull()
        expect(canvas.queryByText("Ballot boxes")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}
