import React, {useState, useEffect} from "react"
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
    background: #cce5ff;
    transition: width 1s linear;
`

const CountdownTimer = ({duration}) => {
    const [timeLeft, setTimeLeft] = useState(duration)

    useEffect(() => {
        if (timeLeft > 0) {
            const timerId = setInterval(() => {
                setTimeLeft(timeLeft - 1)
            }, 1000)
            return () => clearInterval(timerId)
        }
    }, [timeLeft])

    const percentage = (timeLeft / duration) * 100

    return (
        <StyledContainer className="countdown-bar-container">
            <StyledBar className="countdown-bar" style={{width: `${percentage}%`}}></StyledBar>
        </StyledContainer>
    )
}

export default CountdownTimer
