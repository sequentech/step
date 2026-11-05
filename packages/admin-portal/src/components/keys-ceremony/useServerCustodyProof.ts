// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useRef, useState} from "react"
import {ApolloError, useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {IPermissions} from "@/types/keycloak"
import {REQUEST_SERVER_CUSTODY} from "@/queries/RequestServerCustody"
import {
    resolveCustodyFile,
    SERVER_CUSTODY_TIMEOUT_MS,
    ServerCustodyError,
    type IServerCustodyInput,
    type IServerCustodyReply,
    type IServerCustodyScope,
} from "@/services/serverCustody"

export const useServerCustodyProof = (scope: IServerCustodyScope) => {
    const {t} = useTranslation()
    const [request] = useMutation<
        {request_server_custody: IServerCustodyReply},
        {object: IServerCustodyInput}
    >(REQUEST_SERVER_CUSTODY)
    const [waiting, setWaiting] = useState(false)
    const controller = useRef<AbortController | null>(null)
    const {election_event_id, keys_ceremony_id, tally_session_id} = scope
    const scopeId = `${election_event_id}:${keys_ceremony_id ?? ""}:${tally_session_id ?? ""}`
    const activeScope = useRef<string | null>(scopeId)
    useEffect(() => {
        activeScope.current = scopeId
        return () => {
            activeScope.current = null
            controller.current?.abort()
        }
    }, [scopeId])
    const resolve = useCallback(
        async (text: string) => {
            if (activeScope.current !== scopeId) throw new ServerCustodyError("cancelled")
            controller.current?.abort()
            const current = new AbortController()
            controller.current = current
            setWaiting(true)
            const deadline = setTimeout(
                () => current.abort(new ServerCustodyError("unavailable")),
                SERVER_CUSTODY_TIMEOUT_MS
            )
            try {
                return await resolveCustodyFile(
                    text,
                    {election_event_id, keys_ceremony_id, tally_session_id},
                    async (object) => {
                        const {data, errors} = await request({
                            variables: {object},
                            context: {
                                headers: {"x-hasura-role": IPermissions.TRUSTEE_CEREMONY},
                                fetchOptions: {signal: current.signal},
                            },
                        })
                        if (errors?.length)
                            throw new ServerCustodyError(
                                "failed",
                                errors.map((error) => error.message).join("; ")
                            )
                        if (!data?.request_server_custody)
                            throw new ServerCustodyError("invalidReply")
                        return data.request_server_custody
                    },
                    undefined,
                    undefined,
                    current.signal
                )
            } catch (error) {
                let failure = new ServerCustodyError("unavailable")
                if (current.signal.aborted) {
                    failure =
                        current.signal.reason instanceof ServerCustodyError
                            ? current.signal.reason
                            : new ServerCustodyError("cancelled")
                } else if (error instanceof ServerCustodyError) {
                    failure = error
                } else if (error instanceof ApolloError && error.graphQLErrors.length) {
                    failure = new ServerCustodyError(
                        "failed",
                        error.graphQLErrors.map(({message}) => message).join("; ")
                    )
                }
                failure.message = t(`keysGeneration.serverCustody.${failure.reason}`, {
                    error: failure.detail,
                })
                throw failure
            } finally {
                clearTimeout(deadline)
                if (controller.current === current) setWaiting(false)
            }
        },
        [request, election_event_id, keys_ceremony_id, tally_session_id, scopeId, t]
    )
    return {resolve, waiting}
}
