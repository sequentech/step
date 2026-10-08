// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// VOTE-FREEZE drafts: the Ballot boxes card on a Post's Dashboard, from the
// open ballot boxes to the sealed ones, for a Post closed by two signers and
// for a staff association closing with a 15-minute grace period.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EBallotBoxSealPolicy, i18n, zoneLabel} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EventTimeZoneProvider, MyTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {BallotBoxesCard} from "./BallotBoxesCard"
import {
    ASSOCIATION_CLOSE,
    BALLOT_BOXES_FIXTURES,
    EBallotBoxesScenario,
    MADRID,
    RESTRICTED_RECORD_DOCUMENTS,
    sealingEventPresentation,
} from "./__stories__/BallotBoxesCard.fixtures"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

interface Scenario {
    scenario: EBallotBoxesScenario
    policy: EBallotBoxSealPolicy
    /** The seals can't be read (e.g. a missing role). */
    unavailable?: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>

function Fixture({scenario, policy}: Pick<Scenario, "scenario" | "policy">) {
    const {permissions} = useStoryGlobals()
    const fixture = BALLOT_BOXES_FIXTURES[scenario]
    const event = {id: EVENT_ID, presentation: sealingEventPresentation(fixture.zone, policy)}
    return (
        <AdminStoryProvider boundary={graphql} role={permissions}>
            <MyTimeZoneProvider zone={fixture.zone}>
                <EventTimeZoneProvider event={event}>
                    <BallotBoxesCard
                        electionEventId={EVENT_ID}
                        electionId={STORY_IDS.election}
                        election={fixture.election}
                        now={new Date(fixture.now)}
                    />
                </EventTimeZoneProvider>
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Dashboard/Election/BallotBoxesCard",
    component: BallotBoxesCard,
    args: {scenario: EBallotBoxesScenario.SEALED, policy: EBallotBoxSealPolicy.SEAL_AT_CLOSE},
    argTypes: {
        scenario: {control: "select", options: Object.values(EBallotBoxesScenario)},
        policy: {control: "inline-radio", options: Object.values(EBallotBoxSealPolicy)},
    },
    beforeEach: async ({args}) => {
        const fixture = BALLOT_BOXES_FIXTURES[args.scenario]
        graphql = graphqlBoundary(
            {
                GetBallotBoxSeals: () => {
                    if (args.unavailable) throw new Error("Synthetic seals unavailable")
                    return {data: {sequent_backend_ballot_box_seal: fixture.seals}}
                },
                GetBallotBoxAreas: () => ({
                    data: {
                        sequent_backend_ballot_style: fixture.areas.map((area) => ({
                            area_id: area.id,
                        })),
                    },
                }),
                GetBallotBoxAreaNames: () => ({data: {sequent_backend_area: fixture.areas}}),
                FetchDocument: () => ({
                    data: {fetchDocument: {url: "data:application/json,%7B%7D"}},
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/**
 * The Madrid zone's label as the timezone texts give it ("CET" in the drafts;
 * the browser's own name, e.g. "GMT+1", when no text overrides it).
 */
const madridLabel = () =>
    zoneLabel(MADRID, {t: i18n.t.bind(i18n), lang: i18n.language}, new Date(ASSOCIATION_CLOSE))

const rowOf = async (canvasElement: HTMLElement, area: string) =>
    (await within(canvasElement).findByRole("cell", {name: area})).closest("tr") as HTMLElement

/** Closing 1: both countries sealed after two signers closed the Post. */
export const Sealed: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "Voting closed at 7:01 PM PhST. The ballot boxes are sealed: no ballot can be added, changed or deleted."
            )
        ).toBeVisible()
        await expect(
            canvas.getByText(
                "Closed by Maria L. Santos and Jose R. Dela Cruz with their certificates, signing code 5D90-A3F7."
            )
        ).toBeVisible()
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(spain.getByText("Sealed")).toBeVisible()
        await expect(spain.getByText("1,342")).toBeVisible()
        await expect(spain.getByText("1,340")).toBeVisible()
        await expect(spain.getByText("7:01 PM PhST")).toBeVisible()
        await expect(spain.getByText("ef187f0b…a65e5b")).toBeVisible()
        await expect(spain.getByRole("link", {name: /seal record of Spain/})).toHaveAttribute(
            "href",
            `https://public.admin-story.invalid/ballot-box-seals/${STORY_IDS.election}/${STORY_IDS.area}.json`
        )
        const andorra = within(await rowOf(canvasElement, "Andorra"))
        await expect(andorra.getAllByText("16")).toHaveLength(2)
        // Why a box holds more ballots than it counts.
        await expect(
            canvas.getByText(
                "Ballots that count: each eligible voter's latest valid ballot. The others in the box were replaced by the voter's later ballot, discarded, or cast by a voter who is not eligible."
            )
        ).toBeVisible()
    },
}

/** Closing 2: the second seal is locked but its entry is still being posted. */
export const Publishing: Story = {
    args: {scenario: EBallotBoxesScenario.PUBLISHING},
    play: async ({canvasElement}) => {
        const andorra = within(await rowOf(canvasElement, "Andorra"))
        await expect(andorra.getByText("Sealed, publishing")).toBeVisible()
        await expect(andorra.getByText("Not yet")).toBeVisible()
        expect(andorra.queryByRole("link")).toBeNull()
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(spain.getByRole("link", {name: /seal record of Spain/})).toBeVisible()
    },
}

/**
 * W6: with the Seal Record Publication policy Restricted, a record is a private
 * document: the card downloads it through a presigned URL instead of a public link.
 */
export const RestrictedRecord: Story = {
    args: {scenario: EBallotBoxesScenario.RESTRICTED_RECORD},
    play: async ({canvasElement}) => {
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(spain.getByText("Sealed")).toBeVisible()
        expect(spain.queryByRole("link")).toBeNull()
        await userEvent.click(
            spain.getByRole("button", {name: "Download the seal record of Spain"})
        )
        await waitFor(() =>
            expect(graphql.calls.find(({name}) => name === "FetchDocument")?.variables).toEqual({
                electionEventId: EVENT_ID,
                documentId: RESTRICTED_RECORD_DOCUMENTS[0],
            })
        )
        expect(spain.queryByText("The seal record could not be downloaded. Try again.")).toBeNull()
    },
}

/** Second organization 2: both offices wait for the 15-minute grace period. */
export const SealingAfterTheGracePeriod: Story = {
    args: {scenario: EBallotBoxesScenario.GRACE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                `Voting closed at 6:00 PM ${madridLabel()}. The ballot boxes are sealed when the grace period ends, at 6:15 PM ${madridLabel()}.`
            )
        ).toBeVisible()
        await expect(canvas.getByText("Closed by rrhh.admin.")).toBeVisible()
        for (const office of ["Madrid office", "Canary Islands office"]) {
            const row = within(await rowOf(canvasElement, office))
            await expect(row.getByText(`Sealing at 6:15 PM ${madridLabel()}`)).toBeVisible()
            expect(row.queryByRole("link")).toBeNull()
        }
    },
}

/** Second organization 3: the same card after the grace period. */
export const SealedAfterTheGracePeriod: Story = {
    args: {scenario: EBallotBoxesScenario.GRACE_SEALED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                `Voting closed at 6:00 PM ${madridLabel()}. The ballot boxes are sealed: no ballot can be added, changed or deleted.`
            )
        ).toBeVisible()
        const madrid = within(await rowOf(canvasElement, "Madrid office"))
        await expect(madrid.getByText(`6:15 PM ${madridLabel()}`)).toBeVisible()
        await expect(madrid.getByText("212")).toBeVisible()
        await expect(madrid.getByText("208")).toBeVisible()
    },
}

