// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import {formatNumber} from "@/services/Numbers"
import FenceIcon from "@mui/icons-material/Fence"
import GroupIcon from "@mui/icons-material/Group"
import MarkEmailReadOutlinedIcon from "@mui/icons-material/MarkEmailReadOutlined"
import SmsOutlinedIcon from "@mui/icons-material/SmsOutlined"
import {styled} from "@mui/material/styles"
import StatItem from "../StatItem"

const CardList = styled(Box)`
    display: flex;
    width: 100%;
    justify-content: space-between;
    margin: 20px 0;
`

interface Metrics {
    eligibleVotersCount: number | string
    votersCount: number | string
    electionsCount: number | string
    areasCount: number | string
    emailsSentCount: number | string
    smsSentCount: number | string
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
                count={formatNumber(metrics.electionsCount)}
                label={String(t("electionEventScreen.stats.elections"))}
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
        </CardList>
    )
}
