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

const SECOND_MS = 1_000
const MINUTE_SECONDS = 60
const HOUR_SECONDS = 60 * MINUTE_SECONDS
const DAY_SECONDS = 24 * HOUR_SECONDS
const EMPTY_COUNTDOWN: TimeLeft = {
    years: 0,
    months: 0,
    weeks: 0,
    days: 0,
    hours: 0,
    minutes: 0,
    seconds: 0,
    totalSeconds: 0,
}

/** Advance calendar months, clamping a month-end date instead of overflowing it. */
function addMonths(date: Date, months: number): Date {
    // Preserve the exact instant in the repeated hour when daylight saving ends.
    if (months === 0) return new Date(date)
    const shifted = new Date(date)
    shifted.setDate(1)
    shifted.setMonth(date.getMonth() + months)
    const lastDay = new Date(shifted.getFullYear(), shifted.getMonth() + 1, 0).getDate()
    shifted.setDate(Math.min(date.getDate(), lastDay))
    return shifted
}

function remainingTime(target: Date, now: Date): TimeLeft {
    const totalSeconds = Math.max(0, Math.ceil((target.getTime() - now.getTime()) / SECOND_MS))
    if (totalSeconds === 0) return EMPTY_COUNTDOWN

    // Count complete calendar months first; the remainder is elapsed time.
    // Subtracting day-of-month fields fails around February and leap years.
    let calendarMonths =
        (target.getFullYear() - now.getFullYear()) * 12 + target.getMonth() - now.getMonth()
    if (addMonths(now, calendarMonths) > target) calendarMonths--
    const anchor = addMonths(now, calendarMonths)
    const seconds = Math.ceil((target.getTime() - anchor.getTime()) / SECOND_MS)
    const days = Math.floor(seconds / DAY_SECONDS)

    return {
        years: Math.floor(calendarMonths / 12),
        months: calendarMonths % 12,
        weeks: Math.floor(days / 7),
        days: days % 7,
        hours: Math.floor(seconds / HOUR_SECONDS) % 24,
        minutes: Math.floor(seconds / MINUTE_SECONDS) % 60,
        seconds: seconds % MINUTE_SECONDS,
        totalSeconds,
    }
}

/** Keep the displayed deadline current without restarting the clock on parent renders. */
export const useSelectElectionCountdown = ({date = ""}: CountdownProps): TimeLeft | null => {
    const [timeLeft, setTimeLeft] = useState<TimeLeft | null>(null)

    useEffect(() => {
        const target = new Date(date)
        if (!Number.isFinite(target.getTime())) {
            setTimeLeft(null)
            return
        }

        const update = () => {
            const remaining = remainingTime(target, new Date())
            setTimeLeft(remaining)
            return remaining.totalSeconds > 0
        }

        // Publish immediately, including already-expired dates; do not briefly
        // show the previous election's time while waiting for the first tick.
        if (!update()) return
        const interval = setInterval(() => {
            if (!update()) clearInterval(interval)
        }, SECOND_MS)
        return () => clearInterval(interval)
    }, [date])

    return timeLeft
}
