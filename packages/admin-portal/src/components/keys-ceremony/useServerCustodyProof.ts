// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useRef, useState} from "react"
import {ApolloError, useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {IPermissions} from "@/types/keycloak"
import {REQUEST_SERVER_CUSTODY} from "@/queries/RequestServerCustody"
import {
    resolveCustodyFile,
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
    >(REQUEST_SERVER_CUSTODY, {
        context: {headers: {"x-hasura-role": IPermissions.TRUSTEE_CEREMONY}},
    })
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
            try {
                return await resolveCustodyFile(
                    text,
                    {election_event_id, keys_ceremony_id, tally_session_id},
                    async (object) => {
                        const {data, errors} = await request({variables: {object}})
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
                const failure = current.signal.aborted
                    ? new ServerCustodyError("cancelled")
                    : error instanceof ServerCustodyError
                      ? error
                      : error instanceof ApolloError && error.graphQLErrors.length
                        ? new ServerCustodyError(
                              "failed",
                              error.graphQLErrors.map(({message}) => message).join("; ")
                          )
                        : new ServerCustodyError("unavailable")
                failure.message = t(`keysGeneration.serverCustody.${failure.reason}`, {
                    error: failure.detail,
                })
                throw failure
            } finally {
                if (controller.current === current) setWaiting(false)
            }
        },
        [request, election_event_id, keys_ceremony_id, tally_session_id, scopeId, t]
    )
    return {resolve, waiting}
}
