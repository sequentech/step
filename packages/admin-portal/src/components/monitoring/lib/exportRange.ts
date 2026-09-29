// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum EExportRange {
    /** Nothing chosen, or a start before the end. */
    VALID = "VALID",
    /** The end is not after the start. */
    END_NOT_AFTER_START = "END_NOT_AFTER_START",
}

/**
 * `datetime-local` values are wall-clock times with no zone: the export reads
 * them in the event's time zone, as the dialog says. Seconds are added so the
 * server receives one fixed format.
 */
export function exportBound(value: string): string | undefined {
    if (!value) return undefined
    return value.length === 16 ? `${value}:00` : value
}

export function exportRange(from: string, to: string): EExportRange {
    if (!from || !to) return EExportRange.VALID
    return exportBound(to)! > exportBound(from)!
        ? EExportRange.VALID
        : EExportRange.END_NOT_AFTER_START
}
