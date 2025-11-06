// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import styled from "@emotion/styled"

const StyledContainer = styled.div`
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 0; /* Behind the .main div */
    background: #e0e0e000;
    border-radius: 5px;
    overflow: hidden;
    margin: 0px 0;
    border-bottom: ${({theme}) => `2px solid ${theme.palette.brandColor}`} !important;
`

const StyledBar = styled.div`
    height: 100%;
    background: ${({theme}) => theme.palette.blue.light} !important;
    transition: width 1s linear;
`

interface ICountdownTimer {
    progress?: number
}

const CountdownTimer = ({progress}: ICountdownTimer) => {
    return (
        <StyledContainer className="countdown-bar-container">
            <StyledBar className="countdown-bar" style={{width: `${progress}%`}}></StyledBar>
        </StyledContainer>
    )
}

export default CountdownTimer
