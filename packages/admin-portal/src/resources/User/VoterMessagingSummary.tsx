// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useQuery} from "@apollo/client"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ChannelLabel} from "@sequentech/ui-essentials"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useMessagingAccounts} from "@/hooks/useMessagingAccounts"
import {roleContext, useMessagingReadRole} from "@/hooks/useMessagingReadRole"
import {GET_VOTER_LAST_FAILURE, ILedgerMessagesResult} from "@/queries/GetLedgerMessages"
import {EMessageChannel, EMessagingProvider} from "@/types/messaging"
import {maskDestination} from "@/services/maskDestination"
import {reasonText} from "@/services/messagingReasons"
import {ledgerStatus} from "@/components/logs/LedgerStatus"
import {voterMessaging} from "./voterMessaging"

export interface VoterMessagingSummaryProps {
    attributes?: Record<string, unknown> | null
    email?: string | null
    /** The voter's Keycloak ID, as the ledger records it. */
    userId?: string | null
    electionEventId?: string | null
}

const Row: React.FC<React.PropsWithChildren<{label: string}>> = ({label, children}) => (
    <Box sx={{display: "flex", gap: 2, alignItems: "center", minHeight: 32, flexWrap: "wrap"}}>
        <Typography variant="body2" color="text.secondary" sx={{minWidth: 180}}>
            {label}
        </Typography>
        <Box sx={{display: "flex", gap: 1, flexWrap: "wrap", alignItems: "center"}}>{children}</Box>
    </Box>
)

const formatTime = (iso: string | null | undefined, language: string): string =>
    iso ? new Date(iso).toLocaleString(language) : ""

/**
 * The voter's one channel, as Keycloak verified it: the channel, the masked
 * number or the Messenger Page, the consent time, and the last message on it
 * that failed or whose delivery couldn't be confirmed. Read only.
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
    const {accounts} = useMessagingAccounts({skip: !isMessenger})
    const {data, loading} = useQuery<ILedgerMessagesResult>(GET_VOTER_LAST_FAILURE, {
        variables: {tenantId, electionEventId, voterId: userId},
        skip: !role || !tenantId || !electionEventId || !userId,
        context: roleContext(role),
    })
    const failure = data?.sequent_backend_message?.[0]
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

    const failureText = () => {
        if (!role) return t("messaging.voter.failureNeedsPermission")
        if (loading) return "–"
        if (!failure) return t("messaging.voter.noFailure")
        const status = ledgerStatus(failure.state, failure.created_at)
        return [
            status ? t(`messaging.state.${status}`) : failure.state,
            formatTime(failure.failed_at ?? failure.updated_at, i18n.language),
            reasonText(t, failure.error),
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
                            : maskDestination(channel.destination)}
                    </Typography>
                </Row>
            ) : null}
            <Row label={t("messaging.voter.consent")}>
                <Typography variant="body2">
                    {consentAt ? formatTime(consentAt, i18n.language) : notSet}
                </Typography>
            </Row>
            <Row label={t("messaging.voter.lastFailure")}>
                <Typography variant="body2">{failureText()}</Typography>
            </Row>
        </Box>
    )
}
