// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/material"

import styled from "@emotion/styled"
import Stats from "../Stats"
import VotesByDay from "../charts/VoteByDay"
import VotesByChannel from "../charts/VoteByChannels"

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
`

export default function DashboardElection() {
    const cardWidth = 470
    const cardHeight = 250

    return (
        <>
            <Box sx={{width: 1024, marginX: "auto"}}>
                <Box>
                    <Stats />

                    <Container>
                        <VotesByDay width={cardWidth} height={cardHeight} />
                        <VotesByChannel width={cardWidth} height={cardHeight} />
                    </Container>
                </Box>
            </Box>
        </>
    )
}
