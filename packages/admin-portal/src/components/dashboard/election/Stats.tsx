// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import FenceIcon from "@mui/icons-material/Fence"
import GroupIcon from "@mui/icons-material/Group"
import MarkEmailReadOutlinedIcon from "@mui/icons-material/MarkEmailReadOutlined"
import SmsOutlinedIcon from "@mui/icons-material/SmsOutlined"
import {styled} from "@mui/material/styles"
import {ChannelIcon} from "@sequentech/ui-essentials"
import StatItem from "../StatItem"
import {EMessageChannel} from "@/types/messaging"

const INSTANT_CHANNELS = [
    EMessageChannel.WHATSAPP,
    EMessageChannel.VIBER,
    EMessageChannel.MESSENGER,
] as const
import {formatNumber} from "@/services/Numbers"

const CardList = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    width: 100%;
    justify-content: flex-start;
    margin: 20px 0;
`

interface Metrics {
    eligibleVotersCount: number | string
    votersCount: number | string
    areasCount: number | string
    emailsSentCount: number | string
    smsSentCount: number | string
    messagesSentCount: Record<(typeof INSTANT_CHANNELS)[number], number | string>
}

interface StatsProps {
    metrics: Metrics
}

export const Stats: React.FC<StatsProps> = ({metrics}) => {
    const {t} = useTranslation()
    const iconSize = 60

    return (
        <CardList>
            <StatItem
                icon={<GroupIcon sx={{fontSize: iconSize}} />}
                count={formatNumber(metrics.eligibleVotersCount)}
                label={String(t("electionEventScreen.stats.elegibleVoters"))}
            ></StatItem>
            <StatItem
                icon={<GroupIcon sx={{fontSize: iconSize}} />}
                count={formatNumber(metrics.votersCount)}
                label={String(t("electionEventScreen.stats.voters"))}
            ></StatItem>
            <StatItem
                icon={<FenceIcon sx={{fontSize: iconSize}} />}
                count={formatNumber(metrics.areasCount)}
                label={String(t("electionEventScreen.stats.areas"))}
            ></StatItem>
            <StatItem
                icon={<MarkEmailReadOutlinedIcon sx={{fontSize: iconSize}} />}
                count={formatNumber(metrics.emailsSentCount)}
                label={String(t("electionEventScreen.stats.sentEmails"))}
            ></StatItem>
            <StatItem
                icon={<SmsOutlinedIcon sx={{fontSize: iconSize}} />}
                count={formatNumber(metrics.smsSentCount)}
                label={String(t("electionEventScreen.stats.sentSMS"))}
            ></StatItem>
            {INSTANT_CHANNELS.map((channel) => (
                <StatItem
                    key={channel}
                    icon={<ChannelIcon channel={channel} sx={{fontSize: iconSize}} />}
                    count={formatNumber(metrics.messagesSentCount[channel])}
                    label={String(t(`messaging.stats.sent.${channel}`))}
                ></StatItem>
            ))}
        </CardList>
    )
}
