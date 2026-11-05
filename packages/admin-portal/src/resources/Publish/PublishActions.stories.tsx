// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {
    EBallotBoxSealPolicy,
    EGracePeriodPolicy,
    EVotingStatus,
    type IElectionPresentation,
    type IElectionStatus,
} from "@sequentech/ui-core"
import {EventTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {PublishActions} from "./PublishActions"
import {PublishStatus} from "./EPublishStatus"
import {EPublishActionsType, EPublishType} from "./EPublishType"
import {AuthContext} from "@/providers/AuthContextProvider"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {STORY_IDS, storyId} from "@/__stories__/fixtures"

const roles = [
    "publish-write",
    "election-state-write",
    "publish-regenerate",
    "publish-start-voting",
    "publish-pause-voting",
    "publish-stop-voting",
    "publish-changes",
]
const disabledChannel = {is_channel_enabled: false, status: EVotingStatus.NOT_STARTED}
const pendingKeys = [
    "pendingStartVotingPeriod",
    "pendingPauseVotingPeriod",
    "pendingStopVotingPeriod",
    "pendingPublishAction",
]
let boundary: ReturnType<typeof graphqlBoundary>
type Props = React.ComponentProps<typeof PublishActions> & {
    roles: string[]
    gold: boolean
    reauthenticate: (url: string) => Promise<void>
    requireInitializationReport: boolean
    /** The event seals its ballot boxes at close (VOTE-FREEZE). */
    sealAtClose: boolean
    /** The election's name, on an election's Publish tab. */
    electionName?: string
    /** The event's elections, which an event-wide Start or Stop reads. */
    eventElections?: Array<Record<string, unknown>>
    /** The election has ballot box seals (it never opens again). */
    sealed?: boolean
}

/** An election of the event, with its channel statuses and its grace period. */
const eventElection = (
    index: number,
    name: string,
    status: Record<string, string>,
    channels: Record<string, boolean>,
    graceSecs = 0
) => ({
    id: storyId(3, index),
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    presentation: {
        i18n: {en: {name, alias: name}},
        ...(graceSecs
            ? {
                  grace_period_policy: EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT,
                  grace_period_secs: graceSecs,
              }
            : {}),
    },
    // A channel that has a status other than NOT_STARTED has run: it has a start date.
    status: {
        ...status,
        ...Object.fromEntries(
            Object.entries(status)
                .filter(([, value]) => value !== "NOT_STARTED")
                .map(([key]) => [
                    key.replace("_status", "_period_dates"),
                    {first_started_at: "2028-05-01T08:00:00Z"},
                ])
        ),
    },
    voting_channels: channels,
})

function PublicationFixture(props: Props) {
    const auth = useContext(AuthContext)
    return (
        <AdminStoryProvider
            boundary={boundary}
            dataProvider={
                resourceBoundary({sequent_backend_election: (props.eventElections ?? []) as never})
                    .provider
            }
        >
            <AuthContext.Provider
                value={{
                    ...auth,
                    isAuthenticated: true,
                    tenantId: TENANT_ID,
                    isGoldUser: () => props.gold,
                    reauthWithGold: props.reauthenticate,
                    hasRole: (role) => props.roles.includes(role),
                    isAuthorized: (_superAdmin, tenant, permission) =>
                        tenant === TENANT_ID &&
                        (Array.isArray(permission) ? permission : [permission]).every((role) =>
                            props.roles.includes(role)
                        ),
                }}
            >
                <RecordContextProvider
                    value={{
                        id:
                            props.publishType === EPublishType.Election
                                ? STORY_IDS.election
                                : EVENT_ID,
                        tenant_id: TENANT_ID,
                        election_event_id: EVENT_ID,
                        presentation: {
                            initialization_report_policy: props.requireInitializationReport
                                ? "required"
                                : "not-required",
                            ...(props.electionName
                                ? {
                                      i18n: {
                                          en: {name: props.electionName, alias: props.electionName},
                                      },
                                  }
                                : {}),
                        },
                        initialization_report_generated: false,
                    }}
                >
                    <EventTimeZoneProvider
                        event={{
                            id: EVENT_ID,
                            presentation: {
                                ballot_box_seal_policy: props.sealAtClose
                                    ? EBallotBoxSealPolicy.SEAL_AT_CLOSE
                                    : EBallotBoxSealPolicy.DO_NOT_SEAL,
                            },
                        }}
                    >
                        <PublishActions {...props} />
                    </EventTimeZoneProvider>
                </RecordContextProvider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/Publish/PublishActions",
    component: PublishActions,
    args: {
        roles,
        gold: true,
        reauthenticate: fn(async () => {}),
        requireInitializationReport: false,
        sealAtClose: false,
        status: PublishStatus.Published,
        publishType: EPublishType.Event,
        type: EPublishActionsType.List,
        electionStatus: null,
        electionPresentation: null,
        changingStatus: false,
        kioskModeEnabled: disabledChannel,
        earlyVotingEnabled: disabledChannel,
        telephoneVotingEnabled: disabledChannel,
        onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.NOT_STARTED},
        onGenerate: fn(),
        onPublish: fn(),
        onChangeStatus: fn(),
    },
    beforeEach: ({args}) => {
        boundary = graphqlBoundary({
            // Every election asked about has a seal when the story is sealed.
            GetBallotBoxSeals: ({variables}) => ({
                data: {
                    sequent_backend_ballot_box_seal: args.sealed
                        ? (variables.electionIds as string[]).map((electionId, index) => ({
                              id: storyId(0, index + 1),
                              election_id: electionId,
                              area_id: STORY_IDS.area,
                          }))
                        : [],
                },
            }),
        })
        for (const key of pendingKeys) {
            sessionStorage.removeItem(key)
            sessionStorage.removeItem(`${key}_CHANNELS`)
        }
        const storyBoundary = boundary
        return () => expect(storyBoundary.unexpected).toEqual([])
    },
    render: (args) => <PublicationFixture {...args} />,
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

async function openChannelAction(
    canvasElement: HTMLElement,
    action: string,
    channelAction: string
) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: action}))
    await userEvent.click(await within(document.body).findByRole("menuitem", {name: channelAction}))
    const dialogElement = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialogElement).toBeVisible())
    return within(dialogElement)
}
async function confirm(dialog: ReturnType<typeof within>) {
    await userEvent.click(dialog.getByRole("button", {name: "Confirm"}))
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument())
    // Nothing but the seal reads (VOTE-FREEZE) is asked: the change is the caller's.
    expect(boundary.calls.filter(({name}) => name !== "GetBallotBoxSeals")).toEqual([])
    expect(boundary.unexpected).toEqual([])
}

