// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useContext, useEffect, useMemo, useState} from "react"
import {NetworkStatus, useMutation, useQuery} from "@apollo/client"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {demoLabel} from "./messageContent"
import {FAKE_PROVIDER_GET_STARTED, FAKE_PROVIDER_MESSAGE, FAKE_PROVIDER_MESSAGES} from "./queries"
import {
    IFakeGetStartedAnswer,
    IFakeProviderGetStartedResult,
    IFakeProviderMessage,
    IFakeProviderMessageResult,
    IFakeProviderMessagesResult,
} from "./types"

// Both actions are permitted to messaging-account-write only.
const WRITE_ROLE = {headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE}}

const NO_MESSAGES: IFakeProviderMessage[] = []

export interface ISentMessages {
    /**
     * The list action has answered: the fake provider routes are mounted.
     * False while it hasn't, when it fails, and without messaging-account-write
     * (the action would refuse the request, so it isn't sent).
     */
    available: boolean
    messages: IFakeProviderMessage[]
    /** A refresh after the list was shown failed; the last list read stays. */
    error?: Error
    /** A refresh the user asked for is running. */
    refreshing: boolean
    refresh: () => void
}

/**
 * What the dev fake provider received. Read once to decide whether the Sent
 * messages tab exists, then every `refreshMs` while `active` (0: never, as in
 * stories), and only once it has answered: a deep link to the tab on a
 * deployment without the routes never polls a failing action (R6-05).
 */
export const useSentMessages = ({
    active,
    refreshMs,
}: {
    active: boolean
    refreshMs: number
}): ISentMessages => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canWrite = authContext.isAuthorized(true, tenantId, IPermissions.MESSAGING_ACCOUNT_WRITE)
    // Set from the answer of a previous render: the first read never polls.
    const [answered, setAnswered] = useState(false)
    const {data, previousData, error, networkStatus, refetch} =
        useQuery<IFakeProviderMessagesResult>(FAKE_PROVIDER_MESSAGES, {
            skip: !canWrite || !tenantId,
            pollInterval: active && answered && refreshMs > 0 ? refreshMs : 0,
            fetchPolicy: "network-only",
            notifyOnNetworkStatusChange: true,
            context: WRITE_ROLE,
        })
    // After a failed refresh Apollo clears `data`; `previousData` keeps the last answer.
    const list =
        data?.messaging_fake_provider_messages ?? previousData?.messaging_fake_provider_messages
    const messages = useMemo(
        () =>
            list?.messages.map((message) => ({
                ...message,
                sender: demoLabel(message.sender),
                recipient: demoLabel(message.recipient),
            })) ?? NO_MESSAGES,
        [list]
    )
    useEffect(() => setAnswered(!!list), [list])
    const refresh = useCallback(() => {
        refetch().catch(() => undefined)
    }, [refetch])
    return {
        available: !!list,
        messages,
        error: list ? error : undefined,
        refreshing: networkStatus === NetworkStatus.refetch,
        refresh,
    }
}

/** Get Started for a pasted m.me link; a refused request comes back as its reason. */
export const useFakeGetStarted = (): ((link: string) => Promise<IFakeGetStartedAnswer>) => {
    const [getStarted] = useMutation<IFakeProviderGetStartedResult>(FAKE_PROVIDER_GET_STARTED, {
        context: WRITE_ROLE,
    })
    return useCallback(
        async (link: string) => {
            try {
                const {data} = await getStarted({variables: {link}})
                return data?.messaging_fake_provider_get_started ?? {status: null, reason: null}
            } catch (error) {
                // A refused link (no ref, an unknown Page) comes back as the action's error.
                return {status: null, reason: getGraphQLActionErrorReason(error) ?? null}
            }
        },
        [getStarted]
    )
}

export interface IFakeEmailHtml {
    html: string | null
    loading: boolean
    /** The read failed, or the fake no longer keeps the entry. */
    failed: boolean
}

/**
 * The HTML of the email open in the reading pane, read on its own: the list
 * leaves it out (R6-03). Not cached, so the list's polls never drop it.
 */
export const useFakeEmailHtml = (id: string, hasHtml: boolean): IFakeEmailHtml => {
    const {data, loading, error} = useQuery<IFakeProviderMessageResult>(FAKE_PROVIDER_MESSAGE, {
        variables: {id},
        skip: !hasHtml,
        fetchPolicy: "no-cache",
        context: WRITE_ROLE,
    })
    const entry = data?.messaging_fake_provider_message
    return {
        html: entry?.content.html ?? null,
        loading: hasHtml && loading,
        failed: hasHtml && !loading && (!!error || (!!data && !entry?.content.html)),
    }
}
