// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
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

const Background = styled(Box)<BackgroundProps>(({imageUrl}) => ({
    "position": "absolute",
    "width": "100%",
    "height": "100%",
    "overflow": "hidden",
    "&::before": {
        content: '""',
        position: "absolute",
        top: 0,
        left: 0,
        width: "100%",
        height: "100%",
        backgroundImage: `url(${imageUrl})`,
        backgroundRepeat: "repeat",
        backgroundPosition: "center",
        backgroundSize: "100px 100px",
        opacity: 0.1,
    },
}))

const WatermarkBackground: React.FC = () => {
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    const isDemo = useMemo(() => {
        return oneBallotStyle?.ballot_eml.public_key?.is_demo
    }, [oneBallotStyle])

    const imageUrl = useCallback(() => {
        if (isDemo) {
            return demoBanner
        }
    }, [isDemo])

    return imageUrl() ? <Background imageUrl={imageUrl()} /> : null
}

export default WatermarkBackground