export const StartOnlineVoting: Story = {
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Online Voting")
        expect(args.onChangeStatus).not.toHaveBeenCalled()
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("OPEN", ["ONLINE"])
        expect(args.reauthenticate).not.toHaveBeenCalled()
    },
}
export const PauseOnlineVoting: Story = {
    args: {onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.OPEN}},
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Pause Voting", "Pause Online Voting")
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("PAUSED", ["ONLINE"])
    },
}
export const StopOnlineVoting: Story = {
    args: {onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.OPEN}},
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Stop Voting", "Stop Online Voting")
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("CLOSED", ["ONLINE"])
    },
}
const sealedOnline = {is_channel_enabled: true, status: EVotingStatus.OPEN}
/** The event's elections load from the data provider before an event-wide action reads them. */
const electionsLoaded = () => new Promise((resolve) => setTimeout(resolve, 500))
const stopWithText = async (canvasElement: HTMLElement, text: string) => {
    await electionsLoaded()
    const dialog = await openChannelAction(canvasElement, "Stop Voting", "Stop Online Voting")
    await expect(dialog.getByText(text)).toBeVisible()
    await confirm(dialog)
}

/** VOTE-FREEZE: stopping the whole event says its ballot boxes are then sealed. */
export const StopVotingSealsTheEvent: Story = {
    args: {sealAtClose: true, onlineModeEnabled: sealedOnline},
    play: async ({canvasElement, args}) => {
        await stopWithText(
            canvasElement,
            "You are about to stop voting in every election. Their ballot boxes are then sealed: no ballot can be added, changed or deleted, and voting cannot start again. Are you sure you want to continue?"
        )
        expect(args.onChangeStatus).toHaveBeenCalledWith("CLOSED", ["ONLINE"])
    },
}

