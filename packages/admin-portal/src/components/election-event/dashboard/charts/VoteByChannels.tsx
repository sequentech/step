import {Typography} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import React from "react"
import Chart, {Props} from "react-apexcharts"
import {Separator, StyledPaper} from "../Charts"

export default function VotesByChannel({width}: {width: number}) {
    const state: Props = {
        options: {
            labels: ["Online", "Paper", "IVR", "Postal"],
        },
        series: [65, 45, 34, 12],
    }

    return (
        <StyledPaper>
            <Chart
                options={state.options}
                series={state.series}
                type="donut"
                width={width}
                height={250}
            />
            <Separator />
            <Typography fontSize="16px" color={theme.palette.customGrey.main}>
                Votes by Channel
            </Typography>
        </StyledPaper>
    )
}
