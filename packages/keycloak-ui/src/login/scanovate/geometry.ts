// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {CaptureStep, type Ellipse, type Rect, type Size} from "./types"

// ID-1 cards (driver's licenses, national IDs) are 85.60 × 53.98 mm.
export const CARD_ASPECT = 1.585
export const OVAL_ASPECT = 1.3
// The progress ring is drawn outside the oval by this many pixels.
export const RING_GAP = 12
export const PILL_HEIGHT = 44

export enum StageLayout {
    Phone = "PHONE",
    Desktop = "DESKTOP",
}

export type Insets = {top: number; bottom: number}

export type Overlay = {
    guide?: Rect
    oval?: Ellipse
    bracket?: Rect
    pillTop: number
}

const PHONE_MARGIN = 22
const PILL_GAP = 13
const DESKTOP_PILL_BOTTOM = 54
// Video step: the oval sits left of centre and the ID frame to its right.
const VIDEO_OVAL_WIDTH = 0.25
const VIDEO_OVAL_CENTER = 0.36
const BRACKET_OFFSET = 0.8
const BRACKET_DROP = 0.15

function isDocument(step: CaptureStep): boolean {
    return step === CaptureStep.Front || step === CaptureStep.Back
}

/**
 * Where the guide frame, the oval and the status pill go on a camera stage of
 * the given size. The free region excludes the top bar and the bottom sheet.
 */
export function overlayFor(
    step: CaptureStep,
    stage: Size,
    insets: Insets,
    layout: StageLayout
): Overlay {
    const top = insets.top
    const bottom = Math.max(stage.height - insets.bottom, top + 1)
    const pillSpace = PILL_HEIGHT + PILL_GAP * 2
    const phone = layout === StageLayout.Phone

    if (isDocument(step)) {
        const available = bottom - top - pillSpace
        const margin = phone ? PHONE_MARGIN : PHONE_MARGIN * 2
        const width = Math.max(
            Math.min(stage.width - margin * 2, available * CARD_ASPECT),
            CARD_ASPECT
        )
        const height = width / CARD_ASPECT
        const x = (stage.width - width) / 2
        if (phone) {
            const y = top + pillSpace + (available - height) / 2
            return {
                guide: {x, y, width, height},
                pillTop: y - PILL_HEIGHT - PILL_GAP,
            }
        }
        const y = top + (available - height) / 2
        return {guide: {x, y, width, height}, pillTop: stage.height - DESKTOP_PILL_BOTTOM}
    }

    const video = step === CaptureStep.Video
    const available = bottom - top - pillSpace
    const maxRy = Math.max(available / 2 - RING_GAP, 1)
    const rx = Math.min(
        stage.width * (video ? VIDEO_OVAL_WIDTH : 0.34),
        stage.height * 0.3,
        maxRy / OVAL_ASPECT
    )
    const ry = rx * OVAL_ASPECT
    const cy = top + available / 2
    const pillTop = phone ? cy + ry + RING_GAP + PILL_GAP * 2 : stage.height - DESKTOP_PILL_BOTTOM
    if (!video) {
        return {oval: {cx: stage.width / 2, cy, rx, ry}, pillTop}
    }
    // The ID is held beside the face, below the eyes, overlapping only the
    // oval's edge, so holding it where it is asked for never covers the face.
    const width = Math.min(stage.width * 0.4, rx * 1.6)
    const height = width / CARD_ASPECT
    const cx = Math.max(
        Math.min(
            stage.width * VIDEO_OVAL_CENTER,
            stage.width - PHONE_MARGIN / 2 - width - rx * BRACKET_OFFSET
        ),
        rx + PHONE_MARGIN / 2
    )
    const bracket = {x: cx + rx * BRACKET_OFFSET, y: cy + ry * BRACKET_DROP, width, height}
    return {oval: {cx, cy, rx, ry}, bracket, pillTop}
}

type Cover = {scale: number; offsetX: number; offsetY: number}

// How `object-fit: cover` places the video inside the stage.
export function coverTransform(stage: Size, video: Size): Cover {
    const scale = Math.max(stage.width / video.width, stage.height / video.height)
    return {
        scale,
        offsetX: (stage.width - video.width * scale) / 2,
        offsetY: (stage.height - video.height * scale) / 2,
    }
}

// A frame at most `longSide` pixels long, keeping the video's aspect ratio.
export function fitSize(video: Size, longSide: number): Size {
    const scale = Math.min(1, longSide / Math.max(video.width, video.height))
    return {
        width: Math.max(1, Math.round(video.width * scale)),
        height: Math.max(1, Math.round(video.height * scale)),
    }
}

function clampRect(rect: Rect, frame: Size): Rect {
    const x = Math.min(Math.max(rect.x, 0), frame.width)
    const y = Math.min(Math.max(rect.y, 0), frame.height)
    return {
        x,
        y,
        width: Math.max(Math.min(rect.x + rect.width, frame.width) - x, 0),
        height: Math.max(Math.min(rect.y + rect.height, frame.height) - y, 0),
    }
}

/**
 * Maps a rectangle drawn over the stage to the analysis frame. The preview is
 * mirrored with a CSS transform, the frame never is.
 */
export function rectToFrame(
    rect: Rect,
    stage: Size,
    video: Size,
    mirrored: boolean,
    frame: Size
): Rect {
    const {scale, offsetX, offsetY} = coverTransform(stage, video)
    const width = rect.width / scale
    const height = rect.height / scale
    const left = (rect.x - offsetX) / scale
    const x = mirrored ? video.width - left - width : left
    const y = (rect.y - offsetY) / scale
    const k = frame.width / video.width
    return clampRect({x: x * k, y: y * k, width: width * k, height: height * k}, frame)
}

export function ellipseToFrame(
    ellipse: Ellipse,
    stage: Size,
    video: Size,
    mirrored: boolean,
    frame: Size
): Ellipse {
    const {scale, offsetX, offsetY} = coverTransform(stage, video)
    const left = (ellipse.cx - offsetX) / scale
    const k = frame.width / video.width
    return {
        cx: (mirrored ? video.width - left : left) * k,
        cy: ((ellipse.cy - offsetY) / scale) * k,
        rx: (ellipse.rx / scale) * k,
        ry: (ellipse.ry / scale) * k,
    }
}
