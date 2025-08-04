// SPDX-FileCopyrightText: 2024 Sequent Tech <legal[@sequentech.io>](https://github.com/sequentech.io>)
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useMemo} from "react"
import demoBanner from "./assets/demo-banner.png"
import {useAppSelector} from "../../store/hooks"
import {selectFirstBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import styled from "@emotion/styled"
import {Box} from "@mui/material"
import {SystemProps} from "@mui/system"

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

const WatermarkBackground: React.FC = () => {
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    const isDemo = useMemo(() => {
        const isDemo = sessionStorage.getItem("isDemo")
        return oneBallotStyle?.ballot_eml.public_key?.is_demo || isDemo
    }, [oneBallotStyle])
    const imageUrlPath = useCallback(() => {
        if (isDemo) {
            return "/demo-banner.png"
        }
    }, [isDemo])

    return imageUrlPath() ? <Background imageUrl={imageUrlPath()} /> : null
}

export default WatermarkBackground