/** No grace period: right after the close, the sealer is on it. */
export const SealingNow: Story = {
    args: {scenario: EBallotBoxesScenario.DUE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "Voting closed at 7:01 PM PhST. The ballot boxes are being sealed."
            )
        ).toBeVisible()
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(spain.getByText("Sealing now")).toBeVisible()
        await expect(spain.getByText("Being sealed: this takes up to a minute.")).toBeVisible()
    },
}

/** D5: the sealer recorded why the box waits; the card says it inline. */
export const OverdueKioskStillOpen: Story = {
    args: {scenario: EBallotBoxesScenario.OVERDUE_CHANNEL},
    play: async ({canvasElement}) => {
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(spain.getByText("Sealing overdue")).toBeVisible()
        await expect(
            spain.getByText(
                "Kiosk is still enabled and not closed: stop it to seal the ballot box."
            )
        ).toBeVisible()
    },
}

/** W6 / R10: a channel the Post doesn't enable isn't closed: the card says to stop it. */
export const OverdueChannelNotEnabled: Story = {
    args: {scenario: EBallotBoxesScenario.OVERDUE_NOT_ENABLED},
    play: async ({canvasElement}) => {
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(spain.getByText("Sealing overdue")).toBeVisible()
        await expect(
            spain.getByText(
                "Kiosk isn't closed and isn't enabled for this Post: stop it to seal the ballot box."
            )
        ).toBeVisible()
    },
}

