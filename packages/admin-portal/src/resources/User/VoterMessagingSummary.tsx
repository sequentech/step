// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {useQuery} from "@apollo/client"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ChannelLabel} from "@sequentech/ui-essentials"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useMessagingAccounts} from "@/hooks/useMessagingAccounts"
import {roleContext, useMessagingReadRole} from "@/hooks/useMessagingReadRole"
import {
    GET_VOTER_MESSAGE_ATTEMPTS,
    GET_VOTER_UNSETTLED_MESSAGES,
    ILedgerMessagesResult,
} from "@/queries/GetLedgerMessages"
import {EMessageChannel, EMessagingProvider, ILedgerMessage} from "@/types/messaging"
import {maskDestination} from "@/services/maskDestination"
import {reasonText} from "@/services/messagingReasons"
import {ledgerStatus} from "@/components/logs/LedgerStatus"
import {voterDeliveryProblems, voterMessaging} from "./voterMessaging"

export interface VoterMessagingSummaryProps {
    attributes?: Record<string, unknown> | null
    email?: string | null
    /** The voter's Keycloak ID, as the ledger records it. */
    userId?: string | null
    electionEventId?: string | null
}

const Row: React.FC<React.PropsWithChildren<{label: string; help?: string}>> = ({
    label,
    help,
    children,
}) => (
    <Box sx={{display: "flex", gap: 2, alignItems: "center", minHeight: 32, flexWrap: "wrap"}}>
        <Typography variant="body2" color="text.secondary" sx={{minWidth: 180}} title={help}>
            {label}
        </Typography>
        <Box sx={{display: "flex", gap: 1, flexWrap: "wrap", alignItems: "center"}}>{children}</Box>
    </Box>
)

const formatTime = (iso: string | null | undefined, language: string): string =>
    iso ? new Date(iso).toLocaleString(language) : ""

/**
 * The voter's one channel, as Keycloak verified it: the channel, the masked
 * number or the Messenger Page, the consent time, the last message that
 * failed, and apart from it the last one whose delivery isn't confirmed
 * (never shown as a failure). Read only.
 */
export const VoterMessagingSummary: React.FC<VoterMessagingSummaryProps> = ({
    attributes,
    email,
    userId,
    electionEventId,
}) => {
    const {t, i18n} = useTranslation()
    const [tenantId] = useTenantStore()
    const role = useMessagingReadRole()
    const {channel, consentAt} = voterMessaging(email, attributes)
    const isMessenger = channel?.channel === EMessageChannel.MESSENGER
    const {accounts} = useMessagingAccounts({skip: !isMessenger, summaries: true})
    const {data, loading} = useQuery<ILedgerMessagesResult>(GET_VOTER_UNSETTLED_MESSAGES, {
        variables: {tenantId, electionEventId, voterId: userId},
        skip: !role || !tenantId || !electionEventId || !userId,
        context: roleContext(role),
    })
    const candidates = useMemo(() => data?.sequent_backend_message ?? [], [data])
    const keys = useMemo(
        () =>
            Array.from(
                new Set(
                    candidates
                        .map((message) => message.logical_key)
                        .filter((key): key is string => !!key)
                )
            ).sort(),
        [candidates]
    )
    const {data: attemptsData, loading: attemptsLoading} = useQuery<ILedgerMessagesResult>(
        GET_VOTER_MESSAGE_ATTEMPTS,
        {
            variables: {tenantId, keys},
            skip: !role || !tenantId || keys.length === 0,
            context: roleContext(role),
        }
    )
    const {failure, unconfirmed} = useMemo(
        () => voterDeliveryProblems(candidates, attemptsData?.sequent_backend_message ?? []),
        [candidates, attemptsData]
    )
    const notSet = t("messaging.voter.notSet")

    const page = isMessenger
        ? accounts
              .map((account) => account.sender)
              .find(
                  (sender) =>
                      sender.provider === EMessagingProvider.MESSENGER_SEND_API &&
                      sender.page_id === channel?.messengerPage
              )
        : undefined
    const pageName =
        page?.provider === EMessagingProvider.MESSENGER_SEND_API ? page.page_name : undefined

    const messageText = (message: ILedgerMessage | undefined) => {
        if (!role) return t("messaging.voter.failureNeedsPermission")
        if (loading || attemptsLoading) return "–"
        if (!message) return t("messaging.voter.noFailure")
        const status = ledgerStatus(message.state, message.created_at)
        return [
            status ? t(`messaging.state.${status}`) : message.state,
            formatTime(message.failed_at ?? message.updated_at, i18n.language),
            reasonText(t, message.error),
        ]
            .filter(Boolean)
            .join(" · ")
    }

    return (
        <Box
            component="section"
            aria-label={String(t("messaging.voter.title"))}
            sx={{display: "flex", flexDirection: "column", gap: 0.5, margin: "16px 0"}}
        >
            <ElectionHeaderStyles.Title>{t("messaging.voter.title")}</ElectionHeaderStyles.Title>
            <Row label={t("messaging.voter.channel")}>
                {channel ? (
                    <ChannelLabel
                        channel={channel.channel}
                        label={t(`messaging.channel.${channel.channel}`)}
                    />
                ) : (
                    <Typography variant="body2">{t("messaging.voter.noChannel")}</Typography>
                )}
            </Row>
            {channel ? (
                <Row
                    label={
                        isMessenger
                            ? t("messaging.voter.messengerPage")
                            : t("messaging.voter.destination")
                    }
                >
                    <Typography variant="body2">
                        {isMessenger
                            ? (pageName ?? channel.messengerPage)
                            : (maskDestination(channel.channel, channel.destination) ??
                              // The backend refuses it: shown as the reason a send would give.
                              t("messaging.reason.INVALID_DESTINATION"))}
                    </Typography>
                </Row>
            ) : null}
            <Row label={t("messaging.voter.consent")}>
                <Typography variant="body2">
                    {consentAt ? formatTime(consentAt, i18n.language) : notSet}
                </Typography>
            </Row>
            <Row label={t("messaging.voter.lastFailure")}>
                <Typography variant="body2">{messageText(failure)}</Typography>
            </Row>
            <Row
                label={t("messaging.voter.notConfirmed")}
                help={t("messaging.voter.notConfirmedHelp")}
            >
                <Typography variant="body2">{messageText(unconfirmed)}</Typography>
            </Row>
        </Box>
    )
}
