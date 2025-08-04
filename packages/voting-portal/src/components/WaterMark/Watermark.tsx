// SPDX-FileCopyrightText: 2024 Sequent Tech <legal[@sequentech.io>](https://github.com/sequentech.io>)
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useMemo} from "react"
import demoBanner from "./assets/demo-banner.png"
import {useAppSelector} from "../../store/hooks"
import {
    selectAllBallotStyles,
    selectFirstBallotStyle,
    showDemo,
} from "../../store/ballotStyles/ballotStylesSlice"
import styled from "@emotion/styled"
import {Box} from "@mui/material"
import {SystemProps} from "@mui/system"
import {useParams} from "react-router-dom"

interface BackgroundProps extends SystemProps {
    imageUrl: string | undefined
}

const Background = styled(Box)<{imageUrl: string | undefined}>`
    position: absolute;
    width: 100%;
    height: 100%;
    overflow: hidden;
    z-index: -1;
    &::before {
        content: "";
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background-image: ${({imageUrl}) => (imageUrl ? `url(${imageUrl})` : "none")};
        background-repeat: repeat;
        background-position: center;
        background-size: 100px 100px;
        opacity: 0.3;
    }
`

const DEMO_URL_PATH = "/demo-banner.png"

const WatermarkBackground: React.FC = () => {
    const {electionId} = useParams<{electionId?: string}>()
    const isDemo = useAppSelector(showDemo(electionId))

    return isDemo ? <Background imageUrl={DEMO_URL_PATH} /> : null
}

export default WatermarkBackground
