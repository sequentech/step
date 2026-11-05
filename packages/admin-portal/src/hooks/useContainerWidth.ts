// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useRef, useState} from "react"

/**
 * The width of an element, so a widget can follow the room it has rather
 * than the window's: attach the returned ref; the width is null until the
 * element is measured.
 */
export const useContainerWidth = <T extends HTMLElement>(): [
    (element: T | null) => void,
    number | null,
] => {
    const [width, setWidth] = useState<number | null>(null)
    const observer = useRef<ResizeObserver | null>(null)

    const ref = useCallback((element: T | null) => {
        observer.current?.disconnect()
        observer.current = null
        if (!element) return
        setWidth(Math.round(element.getBoundingClientRect().width))
        if (typeof ResizeObserver === "undefined") return
        observer.current = new ResizeObserver(([entry]) => {
            if (entry) setWidth(Math.round(entry.contentRect.width))
        })
        observer.current.observe(element)
    }, [])

    useEffect(() => () => observer.current?.disconnect(), [])

    return [ref, width]
}
