// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import demoBanner from "./assets/demo-banner.png"
import {useAppSelector} from "../../store/hooks"
import {showDemo} from "../../store/ballotStyles/ballotStylesSlice"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import {useParams} from "react-router-dom"

// `imageUrl` styles the box and is not an attribute of the element.
const Background = styled(Box, {shouldForwardProp: (prop) => prop !== "imageUrl"})<{
    imageUrl: string | undefined
}>`
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
    const {electionId} = useParams<{electionId?: string}>()
    const isDemo = useAppSelector(showDemo(electionId))

    // Bundled rather than served from `/demo-banner.png`: the portal also runs below another
    // path, as the voter preview another tool frames, where the host's root has no banner.
    return isDemo ? <Background imageUrl={demoBanner} className="watermark-background" /> : null
}

export default WatermarkBackground