/** VOTE-FREEZE: stopping one channel while another is open seals nothing yet, and says when it will. */
export const StopOneOfTwoOpenChannels: Story = {
    args: {
        sealAtClose: true,
        publishType: EPublishType.Election,
        electionName: "Madrid Post",
        onlineModeEnabled: sealedOnline,
        kioskModeEnabled: sealedOnline,
    },
    play: async ({canvasElement}) =>
        stopWithText(
            canvasElement,
            "You are about to stop voting period. With Seal at close, its ballot boxes are sealed once every enabled channel is closed: Kiosk is still enabled and not closed. Are you sure you want to continue?"
        ),
}

/** W6: enabled early voting that never opened holds the seal like any channel, until it is stopped. */
export const StopWaitsForNeverOpenedEarlyVoting: Story = {
    args: {
        sealAtClose: true,
        publishType: EPublishType.Election,
        electionName: "Madrid Post",
        onlineModeEnabled: sealedOnline,
        earlyVotingEnabled: {is_channel_enabled: true, status: EVotingStatus.NOT_STARTED},
    },
    play: async ({canvasElement}) =>
        stopWithText(
            canvasElement,
            "You are about to stop voting period. With Seal at close, its ballot boxes are sealed once every enabled channel is closed: Early voting is still enabled and not closed. Are you sure you want to continue?"
        ),
}

/** W6: a channel the Post no longer enables but that is open holds the seal; the dialog says what to do. */
export const StopWithAnOpenChannelThatIsNotEnabled: Story = {
    args: {
        sealAtClose: true,
        publishType: EPublishType.Election,
        electionName: "Madrid Post",
        onlineModeEnabled: sealedOnline,
        kioskModeEnabled: {is_channel_enabled: false, status: EVotingStatus.OPEN},
    },
    play: async ({canvasElement}) =>
        stopWithText(
            canvasElement,
            "You are about to stop voting period. Kiosk isn't closed and isn't enabled for this Post: stop it to seal the ballot boxes. Are you sure you want to continue?"
        ),
}

/** R10 B1: the Post's Stop offers a channel it doesn't enable that isn't closed; stopping it seals. */
export const StopAChannelThePostDoesNotEnable: Story = {
    args: {
        sealAtClose: true,
        publishType: EPublishType.Election,
        electionName: "Madrid Post",
        onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.CLOSED},
        kioskModeEnabled: {is_channel_enabled: false, status: EVotingStatus.OPEN},
    },
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Stop Voting", "Stop Kiosk Voting")
        await expect(
            dialog.getByText(
                "You are about to stop voting in Madrid Post. Its ballot boxes are then sealed: no ballot can be added, changed or deleted, and voting cannot start again. Are you sure you want to continue?"
            )
        ).toBeVisible()
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("CLOSED", ["KIOSK"])
    },
}

/** D1: a kiosk that never opened holds the seal; Stop closes it, and the dialog says it won't open. */
export const StopANeverOpenedKiosk: Story = {
    args: {
        sealAtClose: true,
        publishType: EPublishType.Election,
        electionName: "Madrid Post",
        onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.CLOSED},
        kioskModeEnabled: {is_channel_enabled: true, status: EVotingStatus.NOT_STARTED},
    },
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Stop Voting", "Stop Kiosk Voting")
        await expect(
            dialog.getByText(
                "Kiosk never opened: stopping it means it won't open. You are about to stop voting in Madrid Post. Its ballot boxes are then sealed: no ballot can be added, changed or deleted, and voting cannot start again. Are you sure you want to continue?"
            )
        ).toBeVisible()
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("CLOSED", ["KIOSK"])
    },
}