/** W6: the box has ballots of a channel that isn't closed: the card names it. */
export const OverdueChannelHasBallots: Story = {
    args: {scenario: EBallotBoxesScenario.OVERDUE_BALLOTS},
    play: async ({canvasElement}) => {
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(spain.getByText("Sealing overdue")).toBeVisible()
        await expect(
            spain.getByText(
                "Telephone has ballots in this ballot box and isn't closed: stop it to seal the ballot box."
            )
        ).toBeVisible()
    },
}

export const OverdueDatafixVotes: Story = {
    args: {scenario: EBallotBoxesScenario.OVERDUE_DATAFIX},
    play: async ({canvasElement}) => {
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(
            spain.getByText(
                "3 votes are in progress in Datafix: the ballot box is sealed once they are resolved."
            )
        ).toBeVisible()
    },
}

export const OverdueAfterAnError: Story = {
    args: {scenario: EBallotBoxesScenario.OVERDUE_ERROR},
    play: async ({canvasElement}) => {
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(
            spain.getByText(
                "The last attempt couldn't get the signing key; it is retried every minute."
            )
        ).toBeVisible()
    },
}

/** No attempt since the deadline: the sealer may not be running. */
export const OverdueNotTried: Story = {
    args: {scenario: EBallotBoxesScenario.OVERDUE_STALE},
    play: async ({canvasElement}) => {
        const spain = within(await rowOf(canvasElement, "Spain"))
        await expect(spain.getByText("Sealing overdue")).toBeVisible()
        await expect(
            spain.getByText(
                "Not tried yet: the sealer may not be running. Check Beat and the seal worker."
            )
        ).toBeVisible()
    },
}

/** A seal that failed is an incident: the ballot box stays locked. */
export const Failed: Story = {
    args: {scenario: EBallotBoxesScenario.FAILED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "Voting closed at 7:01 PM PhST. A ballot box could not be sealed: it stays locked, and the incident is in the logs."
            )
        ).toBeVisible()
        const andorra = within(await rowOf(canvasElement, "Andorra"))
        await expect(andorra.getByText("Not sealed: incident")).toBeVisible()
        // The known reason, in the admin's language.
        await expect(andorra.getByText("A ballot does not match its Ballot ID.")).toBeVisible()
    },
}

/** Before the close each area of the election is listed as Open. */
export const Open: Story = {
    args: {scenario: EBallotBoxesScenario.OPEN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "Voting is open on Online. The ballot box of each area is sealed when voting closes."
            )
        ).toBeVisible()
        for (const area of ["Spain", "Andorra"]) {
            await expect(within(await rowOf(canvasElement, area)).getByText("Open")).toBeVisible()
        }
    },
}

export const NotOpenedYet: Story = {
    args: {scenario: EBallotBoxesScenario.NOT_STARTED},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                "Voting hasn't opened yet. The ballot box of each area is sealed when voting closes."
            )
        ).toBeVisible()
    },
}

/** D1: online closed, kiosk never opened: kiosk holds the seal, and the card says so. */
export const KioskHoldsTheSeal: Story = {
    args: {scenario: EBallotBoxesScenario.HOLDING},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                "Kiosk is enabled and not closed: stop it to seal the ballot boxes."
            )
        ).toBeVisible()
    },
}

/** An event that does not seal its ballot boxes shows no card and asks nothing. */
export const NotSealing: Story = {
    args: {policy: EBallotBoxSealPolicy.DO_NOT_SEAL},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByText("Ballot boxes")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

/** Seals that can't be read show as unknown, never as open. */
export const SealsUnavailable: Story = {
    args: {unavailable: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "The ballot boxes' seals could not be read. Reload the page, or check the connection to the server."
            )
        ).toBeVisible()
        expect(canvas.queryByRole("table")).toBeNull()
    },
}
