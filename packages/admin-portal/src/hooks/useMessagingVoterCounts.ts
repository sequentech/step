// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect} from "react"
import {useMutation, useQuery} from "@apollo/client"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {roleContext, useMessagingReadRole} from "@/hooks/useMessagingReadRole"
import {
    GET_MESSAGING_VOTER_COUNTS,
    IGetMessagingVoterCountsResult,
    IRecountMessagingVotersResult,
    IVoterCounts,
    RECOUNT_MESSAGING_VOTERS,
} from "@/queries/GetMessagingVoterCounts"

/** How often counts with a recount on its way are read again. */
export const RECOUNT_POLL_MS = 15_000

/**
 * The Recount button: asks for the event's voters to be counted again, says
 * whether it was asked for or already on its way, then calls `onAsked` (to
 * read the counts again). `undefined` without the read permission.
 */
export const useRecountMessagingVoters = (
    electionEventId: string | undefined,
    onAsked?: () => unknown
): {recount?: () => Promise<void>; recounting: boolean} => {
    const {t} = useTranslation()
    const notify = useNotify()
    const readRole = useMessagingReadRole()
    const [mutate, {loading}] = useMutation<IRecountMessagingVotersResult>(
        RECOUNT_MESSAGING_VOTERS,
        {context: roleContext(readRole)}
    )
    const recount = useCallback(async () => {
        try {
            const {data} = await mutate({variables: {electionEventId}})
            if (!data?.messaging_recount_voters) {
                notify(t("messaging.voterCount.recountError"), {type: "error"})
                return
            }
            notify(
                t(
                    data.messaging_recount_voters.requested
                        ? "messaging.voterCount.recountAsked"
                        : "messaging.voterCount.recountAlreadyAsked"
                ),
                {type: "info"}
            )
            await onAsked?.()
        } catch (error) {
            console.error(error)
            notify(t("messaging.voterCount.recountError"), {type: "error"})
        }
    }, [mutate, electionEventId, notify, t, onAsked])
    return {
        recount: readRole && electionEventId ? recount : undefined,
        recounting: loading,
    }
}

/**
 * An event's voters per channel, as last counted, read again every
 * [`RECOUNT_POLL_MS`] while a recount is on its way; with the Recount
 * button's action.
 */
export const useMessagingVoterCounts = (electionEventId: string | undefined) => {
    const readRole = useMessagingReadRole()
    const {data, loading, error, refetch, startPolling, stopPolling} =
        useQuery<IGetMessagingVoterCountsResult>(GET_MESSAGING_VOTER_COUNTS, {
            variables: {electionEventId},
            skip: !electionEventId || !readRole,
            context: roleContext(readRole),
        })
    const counts: IVoterCounts | null | undefined = data?.messaging_voter_counts
    const pending = !!counts?.status.pending
    useEffect(() => {
        if (!pending) return undefined
        startPolling(RECOUNT_POLL_MS)
        return () => stopPolling()
    }, [pending, startPolling, stopPolling])
    const {recount, recounting} = useRecountMessagingVoters(electionEventId, refetch)
    return {counts, loading, unavailable: !!error, recount, recounting, refetch}
}
