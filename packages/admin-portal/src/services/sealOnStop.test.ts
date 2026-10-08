// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    electionSealChannels,
    eventStartChannels,
    keptClosedChannels,
    neverOpened,
    onlineRan,
    sealProgress,
    stopSealOutcome,
    type ISealChannel,
} from "./sealOnStop"
import {EVotingStatus, VotingStatusChannel} from "@sequentech/ui-core"
jest.mock("@sequentech/ui-core", () => ({
    ...require("../../../ui-core/src/types/CoreTypes"),
    ...jest.requireActual("@sequentech/ui-core"),
}))

const {Online, Kiosk, EarlyVoting, Telephone} = VotingStatusChannel
const channel = (
    name: VotingStatusChannel,
    status: EVotingStatus,
    overrides: Partial<ISealChannel> = {}
): ISealChannel => ({channel: name, enabled: true, status, ...overrides})

describe("stopSealOutcome mirrors seal_deadline's channel rule (D1)", () => {
    it("seals when the stop closes the only open channel", () => {
        expect(stopSealOutcome([channel(Online, EVotingStatus.OPEN)], [Online])).toEqual({
            seals: true,
            holding: [],
            neverOpened: [],
        })
    })
    it("does not seal while another enabled channel is open or paused, and names it", () => {
        for (const status of [EVotingStatus.OPEN, EVotingStatus.PAUSED]) {
            expect(
                stopSealOutcome(
                    [channel(Online, EVotingStatus.OPEN), channel(Kiosk, status)],
                    [Online]
                )
            ).toEqual({seals: false, holding: [Kiosk], neverOpened: []})
        }
    })
    it("seals when stopping every channel at once", () => {
        expect(
            stopSealOutcome([
                channel(Online, EVotingStatus.OPEN),
                channel(Kiosk, EVotingStatus.PAUSED),
            ]).seals
        ).toBe(true)
    })
    it("a never-started kiosk or telephone channel holds the seal", () => {
        for (const held of [Kiosk, Telephone]) {
            expect(
                stopSealOutcome(
                    [channel(Online, EVotingStatus.OPEN), channel(held, EVotingStatus.NOT_STARTED)],
                    [Online]
                )
            ).toEqual({seals: false, holding: [held], neverOpened: []})
        }
    })
    it("never-started early voting doesn't hold the seal once online voting has started", () => {
        expect(
            stopSealOutcome(
                [
                    channel(Online, EVotingStatus.OPEN),
                    channel(EarlyVoting, EVotingStatus.NOT_STARTED),
                ],
                [Online]
            )
        ).toEqual({seals: true, holding: [], neverOpened: [EarlyVoting]})
    })
    it("closed early voting with online never started is not finished", () => {
        expect(
            stopSealOutcome(
                [
                    channel(Online, EVotingStatus.NOT_STARTED),
                    channel(EarlyVoting, EVotingStatus.OPEN),
                ],
                [EarlyVoting]
            )
        ).toEqual({seals: false, holding: [Online], neverOpened: []})
    })
    it("a NOT_STARTED channel that once opened holds the seal", () => {
        expect(
            stopSealOutcome(
                [
                    channel(Online, EVotingStatus.OPEN),
                    channel(EarlyVoting, EVotingStatus.NOT_STARTED, {
                        firstStartedAt: "2028-03-13T08:00:00Z",
                    }),
                ],
                [Online]
            ).holding
        ).toEqual([EarlyVoting])
    })
    it("ignores disabled channels and needs one closed channel", () => {
        expect(
            stopSealOutcome(
                [
                    channel(Online, EVotingStatus.OPEN),
                    channel(Kiosk, EVotingStatus.OPEN, {enabled: false}),
                ],
                [Online]
            ).seals
        ).toBe(true)
        expect(sealProgress([channel(Kiosk, EVotingStatus.NOT_STARTED)]).finished).toBe(false)
    })
})

describe("online voting started on a Post that doesn't enable it (deadline.rs online_started)", () => {
    it("still makes never-started early voting finished, as an event-wide Start sets it", () => {
        expect(
            sealProgress([
                channel(Online, EVotingStatus.OPEN, {enabled: false}),
                channel(EarlyVoting, EVotingStatus.NOT_STARTED),
                channel(Kiosk, EVotingStatus.CLOSED),
            ])
        ).toEqual({finished: true, holding: [], neverOpened: [EarlyVoting]})
    })
    it("counts an online channel that once opened even if it isn't enabled", () => {
        expect(
            sealProgress([
                channel(Online, EVotingStatus.NOT_STARTED, {
                    enabled: false,
                    firstStartedAt: "2028-01-01T00:00:00Z",
                }),
                channel(EarlyVoting, EVotingStatus.NOT_STARTED),
                channel(Kiosk, EVotingStatus.CLOSED),
            ]).finished
        ).toBe(true)
    })
    it("holds the seal while online voting has never started anywhere", () => {
        expect(
            sealProgress([
                channel(Online, EVotingStatus.NOT_STARTED, {enabled: false}),
                channel(EarlyVoting, EVotingStatus.NOT_STARTED),
                channel(Kiosk, EVotingStatus.CLOSED),
            ])
        ).toEqual({finished: false, holding: [EarlyVoting], neverOpened: []})
    })
})

