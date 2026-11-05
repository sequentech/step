// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef, useState, type RefObject} from "react"
import type {Size} from "./types"

export function useElementSize(ref: RefObject<HTMLElement | null>): Size | null {
    const [size, setSize] = useState<Size | null>(null)
    useEffect(() => {
        const element = ref.current
        if (element === null) return
        // The border box: the top bar's and the sheet's padding cover the stage too.
        const observer = new ResizeObserver(([entry]) => {
            const [box] = entry.borderBoxSize
            const width = box?.inlineSize ?? entry.contentRect.width
            const height = box?.blockSize ?? entry.contentRect.height
            setSize((current) =>
                current !== null && current.width === width && current.height === height
                    ? current
                    : {width, height}
            )
        })
        observer.observe(element)
        return () => observer.disconnect()
    }, [ref])
    return size
}

// A message is announced once it has been shown for a moment, and at most once
// per interval, so screen readers are not flooded by frame-by-frame changes.
const SETTLE_MS = 600

export function useThrottledAnnouncement(message: string, intervalMs: number): string {
    const [announced, setAnnounced] = useState(message)
    const last = useRef(0)
    useEffect(() => {
        if (message === announced) return
        const wait = Math.max(SETTLE_MS, last.current + intervalMs - Date.now())
        const timer = setTimeout(() => {
            last.current = Date.now()
            setAnnounced(message)
        }, wait)
        return () => clearTimeout(timer)
    }, [message, announced, intervalMs])
    return announced
}
