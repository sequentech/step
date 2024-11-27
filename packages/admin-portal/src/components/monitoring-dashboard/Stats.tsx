// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import StatItem from "./StatItem"
import styled from "@emotion/styled"
import {StatItemProps} from "./StatItem"

const Container = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
`

const SectionContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 8px;
`

const StatsContainer = styled(Box)`
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    align-items: flex-start;
    gap: 12px;
    justify-items: start
    justify-content: start;
`

export interface StatSection {
    title: string
    show: boolean
    stats: StatItemProps[]
}

export interface StatsProps {
    statsSections: StatSection[]
}

const Stats = (props: StatsProps) => {
    const {statsSections} = props

    return (
        <Container>
            {statsSections.map((section) => (
                <>
                    {section.show && (
                        <SectionContainer key={section.title}>
                            <Typography sx={{fontSize: "16px", fontWeight: "500"}}>
                                {section.title}
                            </Typography>
                            <StatsContainer>
                                {section.stats.map((stat) => (
                                    <>{stat.show && <StatItem key={stat.title} {...stat} />}</>
                                ))}
                            </StatsContainer>
                        </SectionContainer>
                    )}
                </>
            ))}
        </Container>
    )
}

export default Stats
