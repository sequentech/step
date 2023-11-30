// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/material"

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import styled from "@emotion/styled"
import Stats from "../dashboard/Stats"
import {VotesByChannel, VotesByDay} from "../dashboard/Charts"

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
`

export default function DashboardElection() {
    return (
        <>
            <Box sx={{width: 1024, marginX: "auto"}}>
                <Box>
                    <Stats />

                    <Container>
                        <VotesByDay />
                        <VotesByChannel />
                    </Container>
                </Box>
            </Box>
        </>
    )
}
