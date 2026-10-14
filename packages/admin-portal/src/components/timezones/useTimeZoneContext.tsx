// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMemo} from "react"
import {useGetList, useGetOne} from "react-admin"
import {
    ELogTimeZonePolicy,
    effectiveTimeZone,
    primaryTimeZone,
    type IElectionEventPresentation,
    type IElectionPresentation,
} from "@sequentech/ui-core"
import type {Sequent_Backend_Election, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"

/** An election as the timezone rule reads it. */
export interface ITimeZoneElection {
    id: string
    presentation?: unknown
}

/** An event's configured timezones, and the zone of each of its elections. */
export interface ITimeZoneContext {
    configured: Array<string>
    primary: string
    logs: ELogTimeZonePolicy
    /**
     * A row's zone (design §3, ui-core `effectiveTimeZone`): its election's
     * timezone if configured, else the event primary, else UTC. Event-wide rows
     * (no election) use the primary.
     */
    zoneOf: (electionId?: string | null) => string
}

/** The timezone context of an event presentation and its elections. */
export const timeZoneContextOf = (
    event: IElectionEventPresentation | null | undefined,
    elections: ReadonlyArray<ITimeZoneElection> = []
): ITimeZoneContext => {
    const primary = primaryTimeZone(event)
    const configured = event?.timezones?.configured?.length ? event.timezones.configured : [primary]
    const zones = new Map<string, string>(
        elections.map((election) => [
            election.id,
            effectiveTimeZone(event, election.presentation as IElectionPresentation | undefined),
        ])
    )
    return {
        configured,
        primary,
        logs: event?.timezones?.logs ?? ELogTimeZonePolicy.ELECTION,
        zoneOf: (electionId) => (electionId && zones.get(electionId)) || primary,
    }
}

/** The timezone context of an election event, read with react-admin. */
export const useTimeZoneContext = (electionEventId?: string | null): ITimeZoneContext => {
    const [tenantId] = useTenantStore()
    const {data: event} = useGetOne<Sequent_Backend_Election_Event>(
        "sequent_backend_election_event",
        {id: electionEventId},
        {enabled: !!electionEventId}
    )
    const {data: elections} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {tenant_id: tenantId, election_event_id: electionEventId},
        },
        {enabled: !!electionEventId}
    )
    // Kept by value: screens re-run effects when the context changes.
    const timezones = (event?.presentation as IElectionEventPresentation | undefined)?.timezones
    const key = JSON.stringify([
        timezones ?? null,
        (elections ?? []).map(({id, presentation}) => [
            id,
            (presentation as IElectionPresentation | undefined)?.timezone ?? null,
        ]),
    ])
    return useMemo(() => {
        const [zones, electionZones] = JSON.parse(key) as [
            IElectionEventPresentation["timezones"] | null,
            Array<[string, string | null]>,
        ]
        return timeZoneContextOf(
            {timezones: zones ?? undefined} as IElectionEventPresentation,
            electionZones.map(([id, timezone]) => ({id, presentation: {timezone}}))
        )
    }, [key])
}
