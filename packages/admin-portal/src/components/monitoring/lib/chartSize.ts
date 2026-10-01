// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** A frame grows with its chart up to this many widget heights, then scrolls. */
export const CHART_MAX_HEIGHT_FACTOR = 2

/** The chart's shape, from its root `<svg>`: height over width, and its own width in px. */
export interface ChartSize {
    aspect: number
    width?: number
}

const ROOT_TAG = /^\s*<svg\b([^>]*)>/i
const VIEW_BOX = /(?:^|\s)viewBox\s*=\s*(["'])(.*?)\1/
const PIXEL_WIDTH = /(?:^|\s)width\s*=\s*(["'])\s*(\d+(?:\.\d+)?)\s*(?:px)?\s*\1/

/**
 * The size of a sanitized chart, read from the markup rather than inside the
 * frame, where no script runs. `null` without a usable `viewBox`.
 */
export function chartSize(svg: string): ChartSize | null {
    const attributes = ROOT_TAG.exec(svg)?.[1]
    if (attributes === undefined) return null
    const viewBox = VIEW_BOX.exec(attributes)?.[2]
    if (!viewBox) return null
    const parts = viewBox
        .trim()
        .split(/[\s,]+/)
        .map(Number)
    if (parts.length !== 4 || parts.some((part) => !Number.isFinite(part))) return null
    const [, , boxWidth, boxHeight] = parts
    if (boxWidth <= 0 || boxHeight <= 0) return null
    const width = Number(PIXEL_WIDTH.exec(attributes)?.[2])
    return width > 0 ? {aspect: boxHeight / boxWidth, width} : {aspect: boxHeight / boxWidth}
}

/**
 * The frame's height: the chart's own at the width it is drawn (its width, at
 * most the frame's), up to {@link CHART_MAX_HEIGHT_FACTOR} widget heights.
 * The widget height when the chart has no shape or the frame is not measured.
 */
export function frameHeight(
    size: ChartSize | null,
    frameWidth: number | null,
    widgetHeight: number
): number {
    if (!size || !frameWidth || frameWidth <= 0) return widgetHeight
    const drawn = Math.min(frameWidth, size.width ?? frameWidth)
    return Math.min(Math.ceil(drawn * size.aspect), CHART_MAX_HEIGHT_FACTOR * widgetHeight)
}
