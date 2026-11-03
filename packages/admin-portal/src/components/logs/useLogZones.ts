// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMemo} from "react"
import {useGetMany, useGetOne, useListContext} from "react-admin"
import {
    ELogTimeZonePolicy,
    type IElectionEventPresentation,
    type IElectionPresentation,
    type ITenantSettings,
} from "@sequentech/ui-core"
import type {Sequent_Backend_Election, Sequent_Backend_Tenant} from "@/gql/graphql"
import {DEFAULT_TIME_ZONE, logTimeZone, primaryTimeZone} from "@/lib/timezones/zonedFormat"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useEventPresentation, type ZonedEventRef} from "@/hooks/useZonedFormat"
import {useMyTimeZone} from "@/providers/EventTimeZoneProvider"
import {logMessage} from "./logMessage"
import {ELogScope} from "./logScope"

export interface ILogZones {
    /** The log the zones are of. */
    scope: ELogScope
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
            scope: ELogScope.EVENT,
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
 * The tenant log's zones: the tenant setting `logs_timezone` (UTC when not
 * set), then mine. The tenant is read only when `enabled`.
 */
export const useTenantLogZones = (enabled = true): ILogZones => {
    const [tenantId] = useTenantStore()
    const myZone = useMyTimeZone()
    const {data: tenant} = useGetOne<Sequent_Backend_Tenant>(
        "sequent_backend_tenant",
        {id: tenantId ?? ""},
        {enabled: enabled && !!tenantId}
    )
    const configured = (tenant?.settings as ITenantSettings | undefined)?.logs_timezone
    return useMemo(() => {
        const primary = configured || DEFAULT_TIME_ZONE
        return {
            scope: ELogScope.TENANT,
            primary,
            byElection: false,
            choices: Array.from(new Set([primary, myZone])),
        }
    }, [configured, myZone])
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
        // Every entry of the tenant log is in the tenant's log zone.
        if (zones.scope === ELogScope.TENANT) return () => zones.primary
        return (electionId) =>
            logTimeZone(zones.presentation, electionId ? presentations.get(electionId) : undefined)
    }, [elections, zones.presentation, zones.scope, zones.primary])
}
