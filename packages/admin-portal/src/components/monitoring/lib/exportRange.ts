// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import moment from "moment-timezone"

export enum EExportRange {
    /** Nothing chosen, or a start before the end. */
    VALID = "VALID",
    /** The end is not after the start. */
    END_NOT_AFTER_START = "END_NOT_AFTER_START",
}

const WALL_CLOCK = ["YYYY-MM-DDTHH:mm", "YYYY-MM-DDTHH:mm:ss"]

/** A `datetime-local` value, a wall-clock time of the event's zone, as that instant. */
function instant(value: string, timeZone: string): moment.Moment | undefined {
    if (!value) return undefined
    const zone = moment.tz.zone(timeZone) ? timeZone : "UTC"
    const parsed = moment.tz(value, WALL_CLOCK, true, zone)
    return parsed.isValid() ? parsed : undefined
}

/**
 * `datetime-local` values are wall-clock times with no zone. They are read in
 * the event's time zone, as the dialog says, and sent as RFC 3339 instants with
 * that zone's offset at that moment, which the export requires: across a change
 * of clocks the two bounds can have different offsets.
 */
export function exportBound(value: string, timeZone: string): string | undefined {
    const parsed = instant(value, timeZone)
    if (!parsed) return undefined
    return parsed.utcOffset() === 0
        ? parsed.utc().format("YYYY-MM-DDTHH:mm:ss[Z]")
        : parsed.format()
}

export function exportRange(from: string, to: string, timeZone: string): EExportRange {
    const start = instant(from, timeZone)
    const end = instant(to, timeZone)
    if (!start || !end) return EExportRange.VALID
    return end.isAfter(start) ? EExportRange.VALID : EExportRange.END_NOT_AFTER_START
}