describe("electionSealChannels", () => {
    it("reads each channel's status, dates and whether it is enabled", () => {
        const channels = electionSealChannels({
            status: {
                voting_status: "CLOSED",
                kiosk_voting_status: "NOT_STARTED",
                voting_period_dates: {first_started_at: "2028-01-01T00:00:00Z"},
            },
            voting_channels: {online: true, kiosk: true, early_voting: false},
        })
        expect(channels.find(({channel: name}) => name === Online)).toEqual({
            channel: Online,
            enabled: true,
            status: "CLOSED",
            firstStartedAt: "2028-01-01T00:00:00Z",
        })
        expect(channels.find(({channel: name}) => name === Kiosk)?.enabled).toBe(true)
        expect(channels.find(({channel: name}) => name === EarlyVoting)?.enabled).toBe(false)
        expect(sealProgress(channels)).toEqual({finished: false, holding: [Kiosk], neverOpened: []})
    })
})

describe("an event-wide Start, per channel (election_event_status.rs)", () => {
    const event = [
        channel(Online, EVotingStatus.OPEN),
        channel(Kiosk, EVotingStatus.NOT_STARTED),
        channel(EarlyVoting, EVotingStatus.NOT_STARTED, {enabled: false}),
    ]
    it("applies the named channels the event hasn't open, else its enabled ones", () => {
        expect(eventStartChannels(event, [Online, Kiosk])).toEqual([Kiosk])
        expect(eventStartChannels(event)).toEqual([Kiosk])
        expect(eventStartChannels(event, [EarlyVoting])).toEqual([EarlyVoting])
    })
    it("keeps closed only the applied channels that are CLOSED on the Post (R8 S4)", () => {
        // ONLINE CLOSED, KIOSK never started: a Start of ONLINE and KIOSK opens KIOSK there.
        const post = [
            channel(Online, EVotingStatus.CLOSED),
            channel(Kiosk, EVotingStatus.NOT_STARTED),
        ]
        expect(keptClosedChannels(post, eventStartChannels(event, [Online, Kiosk]))).toEqual([])
        expect(keptClosedChannels(post, [Online, Kiosk])).toEqual([Online])
    })
    it("reads the channel's status whether or not the Post enables it", () => {
        expect(
            keptClosedChannels([channel(Online, EVotingStatus.CLOSED, {enabled: false})], [Online])
        ).toEqual([Online])
    })
})

describe("onlineRan and neverOpened (deadline.rs online_ran)", () => {
    it("needs ONLINE enabled and started for the grace period", () => {
        expect(
            onlineRan([
                channel(Online, EVotingStatus.OPEN, {firstStartedAt: "2028-01-01T00:00:00Z"}),
            ])
        ).toBe(true)
        expect(onlineRan([channel(Online, EVotingStatus.CLOSED)])).toBe(false)
        expect(
            onlineRan([
                channel(Online, EVotingStatus.OPEN, {
                    enabled: false,
                    firstStartedAt: "2028-01-01T00:00:00Z",
                }),
            ])
        ).toBe(false)
    })
    it("knows a Post where no enabled channel ever opened", () => {
        expect(
            neverOpened([
                channel(Online, EVotingStatus.NOT_STARTED),
                channel(Kiosk, EVotingStatus.OPEN, {enabled: false}),
            ])
        ).toBe(true)
        expect(
            neverOpened([
                channel(Online, EVotingStatus.PAUSED, {firstStartedAt: "2028-01-01T00:00:00Z"}),
            ])
        ).toBe(false)
    })
})

describe("R9 small pass", () => {
    it("a channel closed without ever starting is still never-opened", () => {
        expect(neverOpened([channel(Online, EVotingStatus.CLOSED)])).toBe(true)
        expect(
            neverOpened([
                channel(Online, EVotingStatus.CLOSED, {firstStartedAt: "2028-01-01T00:00:00Z"}),
            ])
        ).toBe(false)
    })
    it("applies every channel when the event enables none", () => {
        const none = [Online, Kiosk, EarlyVoting, Telephone].map((name) =>
            channel(name, EVotingStatus.NOT_STARTED, {enabled: false})
        )
        expect(eventStartChannels(none)).toEqual([Online, Kiosk, EarlyVoting, Telephone])
    })
})
