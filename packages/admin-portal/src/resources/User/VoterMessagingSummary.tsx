// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Chip, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ChannelLabel} from "@sequentech/ui-essentials"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {voterMessaging} from "./voterMessaging"

interface VoterMessagingSummaryProps {
    attributes?: Record<string, unknown> | null
}

const Row: React.FC<React.PropsWithChildren<{label: string}>> = ({label, children}) => (
    <Box sx={{display: "flex", gap: 2, alignItems: "center", minHeight: 32}}>
        <Typography variant="body2" color="text.secondary" sx={{minWidth: 180}}>
            {label}
        </Typography>
        <Box sx={{display: "flex", gap: 1, flexWrap: "wrap", alignItems: "center"}}>{children}</Box>
    </Box>
)

/** The voter's messaging channels, as Keycloak verified them. Read only. */
export const VoterMessagingSummary: React.FC<VoterMessagingSummaryProps> = ({attributes}) => {
    const {t} = useTranslation()
    const messaging = voterMessaging(attributes)
    const notSet = t("messaging.voter.notSet")

    return (
        <Box
            component="section"
            aria-label={String(t("messaging.voter.title"))}
            sx={{display: "flex", flexDirection: "column", gap: 0.5, margin: "16px 0"}}
        >
            <ElectionHeaderStyles.Title>{t("messaging.voter.title")}</ElectionHeaderStyles.Title>
            <Row label={t("messaging.voter.preferredChannel")}>
                {messaging.preferredChannel ? (
                    <ChannelLabel
                        channel={messaging.preferredChannel}
                        label={t(`messaging.channel.${messaging.preferredChannel}`)}
                    />
                ) : (
                    <Typography variant="body2">{notSet}</Typography>
                )}
            </Row>
            <Row label={t("messaging.voter.whatsappNumber")}>
                <Typography variant="body2">{messaging.whatsappNumber ?? notSet}</Typography>
            </Row>
            <Row label={t("messaging.voter.viberNumber")}>
                <Typography variant="body2">{messaging.viberNumber ?? notSet}</Typography>
            </Row>
            <Row label={t("messaging.channel.MESSENGER")}>
                <Typography variant="body2">
                    {messaging.messengerConnected
                        ? t("messaging.voter.messengerConnected")
                        : t("messaging.voter.messengerNotConnected")}
                </Typography>
            </Row>
            <Row label={t("messaging.voter.verifiedChannels")}>
                {messaging.verifiedChannels.length ? (
                    messaging.verifiedChannels.map((channel) => (
                        <Chip
                            key={channel}
                            size="small"
                            variant="outlined"
                            label={t(`messaging.channel.${channel}`)}
                        />
                    ))
                ) : (
                    <Typography variant="body2">{t("messaging.voter.noneVerified")}</Typography>
                )}
            </Row>
        </Box>
    )
}
