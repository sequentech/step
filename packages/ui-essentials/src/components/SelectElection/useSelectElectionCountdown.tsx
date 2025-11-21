// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useState, useEffect} from "react"

interface TimeLeft {
    years: number
    months: number
    weeks: number
    days: number
    hours: number
    minutes: number
    seconds: number
    totalSeconds: number
}

interface CountdownProps {
    date?: string
}

export const useSelectElectionCountdown = ({date = ""}: CountdownProps): TimeLeft | null => {
    const [timeLeft, setTimeLeft] = useState<TimeLeft | null>(null)

    const targetDate = date ? new Date(date) : null

    useEffect(() => {
        if (!targetDate || isNaN(targetDate.getTime())) {
            setTimeLeft(null)
            return
        }

        const updateCountdown = () => {
            const now = new Date()
            let years = targetDate.getFullYear() - now.getFullYear()
            let months = targetDate.getMonth() - now.getMonth()
            let days = targetDate.getDate() - now.getDate()
            let hours = targetDate.getHours() - now.getHours()
            let minutes = targetDate.getMinutes() - now.getMinutes()
            let seconds = targetDate.getSeconds() - now.getSeconds()
            let totalSeconds = (targetDate.getTime() - now.getTime()) / 1000

            if (seconds < 0) {
                seconds += 60
                minutes--
            }
            if (minutes < 0) {
                minutes += 60
                hours--
            }
            if (hours < 0) {
                hours += 24
                days--
            }
            if (days < 0) {
                const lastMonth = new Date(now.getFullYear(), now.getMonth(), 0)
                days += lastMonth.getDate()
                months--
            }
            if (months < 0) {
                months += 12
                years--
            }

            const weeks = Math.floor(days / 7)
            days = days % 7

            if (
                years <= 0 &&
                months <= 0 &&
                weeks <= 0 &&
                days <= 0 &&
                hours <= 0 &&
                minutes <= 0 &&
                seconds <= 0
            ) {
                setTimeLeft({
                    years: 0,
                    months: 0,
                    weeks: 0,
                    days: 0,
                    hours: 0,
                    minutes: 0,
                    seconds: 0,
                    totalSeconds: -totalSeconds,
                })
                clearInterval(intervalId)
                return
            }

            setTimeLeft({years, months, weeks, days, hours, minutes, seconds, totalSeconds})
        }

        const intervalId = setInterval(updateCountdown, 1000)

        return () => clearInterval(intervalId)
    }, [targetDate])

    return timeLeft
}
