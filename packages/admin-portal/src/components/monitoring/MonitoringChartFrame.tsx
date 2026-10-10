// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useRef, useState} from "react"
import {DEFAULT_WIDGET_HEIGHT, EColorScheme} from "./types"
import {chartDocument, type ChartDocument} from "./lib/chartDocument"
import {frameHeight} from "./lib/chartSize"

export interface MonitoringChartFrameProps {
    svg: string
    /** Names the frame for assistive technology. */
    title: string
    height?: number
    colorScheme: EColorScheme
    fontCss?: string
}

/** The element's width in px, measured again whenever it changes. */
function useWidth(ref: React.RefObject<HTMLElement | null>): number | null {
    const [width, setWidth] = useState<number | null>(null)
    useEffect(() => {
        const element = ref.current
        if (!element) return
        const measure = () => setWidth(element.getBoundingClientRect().width)
        measure()
        if (typeof ResizeObserver === "undefined") return
        const observer = new ResizeObserver(measure)
        observer.observe(element)
        return () => observer.disconnect()
    }, [ref])
    return width
}

/**
 * A rendered chart in a frame that runs no script and loads nothing: an empty
 * `sandbox` and a CSP that allows only inline styles and data: fonts and
 * images. The document is replaced only when it changes, so a poll that brings
 * the same chart does not reload the frame.
 *
 * The frame is as tall as the chart at the width it is drawn, read from the
 * sanitized SVG's `viewBox`, up to twice the widget height; past that the
 * chart scrolls in it. A chart with no `viewBox` gets the widget height.
 */
export function MonitoringChartFrame({
    svg,
    title,
    height = DEFAULT_WIDGET_HEIGHT,
    colorScheme,
    fontCss,
}: MonitoringChartFrameProps) {
    const shown = useRef<ChartDocument | null>(null)
    const next = useMemo(
        () => chartDocument({svg, colorScheme, fontCss}),
        [svg, colorScheme, fontCss]
    )
    if (shown.current?.hash !== next.hash) shown.current = next
    const element = useRef<HTMLIFrameElement>(null)
    const width = useWidth(element)
    return (
        <iframe
            ref={element}
            sandbox=""
            srcDoc={shown.current.html}
            title={title}
            data-chart-hash={shown.current.hash}
            style={{
                display: "block",
                width: "100%",
                height: frameHeight(shown.current.size, width, height),
                border: 0,
            }}
        />
    )
}
