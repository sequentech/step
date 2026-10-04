// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import {
    IElectionDates,
    canonicalZone,
    formatOnThisDevice,
    formatVoterDateTimeZone,
    zoneOffsetMinutes,
} from "@sequentech/ui-core"

/**
 * The algorithm for election start date in voting portal's election list should
 * be:
 *
 * 1. If there's a scheduled event for start voting period, use that date
 * 2. Or else, if there's a scheduled event for allow initialization report, use
 *    that date
 * 3. Or else, if the election has been started, use that execution date (field
 *    `StringifiedPeriodDates::first_started_at`)
 *
 * The previously mentioned start-date should be applied in relation to:
 * - the shown start date related to the election
 * - the countdown to start
 *
 * The rationale for prioritizing the scheduled dates instead of the actual
 * execution dates is so that the dates don't change for voters.
 * */
export const getStartDate = (electionDates?: IElectionDates): string | null => {
    return (
        electionDates?.scheduled_event_dates?.START_VOTING_PERIOD?.scheduled_at ||
        electionDates?.scheduled_event_dates?.ALLOW_INIT_REPORT?.scheduled_at ||
        electionDates?.first_started_at ||
        null
    )
}

/**
 *
 * The algorithm for the election end date in voting portal's election list
 * should be:
 *
 * 1. if there's a scheduled event for end voting period, use that date
 * 2. or else, if there's a scheduled event for allow end voting period, use
 *    that date
 * 3. or else, if the election has been stopped, use the execution date (field
 *    `StringifiedPeriodDates::last_stopped_at`)
 */
export const getEndDate = (electionDates?: IElectionDates): string | null =>
    getEndDateEntry(electionDates)?.date ?? null

/**
 * The end date by the same algorithm, with the zone it was scheduled in when
 * the scheduled date names one (a Post's own close names the Post's zone, the
 * event-wide close the primary). An execution date names none.
 */
export const getEndDateEntry = (
    electionDates?: IElectionDates
): {date: string; timeZone?: string} | null => {
    const authoritative = electionDates?.authoritative_close
    if (authoritative !== undefined) {
        return authoritative.scheduled_at
            ? {
                  date: authoritative.scheduled_at,
                  timeZone: authoritative.timezone?.trim() || undefined,
              }
            : null
    }
    const scheduled = electionDates?.scheduled_event_dates
    for (const entry of [scheduled?.END_VOTING_PERIOD, scheduled?.ALLOW_VOTING_PERIOD_END]) {
        if (entry?.scheduled_at) {
            return {date: entry.scheduled_at, timeZone: entry.timezone?.trim() || undefined}
        }
    }
    return electionDates?.last_stopped_at ? {date: electionDates.last_stopped_at} : null
}

/** A stored date that names an instant ("-" marks a date that isn't set). */
export const hasDate = (date: string | null | undefined): date is string =>
    !!date && date.length > 0 && date !== "-"

/** Formats an instant's date and time in a zone, without naming the zone. */
export type ZonedDateTimeFormatter = (input: string, timeZone: string) => string

/**
 * Whether an instant reads the same in two zones: the same zone, or two zones
 * at the same offset then (Asia/Dubai and Asia/Muscat). One line is enough.
 */
export const sameWallClock = (instant: string, zone: string, other: string): boolean =>
    canonicalZone(zone) === canonicalZone(other) ||
    zoneOffsetMinutes(zone, new Date(instant)) === zoneOffsetMinutes(other, new Date(instant))

export interface IElectionTimesInput {
    electionDates?: IElectionDates
    /** The Post's (election's) zone: the opening and the Post's line. */
    timeZone: string
    /**
     * The zone of a close whose date doesn't name its own: the event's primary
     * (where the common close is set).
     */
    closeTimeZone: string
    /** The device's zone. */
    deviceTimeZone: string
    now: Date
    t: TFunction
    lang: string
    formatDateTime: ZonedDateTimeFormatter
}

/** The date lines of one ballot in the ballot list. */
export interface IElectionTimes {
    /** The opening in the Post's zone, the zone named in words. */
    open?: string
    /**
     * The close in the zone it was scheduled in (the primary for the common
     * close), the zone named in words.
     */
    close?: string
    /** The close in the Post's zone, when it reads differently from `close`. */
    closeLocal?: string
    /**
     * `timezones.onThisDevice`: the next relevant instant (the opening while it
     * is ahead, else the close) on the device, when the device's zone reads
     * differently from the Post's.
     */
    device?: string
}

/** The voter's view of a ballot's opening and close (VOTE-LIFECYCLE design §8). */
export const getElectionTimes = ({
    electionDates,
    timeZone,
    closeTimeZone,
    deviceTimeZone,
    now,
    t,
    lang,
    formatDateTime,
}: IElectionTimesInput): IElectionTimes => {
    const start = getStartDate(electionDates)
    const endEntry = getEndDateEntry(electionDates)
    const end = endEntry?.date ?? null
    const closeZone = endEntry?.timeZone ?? closeTimeZone
    const options = {
        t,
        lang,
        formatDateTime: (instant: Date, zone: string) =>
            formatDateTime(instant.toISOString(), zone),
    }
    const inZone = (instant: string, zone: string) =>
        formatVoterDateTimeZone(instant, zone, options)

    const next = hasDate(start) && new Date(start) > now ? start : hasDate(end) ? end : null
    return {
        open: hasDate(start) ? inZone(start, timeZone) : undefined,
        close: hasDate(end) ? inZone(end, closeZone) : undefined,
        closeLocal:
            hasDate(end) && !sameWallClock(end, timeZone, closeZone)
                ? inZone(end, timeZone)
                : undefined,
        device:
            next && !sameWallClock(next, timeZone, deviceTimeZone)
                ? formatOnThisDevice(next, options, deviceTimeZone)
                : undefined,
    }
}
