// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/material"

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import styled from "@emotion/styled"

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
`

export default function DashboardElection() {
    return (
        <>
            <Box sx={{width: 1024, marginX: "auto"}}>
                <BreadCrumbSteps
                    labels={[
                        "electionEventBreadcrumbSteps.created",
                        "electionEventBreadcrumbSteps.keys",
                        "electionEventBreadcrumbSteps.publish",
                        "electionEventBreadcrumbSteps.started",
                        "electionEventBreadcrumbSteps.ended",
                        "electionEventBreadcrumbSteps.results",
                    ]}
                    selected={1}
                    variant={BreadCrumbStepsVariant.Circle}
                    colorPreviousSteps={true}
                />

                <Box>huhu</Box>
            </Box>
        </>
    )
}