/** S4: an event-wide Stop says when each election is sealed, with their grace periods. */
export const StopTheEventWithGracePeriods: Story = {
    args: {
        sealAtClose: true,
        onlineModeEnabled: sealedOnline,
        eventElections: [
            eventElection(1, "Council 2028", {voting_status: "OPEN"}, {online: true}, 900),
            eventElection(2, "Board 2028", {voting_status: "OPEN"}, {online: true}),
        ],
    },
    play: async ({canvasElement}) =>
        stopWithText(
            canvasElement,
            "You are about to stop voting in every election. Their ballot boxes are sealed when each election's grace period ends, up to 15 minutes later: from then on no ballot can be added, changed or deleted. Voting cannot start again. Are you sure you want to continue?"
        ),
}

/** S4: an event-wide Stop of one channel seals the elections it finishes and names the others. */
export const StopTheEventWhileSomeKeepAChannel: Story = {
    args: {
        sealAtClose: true,
        onlineModeEnabled: sealedOnline,
        kioskModeEnabled: sealedOnline,
        eventElections: [
            eventElection(1, "Madrid Post", {voting_status: "OPEN"}, {online: true}),
            eventElection(
                2,
                "Rome Post",
                {voting_status: "OPEN", kiosk_voting_status: "OPEN"},
                {online: true, kiosk: true}
            ),
        ],
    },
    play: async ({canvasElement}) =>
        stopWithText(
            canvasElement,
            "You are about to stop voting in every election. The ballot boxes of Madrid Post are then sealed: no ballot can be added, changed or deleted, and voting cannot start again there. Rome Post keeps another channel enabled and not closed: its ballot boxes are sealed once that channel is closed. Are you sure you want to continue?"
        ),
}

/** S3: an election with seals never opens again; Start isn't offered, and the page says why. */
export const StartIsNotOfferedForASealedElection: Story = {
    args: {
        sealAtClose: true,
        sealed: true,
        roles: [...roles, "tally-read"],
        publishType: EPublishType.Election,
        electionName: "Madrid Post",
        onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.CLOSED},
        kioskModeEnabled: {is_channel_enabled: true, status: EVotingStatus.NOT_STARTED},
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "Start Voting isn't available: this election's ballot boxes are sealed or being sealed, and with Seal at close closed voting stays closed."
            )
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Start Voting"})).toBeDisabled()
    },
}

/** VOTE-FREEZE: an event-wide Start names the closed elections that stay closed. */
export const StartTheEventLeavesClosedElectionsClosed: Story = {
    args: {
        sealAtClose: true,
        roles: [...roles, "tally-read"],
        eventElections: [
            eventElection(1, "Madrid Post", {voting_status: "CLOSED"}, {online: true}),
            eventElection(2, "Rome Post", {voting_status: "PAUSED"}, {online: true}),
        ],
    },
    play: async ({canvasElement, args}) => {
        await electionsLoaded()
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Online Voting")
        await expect(
            await dialog.findByText(
                "You are about to start voting period. Are you sure you want to continue? With Seal at close, closed voting stays closed: Madrid Post: Online stays closed."
            )
        ).toBeVisible()
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("OPEN", ["ONLINE"])
    },
}

/** S5: an online Start opens a Post whose kiosk closed, so the note doesn't name it. */
export const StartOnlineOpensAPostWithAClosedKiosk: Story = {
    args: {
        sealAtClose: true,
        roles: [...roles, "tally-read"],
        eventElections: [
            eventElection(
                1,
                "Madrid Post",
                {voting_status: "NOT_STARTED", kiosk_voting_status: "CLOSED"},
                {online: true, kiosk: true}
            ),
        ],
    },
    play: async ({canvasElement}) => {
        await electionsLoaded()
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Online Voting")
        await expect(
            await dialog.findByText(
                "You are about to start voting period. Are you sure you want to continue?"
            )
        ).toBeVisible()
        await confirm(dialog)
    },
}

/** S5: a Post with ballot box seals stays closed whatever the channel. */
export const StartTheEventKeepsASealedPostClosed: Story = {
    args: {
        sealAtClose: true,
        sealed: true,
        roles: [...roles, "tally-read"],
        eventElections: [
            eventElection(1, "Madrid Post", {voting_status: "PAUSED"}, {online: true}),
        ],
    },
    play: async ({canvasElement}) => {
        await electionsLoaded()
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Online Voting")
        await expect(
            await dialog.findByText(
                "You are about to start voting period. Are you sure you want to continue? With Seal at close, closed voting stays closed: Madrid Post stays closed, as its ballot boxes are sealed."
            )
        ).toBeVisible()
        await confirm(dialog)
    },
}

