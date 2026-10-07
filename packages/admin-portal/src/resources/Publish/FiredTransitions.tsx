// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useGetList, useGetOne} from "react-admin"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Stack, Typography} from "@mui/material"
import type {Sequent_Backend_Election, Sequent_Backend_Scheduled_Event} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {useTenantStore} from "@/providers/TenantContextProvider"
import type {IFiredOutcome, IFiredPost, IRetainedSignedClose} from "@/types/lifecycle"
import {useTimeZoneService} from "@/components/timezones/timeZoneService"
import {useTimeZoneContext} from "@/components/timezones/useTimeZoneContext"
import {GET_SCHEDULED_OUTCOMES, type GetScheduledOutcomesData} from "@/queries/Lifecycle"
import {RetainedSignedCloses} from "@/components/timezones/RetainedSignedCloses"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {ETransition, ScheduledTransitionCard} from "./ScheduledTransitionCard"

const TRANSITIONS: Record<string, ETransition> = {
    "open-voting": ETransition.OPENED,
    "close-voting": ETransition.CLOSED,
}

/** One fired opening or closing at the election, newest first. */
export interface IFiredAtElection {
    id: string
    transition: ETransition
    at: string
    post: IFiredPost
}

/**
 * The scheduled openings and closings that fired at `electionId`: its own rows
 * and the event-wide ones, read from each row's `fired_outcome`.
 */
export const firedAtElection = (
    events: ReadonlyArray<
        Pick<
            Sequent_Backend_Scheduled_Event,
            "id" | "event_processor" | "event_payload" | "annotations"
        >
    >,
    electionId: string
): Array<IFiredAtElection> =>
    events
        .flatMap((event) => {
            const fired = (event.annotations as {fired_outcome?: IFiredOutcome} | null)
                ?.fired_outcome
            if (!fired) return []
            const post = fired.posts.find((entry) => entry.election_id === electionId)
            const transition = post ? TRANSITIONS[post.action] : undefined
            return post && transition
                ? [{id: String(event.id), transition, at: post.fired_at ?? fired.at, post}]
                : []
        })
        .sort((a, b) => Date.parse(b.at) - Date.parse(a.at))

/** Only the same signed close record can replace the retained processing card. */
export const matchesRetainedClose = (close: IRetainedSignedClose, post: IFiredPost): boolean =>
    post.action === "close-voting" &&
    post.outcome === "runs" &&
    post.scheduled_event_id === close.scheduled_event_id &&
    post.election_id === close.election_id &&
    post.fingerprint === close.fingerprint &&
    post.authorized_by?.request_id === close.authorized_by.request_id

/** The Publish tab's cards of what the schedule did at an election. */
export const FiredTransitions: React.FC<{electionEventId: string; electionId: string}> = ({
    electionEventId,
    electionId,
}) => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const aliasRenderer = useAliasRenderer()
    const {data: election} = useGetOne<Sequent_Backend_Election>("sequent_backend_election", {
        id: electionId,
    })
    const {data: outcomeData, error: outcomesError} = useQuery<GetScheduledOutcomesData>(
        GET_SCHEDULED_OUTCOMES,
        {
            context: {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_READ}},
            variables: {electionEventId},
            skip: !electionEventId,
            pollInterval: 30_000,
        }
    )
    const service = useTimeZoneService()
    const zones = useTimeZoneContext(electionEventId)
    const {data} = useGetList<Sequent_Backend_Scheduled_Event>("sequent_backend_scheduled_event", {
        pagination: {page: 1, perPage: 9999},
        filter: {
            tenant_id: tenantId,
            election_event_id: electionEventId,
        },
    })
    const fired = firedAtElection(data ?? [], electionId)
    const retained = (outcomeData?.get_scheduled_outcomes?.retained_closes ?? []).filter(
        (close) => close.election_id === electionId
    )
    const fallback = retained.filter(
        (close) =>
            !close.fired_at || !fired.some((entry) => matchesRetainedClose(close, entry.post))
    )
    if (!fired.length && !fallback.length && !outcomesError) return null
    const zone = zones.zoneOf(electionId)
    return (
        <Stack spacing={1} sx={{mb: 2}} data-testid="fired-transitions">
            <RetainedSignedCloses
                closes={fallback}
                zoneOf={zones.zoneOf}
                nameOf={() =>
                    election ? aliasRenderer(election) : t("eventsScreen.fields.electionId")
                }
                unavailable={!!outcomesError}
            />
            {fired.map(({id, transition, at, post}) => {
                const signed = retained.find((close) => matchesRetainedClose(close, post))
                return (
                    <Stack key={id} spacing={0.5}>
                        <ScheduledTransitionCard
                            transition={transition}
                            time={service.formatDateTimeZone(at, zone, service.text)}
                            post={post}
                            zone={zone}
                        />
                        {signed ? (
                            <Typography variant="body2" color="text.secondary">
                                {t("lifecycle.signedClose.signedAt", {
                                    time: service.formatDateTimeZone(
                                        signed.scheduled_at,
                                        zone,
                                        service.text
                                    ),
                                })}
                            </Typography>
                        ) : null}
                    </Stack>
                )
            })}
        </Stack>
    )
}
