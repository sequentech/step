import React from "react"
import styled from "@emotion/styled"

const StyledContainer = styled.div`
    width: 100%;
    height: 100%;
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