/** R8 N1: online voting enabled but never run: no grace period, the seal is immediate. */
export const StopWithoutOnlineRunningHasNoGrace: Story = {
    args: {
        sealAtClose: true,
        publishType: EPublishType.Election,
        electionName: "Council 2028",
        electionPresentation: {
            grace_period_policy: EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT,
            grace_period_secs: 900,
        } as IElectionPresentation,
        // Online was closed without ever starting; the kiosk ran.
        onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.CLOSED},
        kioskModeEnabled: sealedOnline,
    },
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Stop Voting", "Stop Kiosk Voting")
        await expect(
            dialog.getByText(
                "You are about to stop voting in Council 2028. Its ballot boxes are then sealed: no ballot can be added, changed or deleted, and voting cannot start again. Are you sure you want to continue?"
            )
        ).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        expect(args.onChangeStatus).not.toHaveBeenCalled()
    },
}

/** R8 N2: an event-wide Stop closes a Post that never opened and seals its empty boxes; it says so. */
export const StopTheEventClosesANeverOpenedPost: Story = {
    args: {
        sealAtClose: true,
        onlineModeEnabled: sealedOnline,
        eventElections: [
            eventElection(1, "Madrid Post", {voting_status: "OPEN"}, {online: true}),
            eventElection(2, "Valencia Post", {voting_status: "NOT_STARTED"}, {online: true}),
        ],
    },
    play: async ({canvasElement}) =>
        stopWithText(
            canvasElement,
            "Valencia Post never opened: stopping closes it and seals its empty ballot boxes. You are about to stop voting in every election. Their ballot boxes are then sealed: no ballot can be added, changed or deleted, and voting cannot start again. Are you sure you want to continue?"
        ),
}

/** R8 S4: an event Start names, per Post, the channels that stay closed. */
export const StartKioskNamesOnlyTheClosedChannel: Story = {
    args: {
        sealAtClose: true,
        roles: [...roles, "tally-read"],
        onlineModeEnabled: sealedOnline,
        kioskModeEnabled: {is_channel_enabled: true, status: EVotingStatus.NOT_STARTED},
        eventElections: [
            eventElection(
                1,
                "Madrid Post",
                {voting_status: "CLOSED", kiosk_voting_status: "NOT_STARTED"},
                {online: true, kiosk: true}
            ),
            eventElection(
                2,
                "Rome Post",
                {voting_status: "OPEN", kiosk_voting_status: "CLOSED"},
                {online: true, kiosk: true}
            ),
        ],
    },
    play: async ({canvasElement, args}) => {
        await electionsLoaded()
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Kiosk Voting")
        await expect(
            await dialog.findByText(
                "You are about to start voting period. Are you sure you want to continue? With Seal at close, closed voting stays closed: Rome Post: Kiosk stays closed."
            )
        ).toBeVisible()
        await confirm(dialog)
        expect(args.onChangeStatus).toHaveBeenCalledWith("OPEN", ["KIOSK"])
    },
}

/** VOTE-FREEZE: an election without a grace period is sealed at the close. */
export const StopVotingSealsTheElection: Story = {
    args: {
        sealAtClose: true,
        publishType: EPublishType.Election,
        electionName: "Madrid Post",
        onlineModeEnabled: sealedOnline,
    },
    play: async ({canvasElement}) =>
        stopWithText(
            canvasElement,
            "You are about to stop voting in Madrid Post. Its ballot boxes are then sealed: no ballot can be added, changed or deleted, and voting cannot start again. Are you sure you want to continue?"
        ),
}

