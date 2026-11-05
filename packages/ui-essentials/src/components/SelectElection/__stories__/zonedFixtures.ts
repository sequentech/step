// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The two configurations every timezone screen is checked under (VOTE-LIFECYCLE
 * design §10): an overseas event (Manila primary, a Post per city) and a
 * staff association (Madrid primary, a Canary Islands office). Shared by the
 * ballot list stories and tests, so both show the same instants.
 */
import {IElectionDates} from "@sequentech/ui-core"

export interface IZonedPost {
    title: string
    /** The Post's zone (`ElectionPresentation.timezone`). */
    timeZone: string
    /** Its opening, at 00:00 or 09:00 on the Post's wall clock. */
    opensAt: string
}

export interface IZonedConfiguration {
    /** The event's primary zone: the common close is set in it. */
    primary: string
    configured: Array<string>
    /** The common close (END_VOTING_PERIOD, event-wide). */
    closesAt: string
    posts: Array<IZonedPost>
    /** A voter's clock before the opening, while open and after the close. */
    now: {before: string; open: string; closed: string}
}

export const OVERSEAS: IZonedConfiguration = {
    primary: "Asia/Manila",
    configured: ["Asia/Manila", "Asia/Dubai", "Africa/Nairobi", "Asia/Muscat"],
    // 08 May 2028, 19:00 in Manila (+08:00).
    closesAt: "2028-05-08T11:00:00.000Z",
    posts: [
        // 09 Apr 2028, 00:00 in Dubai (+04:00).
        {title: "Dubai PCG", timeZone: "Asia/Dubai", opensAt: "2028-04-08T20:00:00.000Z"},
        // 09 Apr 2028, 00:00 in Nairobi (+03:00).
        {title: "Nairobi PE", timeZone: "Africa/Nairobi", opensAt: "2028-04-08T21:00:00.000Z"},
    ],
    now: {
        before: "2028-04-05T06:00:00.000Z",
        open: "2028-04-12T09:30:00.000Z",
        closed: "2028-05-09T02:00:00.000Z",
    },
}

export const ASSOCIATION: IZonedConfiguration = {
    primary: "Europe/Madrid",
    configured: ["Europe/Madrid", "Atlantic/Canary"],
    // 09 Jun 2028, 20:00 in Madrid (CEST, +02:00).
    closesAt: "2028-06-09T18:00:00.000Z",
    posts: [
        // 02 Jun 2028, 09:00 in Madrid (+02:00).
        {title: "Central office", timeZone: "Europe/Madrid", opensAt: "2028-06-02T07:00:00.000Z"},
        // 02 Jun 2028, 09:00 in the Canary Islands (WEST, +01:00).
        {title: "Canary office", timeZone: "Atlantic/Canary", opensAt: "2028-06-02T08:00:00.000Z"},
    ],
    now: {
        before: "2028-05-30T06:00:00.000Z",
        open: "2028-06-05T10:00:00.000Z",
        closed: "2028-06-10T08:00:00.000Z",
    },
}

/**
 * Dates as the ballot styles carry them. Each scheduled date names the zone it
 * was scheduled in when given (get_election_dates copies `cron_config.timezone`:
 * the Post's for its opening, the primary for the event-wide close).
 */
export const zonedElectionDates = (
    opensAt: string,
    closesAt: string,
    {openZone, closeZone}: {openZone?: string; closeZone?: string} = {}
): IElectionDates => ({
    scheduled_event_dates: {
        START_VOTING_PERIOD: {scheduled_at: opensAt, stopped_at: "-", timezone: openZone},
        END_VOTING_PERIOD: {scheduled_at: closesAt, stopped_at: "-", timezone: closeZone},
    },
})

/**
 * The test voting window of a Post, scheduled on its own clock (both rows are
 * the Post's): 09 Feb 00:00 to 08 Apr 23:59 in Dubai.
 */
export const TEST_VOTING_DUBAI = zonedElectionDates(
    "2028-02-08T20:00:00.000Z",
    "2028-04-08T19:59:00.000Z",
    {openZone: "Asia/Dubai", closeZone: "Asia/Dubai"}
)
