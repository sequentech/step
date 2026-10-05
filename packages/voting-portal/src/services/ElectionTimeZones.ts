// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import {
    IElectionDates,
    IElectionEventPresentation,
    IElectionPresentation,
    VotingPortalDateTimeEvent,
    effectiveTimeZone,
    formatVoterDateTimeZone,
    formatVotingPortalDateTime,
    primaryTimeZone,
} from "@sequentech/ui-core"
import {getEndDateEntry, hasDate, sameWallClock} from "@sequentech/ui-essentials"
import type {ZonedDateTimeFormatter} from "@sequentech/ui-essentials"

/** The zones a ballot's dates show in. */
export interface IBallotTimeZones {
    /** The Post's (election's) zone. */
    timeZone: string
    /** The event's primary zone, where the common close is set. */
    closeTimeZone: string
}

type EventZones = Pick<IElectionEventPresentation, "timezones"> | null | undefined

const hasTimeZones = (event: EventZones): boolean => !!event?.timezones?.primary?.trim()

/**
 * The zones of one ballot (VOTE-LIFECYCLE design §3, §8), by the sequent-core
 * rule. The event's `timezones` come from the published event (the
 * publication files or `election_event_config.json`), the election's
 * `timezone` from the published election; the ballot style's copies stand in
 * when those aren't loaded.
 *
 * Nothing when neither copy of the event configures timezones (an event
 * published before them): its dates then show as before, plain times in the
 * browser's zone with no zone named.
 */
export const ballotTimeZones = (
    event: EventZones,
    election: Pick<IElectionPresentation, "timezone"> | null | undefined,
    ballotStyle?: {
        election_event_presentation?: Pick<IElectionEventPresentation, "timezones">
        election_presentation?: Pick<IElectionPresentation, "timezone">
    } | null
): IBallotTimeZones | undefined => {
    const eventZones = hasTimeZones(event)
        ? event
        : hasTimeZones(ballotStyle?.election_event_presentation)
          ? ballotStyle?.election_event_presentation
          : undefined
    if (!eventZones) {
        return undefined
    }
    const electionZone = election?.timezone ? election : ballotStyle?.election_presentation
    return {
        timeZone: effectiveTimeZone(eventZones, electionZone),
        closeTimeZone: primaryTimeZone(eventZones),
    }
}

/**
 * A time on a ballot's screens (the Ballot Locator's logs): in the Post's zone
 * with the zone named, or, without zones, as before.
 */
export const formatBallotTime = (
    instant: Date,
    {
        event,
        zones,
        t,
        lang,
    }: {
        event: VotingPortalDateTimeEvent | null | undefined
        zones: IBallotTimeZones | undefined
        t: TFunction
        lang: string
    }
): string =>
    zones
        ? formatVoterDateTimeZone(instant, zones.timeZone, {
              t,
              lang,
              formatDateTime: (date, zone) => formatVotingPortalDateTime(date, event, lang, zone),
          })
        : formatVotingPortalDateTime(instant, event, lang)

export interface IListedBallot {
    electionDates?: IElectionDates
    /** Absent for an event without timezones. */
    zones?: IBallotTimeZones
}

export interface IVotingClosedInput {
    ballots: Array<IListedBallot>
    now: Date
    t: TFunction
    lang: string
    formatDateTime: ZonedDateTimeFormatter
}

/**
 * The ballot list's closed message, once the event's voting is closed (the
 * caller checks that) and every listed ballot's close has passed: the latest
 * close, including a recorded manual stop before the scheduled deadline,
 * in the zone it was scheduled in (the primary for the common close)
 * with the Post's time in brackets (`electionSelectionScreen.votingClosedAt`),
 * or once when both read the same (`electionSelectionScreen.votingClosedOn`).
 * Nothing while a close is still ahead (a Post's test voting closing before its
 * election opens isn't "voting closed"), and nothing for an event without
 * timezones, whose list shows no closed message, as before.
 */
export const votingClosedMessage = ({
    ballots,
    now,
    t,
    lang,
    formatDateTime,
}: IVotingClosedInput): string | undefined => {
    let latest: {end: string; endZone?: string; zones?: IBallotTimeZones} | undefined
    for (const ballot of ballots) {
        const entry = getEndDateEntry(ballot.electionDates, now)
        if (!entry || !hasDate(entry.date)) {
            continue
        }
        if (new Date(entry.date) > now) {
            return undefined
        }
        if (!latest || new Date(entry.date) > new Date(latest.end)) {
            latest = {end: entry.date, endZone: entry.timeZone, zones: ballot.zones}
        }
    }
    if (!latest?.zones) {
        return undefined
    }
    const {end, zones} = latest
    const closeZone = latest.endZone ?? zones.closeTimeZone
    const options = {
        t,
        lang,
        formatDateTime: (instant: Date, zone: string) =>
            formatDateTime(instant.toISOString(), zone),
    }
    const close = formatVoterDateTimeZone(end, closeZone, options)
    if (sameWallClock(end, zones.timeZone, closeZone)) {
        return t("electionSelectionScreen.votingClosedOn", {close})
    }
    return t("electionSelectionScreen.votingClosedAt", {
        close,
        localClose: formatVoterDateTimeZone(end, zones.timeZone, options),
    })
}