/** Second organization 1: a 15-minute grace period delays the seal. */
export const StopVotingSealsAfterTheGracePeriod: Story = {
    args: {
        sealAtClose: true,
        publishType: EPublishType.Election,
        electionName: "Council 2028",
        electionPresentation: {
            grace_period_policy: EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT,
            grace_period_secs: 900,
        } as IElectionPresentation,
        // Online voting ran: the grace period applies (deadline.rs online_ran).
        electionStatus: {
            voting_status: EVotingStatus.OPEN,
            voting_period_dates: {first_started_at: "2028-03-13T08:00:00Z"},
        } as unknown as IElectionStatus,
        onlineModeEnabled: sealedOnline,
    },
    play: async ({canvasElement}) =>
        stopWithText(
            canvasElement,
            "You are about to stop voting in Council 2028. Its ballot boxes are sealed when the grace period ends, 15 minutes later: from then on no ballot can be added, changed or deleted. Voting cannot start again. Are you sure you want to continue?"
        ),
}

export const CancelSensitiveAction: Story = {
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Online Voting")
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        expect(args.onChangeStatus).not.toHaveBeenCalled()
        expect(args.reauthenticate).not.toHaveBeenCalled()
    },
}
export const ActionsHiddenWithoutPermissions: Story = {
    args: {roles: []},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        for (const name of ["Start Voting", "Pause Voting", "Stop Voting", "Publish Changes"]) {
            expect(canvas.queryByRole("button", {name})).not.toBeInTheDocument()
        }
        expect(args.onChangeStatus).not.toHaveBeenCalled()
        expect(boundary.calls).toEqual([])
    },
}
export const StartPermissionDoesNotGrantOtherActions: Story = {
    args: {roles: ["election-state-write", "publish-start-voting"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: "Start Voting"})).toBeEnabled()
        for (const name of ["Pause Voting", "Stop Voting", "Publish Changes"]) {
            expect(canvas.queryByRole("button", {name})).not.toBeInTheDocument()
        }
    },
}
export const ClosedChannelsCannotRestart: Story = {
    args: {onlineModeEnabled: {is_channel_enabled: true, status: EVotingStatus.CLOSED}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        for (const name of ["Start Voting", "Pause Voting", "Stop Voting"]) {
            await expect(canvas.getByRole("button", {name})).toBeDisabled()
        }
    },
}
export const GoldReauthenticationPreservesChannel: Story = {
    args: {gold: false},
    play: async ({canvasElement, args}) => {
        const dialog = await openChannelAction(canvasElement, "Start Voting", "Start Online Voting")
        await confirm(dialog)
        expect(args.onChangeStatus).not.toHaveBeenCalled()
        expect(args.reauthenticate).toHaveBeenCalledTimes(1)
        expect(args.reauthenticate).toHaveBeenCalledWith(expect.stringContaining("tabId=publish"))
        expect(sessionStorage.getItem("pendingStartVotingPeriod")).toBe("true")
        expect(sessionStorage.getItem("pendingStartVotingPeriod_CHANNELS")).toBe('["ONLINE"]')
    },
}
export const PublishChangesRequiresConfirmation: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Publish Changes"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        expect(args.onGenerate).not.toHaveBeenCalled()
        await confirm(dialog)
        expect(args.onGenerate).toHaveBeenCalledTimes(1)
    },
}
export const RegenerationRequiresPermission: Story = {
    args: {type: EPublishActionsType.Generate, roles: ["publish-write"]},
    play: async ({canvasElement, args}) => {
        expect(
            within(canvasElement).queryByRole("button", {name: "Regenerate"})
        ).not.toBeInTheDocument()
        expect(args.onGenerate).not.toHaveBeenCalled()
    },
}
export const RegeneratePublication: Story = {
    args: {type: EPublishActionsType.Generate},
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Regenerate"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await confirm(dialog)
        expect(args.onGenerate).toHaveBeenCalledTimes(1)
    },
}
export const RequiredInitializationReportBlocksOnlineStart: Story = {
    args: {
        requireInitializationReport: true,
        kioskModeEnabled: {is_channel_enabled: true, status: EVotingStatus.NOT_STARTED},
    },
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Start Voting"}))
        const menu = within(await within(document.body).findByRole("menu"))
        await expect(menu.getByRole("menuitem", {name: "Start Online Voting"})).toHaveAttribute(
            "aria-disabled",
            "true"
        )
        expect(menu.getByRole("menuitem", {name: "Start Kiosk Voting"})).not.toHaveAttribute(
            "aria-disabled",
            "true"
        )
        await userEvent.keyboard("{Escape}")
    },
}
