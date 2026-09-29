// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useState, type RefObject} from "react"
import {WIDTH_DEBOUNCE_MS} from "./types"
import {widthBucket} from "./lib/chartDocument"

/**
 * The element's width in 40 px steps: measured at once, then again after a
 * resize settles, so dragging a window does not ask for a render per pixel.
 */
export function useBucketedWidth(ref: RefObject<HTMLElement | null>): number | null {
    const [width, setWidth] = useState<number | null>(null)
    useEffect(() => {
        const element = ref.current
        if (!element) return
        const measure = () => {
            const measured = element.getBoundingClientRect().width
            if (measured > 0) setWidth(widthBucket(measured))
        }
        measure()
        if (typeof ResizeObserver === "undefined") return
        let timer: ReturnType<typeof setTimeout> | undefined
        const observer = new ResizeObserver(() => {
            clearTimeout(timer)
            timer = setTimeout(measure, WIDTH_DEBOUNCE_MS)
        })
        observer.observe(element)
        return () => {
            clearTimeout(timer)
            observer.disconnect()
        }
    }, [ref])
    return width
}
