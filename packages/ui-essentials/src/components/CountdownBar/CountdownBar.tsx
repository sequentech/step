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
    background: ${({theme}) => theme.palette.blue.light} !important;
    transition: width 1s linear;
`

interface ICountdownTimer {
    totalDuration?: number
    duration?: number
    onTimeMinReached?: () => void
    minTime?: number
    onCount?: (v) => void
    progress?: number
}

const CountdownTimer = ({
    // duration = 0,
    // onTimeMinReached,
    // minTime = 60,
    // totalDuration = 0,
    // onCount,
    progress,
}: ICountdownTimer) => {
    useEffect(() => {
        console.log("progress", progress)
    }, [progress])
    //relocated logic to satisfy new requirement...
    //could be restored it component ever required to control own state

    // const [timeLeft, setTimeLeft] = useState<number>(duration)
    // const [timeMinReached, setTimeMinReached] = useState(false)

    // useEffect(() => {
    //     if (!progress && duration) {
    //         if (timeLeft > 0) {
    //             if (timeLeft < minTime && !timeMinReached) {
    //                 setTimeMinReached(true)
    //                 onTimeMinReached?.()
    //             }

    //             const timerId = setInterval(() => {
    //                 setTimeLeft(timeLeft - 1)
    //                 onCount?.(timeLeft - 1)
    //             }, 1000)
    //             return () => clearInterval(timerId)
    //         }
    //     }
    // }, [timeLeft])

    // const percentage = progress ?? (timeLeft / totalDuration) * 100

    return (
        <StyledContainer className="countdown-bar-container">
            <StyledBar className="countdown-bar" style={{width: `${progress}%`}}></StyledBar>
        </StyledContainer>
    )
}

export default CountdownTimer
