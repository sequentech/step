// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useEffect, useMemo, useState} from "react"
import {useQuery} from "@apollo/client"
import {EBallotBoxSealPolicy} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {GET_BALLOT_BOX_SEALS} from "@/queries/GetBallotBoxSeals"
import {IPermissions} from "@/types/keycloak"
import {
    EBallotBoxSealStatus,
    type GetBallotBoxSealsQuery,
    type GetBallotBoxSealsQueryVariables,
} from "@/types/ballotBoxSeal"

/** How often readiness is recomputed while a ballot box is pending. */
const PENDING_TICK_MS = 30_000
import {
    EBallotBoxesReadiness,
    type IBallotBoxesSummary,
    summarizeBallotBoxesByElection,
} from "@/services/tallyEligibility"

/** The event's seal policy, from its presentation (an object, or its JSON text). */
export const ballotBoxSealPolicyOf = (presentation: unknown): EBallotBoxSealPolicy => {
    let value = presentation
    if (typeof value === "string") {
        try {
            value = JSON.parse(value)
        } catch {
            value = null
        }
    }
    const policy =
        value && typeof value === "object"
            ? (value as {ballot_box_seal_policy?: unknown}).ballot_box_seal_policy
            : undefined
    return policy === EBallotBoxSealPolicy.SEAL_AT_CLOSE
        ? EBallotBoxSealPolicy.SEAL_AT_CLOSE
        : EBallotBoxSealPolicy.DO_NOT_SEAL
}

const unknownSummary = (readiness: EBallotBoxesReadiness): IBallotBoxesSummary => ({
    readiness,
    total: 0,
    sealed: 0,
    published: 0,
})

/**
 * The ballot boxes of each election (by id) of a new tally, when the event
 * seals them at close (VOTE-FREEZE); undefined when it doesn't, which leaves
 * the tally rules as they were. While the seals load every election is
 * LOADING, and if they can't be read UNAVAILABLE: neither can be selected.
 */
export const useBallotBoxesReadiness = (
    electionEventId: string | undefined,
    electionIds: string[],
    eventPresentation: unknown
): Record<string, IBallotBoxesSummary> | undefined => {
    const {globalSettings} = useContext(SettingsContext)
    const sealAtClose =
        ballotBoxSealPolicyOf(eventPresentation) === EBallotBoxSealPolicy.SEAL_AT_CLOSE
    const {data, error} = useQuery<GetBallotBoxSealsQuery, GetBallotBoxSealsQueryVariables>(
        GET_BALLOT_BOX_SEALS,
        {
            variables: {electionEventId: electionEventId ?? "", electionIds},
            skip: !sealAtClose || !electionEventId || !electionIds.length,
            pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            context: {headers: {"x-hasura-role": IPermissions.TALLY_READ}},
        }
    )
    const seals = data?.sequent_backend_ballot_box_seal
    const failed = !!error
    // A pending box becomes overdue at its deadline: the clock ticks while one is pending.
    const [now, setNow] = useState(() => new Date())
    const anyPending = !!seals?.some((seal) => seal.status === EBallotBoxSealStatus.PENDING)
    useEffect(() => {
        if (!anyPending) return
        setNow(new Date())
        const timer = window.setInterval(() => setNow(new Date()), PENDING_TICK_MS)
        return () => window.clearInterval(timer)
    }, [anyPending])
    const ids = electionIds.join(",")
    return useMemo(() => {
        if (!sealAtClose) return undefined
        const list = ids ? ids.split(",") : []
        if (failed || !seals) {
            const readiness = failed
                ? EBallotBoxesReadiness.UNAVAILABLE
                : EBallotBoxesReadiness.LOADING
            return Object.fromEntries(list.map((id) => [id, unknownSummary(readiness)]))
        }
        return summarizeBallotBoxesByElection(list, seals, now)
    }, [sealAtClose, seals, failed, ids, now])
}
