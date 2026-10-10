// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useMemo} from "react"
import {useQuery} from "@apollo/client"
import {Identifier, useListContext, useRecordContext} from "react-admin"
import {useTranslation} from "react-i18next"
import {Typography} from "@mui/material"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {roleContext, useMessagingReadRole} from "@/hooks/useMessagingReadRole"
import {GET_LEDGER_MESSAGE_STATES, ILedgerMessagesResult} from "@/queries/GetLedgerMessages"
import {EMessageAttemptState, ILedgerMessage, MESSAGE_ATTEMPT_STATES} from "@/types/messaging"
import {electoralLogDelivery} from "../electoralLogChannel"
import {reasonText} from "@/services/messagingReasons"

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const DAY_MS = 24 * 60 * 60 * 1000

const LedgerContext = createContext<Record<string, ILedgerMessage>>({})

/**
 * Reads the current state of the messages the page's entries record, in one
 * query with messaging-config-read. Without the role the entries show the
 * state they were written with.
 */
export const LedgerStates: React.FC<React.PropsWithChildren> = ({children}) => {
    const {data} = useListContext<{id: Identifier; message?: string}>()
    const [tenantId] = useTenantStore()
    const role = useMessagingReadRole()
    const ids = useMemo(
        () =>
            Array.from(
                new Set(
                    (data ?? [])
                        .map((record) => electoralLogDelivery(record.message)?.messageId)
                        .filter((id): id is string => !!id && UUID.test(id))
                )
            ).sort(),
        [data]
    )
    const {data: ledger} = useQuery<ILedgerMessagesResult>(GET_LEDGER_MESSAGE_STATES, {
        variables: {tenantId, ids},
        skip: !role || !tenantId || ids.length === 0,
        context: roleContext(role),
    })
    const byId = useMemo(
        () =>
            Object.fromEntries(
                (ledger?.sequent_backend_message ?? []).map((message) => [message.id, message])
            ),
        [ledger]
    )
    return <LedgerContext.Provider value={byId}>{children}</LedgerContext.Provider>
}

export type TLedgerStatus = EMessageAttemptState | "UNRESOLVED"

/**
 * The state shown for a message: Unknown for more than 24 hours is
 * Unresolved, never Failed.
 */
export const ledgerStatus = (
    state: string | undefined,
    since: string | null | undefined,
    now: number = Date.now()
): TLedgerStatus | undefined => {
    const known = MESSAGE_ATTEMPT_STATES.find((candidate) => candidate === state)
    if (known === EMessageAttemptState.UNKNOWN && since) {
        const age = now - new Date(since).getTime()
        if (age > DAY_MS) return "UNRESOLVED"
    }
    return known
}

/** The Status of a Logs row: the message's current state in the ledger. */
export const LedgerStatusField: React.FC = () => {
    const {t} = useTranslation()
    const record = useRecordContext<{message?: string}>()
    const ledger = useContext(LedgerContext)
    const delivery = electoralLogDelivery(record?.message)
    if (!delivery?.outcome && !delivery?.messageId) {
        return <span>-</span>
    }
    const current = delivery.messageId ? ledger[delivery.messageId] : undefined
    const status = current
        ? ledgerStatus(current.state, current.created_at)
        : ledgerStatus(delivery.outcome, undefined)
    if (!status) {
        return <span>{delivery.outcome ?? "-"}</span>
    }
    // The reason of the state shown: the ledger's current one, else as logged.
    const reason = reasonText(t, current ? current.error : delivery.reason)
    return (
        <>
            <Typography
                variant="body2"
                component="span"
                title={
                    current
                        ? t(`messaging.stateHelp.${status}`)
                        : t("messaging.logs.statusAsLogged")
                }
                sx={{whiteSpace: "nowrap"}}
            >
                {t(`messaging.state.${status}`)}
            </Typography>
            {reason ? (
                <Typography variant="caption" component="div" color="text.secondary">
                    {reason}
                </Typography>
            ) : null}
        </>
    )
}
