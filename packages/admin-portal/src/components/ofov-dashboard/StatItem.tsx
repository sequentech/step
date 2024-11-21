// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React from "react"
import styled from "@emotion/styled"
import {theme} from "@sequentech/ui-essentials"

const Container = styled(Box)`
    display: grid;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    border-radius: 4px;
    border: 1px solid ${theme.palette.customGrey.light};
    color: ${theme.palette.customGrey.main};
    text-align: center;
    width: 250px;
    height: 150px;
`

const ItemsContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
`

const ItemContainer = styled(Box)`
    display: flex;
    justify-content: space-between;
    width: 100%;
    align-items: center;
`

interface StatItemProps {
    title: string
    items: {
        icon: React.ReactNode
        info: number | string
        percentageInfo?: string
    }[]
}

const StatItem = (props: StatItemProps) => {
    const {title, items} = props
    return (
        <Container>
            <Typography sx={{margin: 0, justifySelf: "flex-start"}}>{title}</Typography>
            <ItemsContainer>
                {items.map((item, index) => (
                    <ItemContainer key={index}>
                        {item.icon}
                        <Typography sx={{margin: 0}}>{item.info}</Typography>
                        <Typography sx={{margin: 0}}>
                            {item.percentageInfo ? `${item.percentageInfo}%` : ""}
                        </Typography>
                    </ItemContainer>
                ))}
            </ItemsContainer>
        </Container>
    )
}

export default StatItem
