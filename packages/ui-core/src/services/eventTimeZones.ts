// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Which timezone applies where (VOTE-LIFECYCLE design §3): a port of
 * sequent-core `time_zones.rs`, so the portals pick the same zone as the
 * server. Zone names only; `timeZones.ts` does the zone arithmetic and texts.
 *
 * The one rule: a row's zone is its election's `timezone` when that zone is
 * configured on the event, else the event's primary, else `UTC`.
 */
import type {IElectionEventPresentation} from "../types/ElectionEventPresentation"
import type {IElectionPresentation} from "../types/ElectionPresentation"

/** The zone used when an event configures none. */
export const DEFAULT_TIME_ZONE = "UTC"

type EventZones = Pick<IElectionEventPresentation, "timezones"> | null | undefined
type ElectionZone = Pick<IElectionPresentation, "timezone"> | null | undefined

/** The event's primary zone, or {@link DEFAULT_TIME_ZONE}. */
export const primaryTimeZone = (event: EventZones): string =>
    event?.timezones?.primary?.trim() || DEFAULT_TIME_ZONE

/**
 * The zone of an election (or of an event-wide row when `election` is absent):
 * the election's own zone when it is one of the configured zones, else the
 * primary.
 */
export const effectiveTimeZone = (event: EventZones, election?: ElectionZone): string => {
    const configured = event?.timezones?.configured ?? []
    const zone = election?.timezone?.trim()
    return zone && configured.includes(zone) ? zone : primaryTimeZone(event)
}
