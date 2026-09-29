// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useRef} from "react"
import {DEFAULT_WIDGET_HEIGHT, EColorScheme} from "./types"
import {chartDocument, type ChartDocument} from "./lib/chartDocument"

export interface MonitoringChartFrameProps {
    svg: string
    /** Names the frame for assistive technology. */
    title: string
    height?: number
    colorScheme: EColorScheme
    fontCss?: string
}

/**
 * A rendered chart in a frame that runs no script and loads nothing: an empty
 * `sandbox` and a CSP that allows only inline styles and data: fonts and
 * images. The document is replaced only when it changes, so a poll that brings
 * the same chart does not reload the frame.
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
    return (
        <iframe
            sandbox=""
            srcDoc={shown.current.html}
            title={title}
            data-chart-hash={shown.current.hash}
            style={{display: "block", width: "100%", height, border: 0}}
        />
    )
}
