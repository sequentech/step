// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMemo} from "react"
import {useGetMany, useListContext} from "react-admin"
import {
    ELogTimeZonePolicy,
    type IElectionEventPresentation,
    type IElectionPresentation,
} from "@sequentech/ui-core"
import type {Sequent_Backend_Election} from "@/gql/graphql"
import {logTimeZone, primaryTimeZone} from "@/lib/timezones/zonedFormat"
import {useEventPresentation, type ZonedEventRef} from "@/hooks/useZonedFormat"
import {useMyTimeZone} from "@/providers/EventTimeZoneProvider"
import {logMessage} from "./logMessage"

export interface ILogZones {
    presentation?: IElectionEventPresentation
    primary: string
    /** Rows can be in different zones: the ELECTION policy with several configured zones. */
    byElection: boolean
    /** The zones the filter and export offer: the primary, the configured ones, then mine. */
    choices: string[]
}

/** The election event's log zones (sequent-core `log_time_zone`). */
export const useLogZones = (event: ZonedEventRef): ILogZones => {
    const presentation = useEventPresentation(event)
    const myZone = useMyTimeZone()
    return useMemo(() => {
        const configured = presentation?.timezones?.configured ?? []
        const primary = primaryTimeZone(presentation)
        return {
            presentation,
            primary,
            byElection:
                (presentation?.timezones?.logs ?? ELogTimeZonePolicy.ELECTION) ===
                    ELogTimeZonePolicy.ELECTION && configured.length > 1,
            choices: Array.from(new Set([primary, ...configured, myZone])),
        }
    }, [presentation, myZone])
}

/**
 * The zone of a row of the list it is rendered in (inside a react-admin
 * List): the elections of the page's rows are read once, by id, and only when
 * rows can differ.
 */
export const useLogRowZone = (zones: ILogZones): ((electionId?: string | null) => string) => {
    const {data: rows} = useListContext()
    const ids = useMemo(
        () =>
            zones.byElection
                ? Array.from(
                      new Set(
                          (rows ?? [])
                              .map((row) => logMessage(row as {message?: string})?.election_id)
                              .filter((id): id is string => !!id)
                      )
                  ).sort()
                : [],
        [rows, zones.byElection]
    )
    const {data: elections} = useGetMany<Sequent_Backend_Election>(
        "sequent_backend_election",
        {ids},
        {enabled: ids.length > 0}
    )
    return useMemo(() => {
        const presentations = new Map(
            (elections ?? []).map((election) => [
                String(election.id),
                (election.presentation ?? undefined) as IElectionPresentation | undefined,
            ])
        )
        return (electionId) =>
            logTimeZone(zones.presentation, electionId ? presentations.get(electionId) : undefined)
    }, [elections, zones.presentation])
}
