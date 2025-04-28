// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React from "react"
import styled from "@emotion/styled"
import {theme} from "@sequentech/ui-essentials"
import {formatNumber} from "@/services/Numbers"

const Container = styled(Box)`
    display: grid;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-radius: 4px;
    border: 1px solid ${theme.palette.customGrey.light};
    text-align: center;
    width: 250px;
    min-height: 140px;
`

const ItemsContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
`

const ItemContainer = styled(Box)`
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    align-items: flex-start;
    gap: 4px;
    width: 100%;
`

const Title = styled(Typography)`
    margin: 0;
    text-align: center !important;
`

const InfoContainer = styled(Box)`
    display: flex;
    gap: 8px;
`

const Text = styled(Typography)`
    margin: 0;
    color: ${theme.palette.customGrey.main};
`

export interface StatItemProps {
    show: boolean
    title: string
    items: {
        icon: React.ReactNode
        count: number | string
        percentage?: string
        info?: string
    }[]
}

const StatItem = (props: StatItemProps) => {
    const {title, items} = props
    return (
        <Container>
            <Title>{title}</Title>
            <ItemsContainer>
                {items.map((item, index) => (
                    <>
                        {item.info && <Text>{item.info}</Text>}
                        <ItemContainer key={index}>
                            <InfoContainer>
                                {item.icon}
                                <Typography sx={{margin: 0}}>{formatNumber(item.count)}</Typography>
                            </InfoContainer>
                            <Typography sx={{margin: 0}}>
                                {item.percentage ? `${item.percentage}%` : ""}
                            </Typography>
                        </ItemContainer>
                    </>
                ))}
            </ItemsContainer>
        </Container>
    )
}

export default StatItem
