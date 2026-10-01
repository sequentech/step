// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {
    CARD_ASPECT,
    RING_GAP,
    StageLayout,
    coverTransform,
    ellipseToFrame,
    fitSize,
    overlayFor,
    rectToFrame,
} from "./geometry"
import {CaptureStep, type Ellipse, type Rect, type Size} from "./types"
import {FACE_ANALYSIS_SIDE} from "./frames"

const FACE_SIDE = (video: Size) => fitSize(video, FACE_ANALYSIS_SIDE)
const DOCUMENT_SIDE = (video: Size) => fitSize(video, 480)

const PHONE = {width: 390, height: 844}
const INSETS = {top: 110, bottom: 250}

describe("coverTransform", () => {
    it("fills the stage and crops the overflowing side", () => {
        // A landscape 1280×720 video on a portrait phone: the height fills.
        const cover = coverTransform(PHONE, {width: 1280, height: 720})
        expect(cover.scale).toBeCloseTo(844 / 720)
        expect(cover.offsetY).toBeCloseTo(0)
        expect(cover.offsetX).toBeCloseTo((390 - 1280 * (844 / 720)) / 2)
    })
})

describe("fitSize", () => {
    it("downscales the long side and keeps the aspect ratio", () => {
        expect(fitSize({width: 1920, height: 1080}, 480)).toEqual({width: 480, height: 270})
        expect(fitSize({width: 1080, height: 1920}, 320)).toEqual({width: 180, height: 320})
    })

    it("never upscales", () => {
        expect(fitSize({width: 640, height: 480}, 1920)).toEqual({width: 640, height: 480})
    })
})

describe("rectToFrame", () => {
    const video = {width: 720, height: 1280}
    const frame = fitSize(video, 480)

    it("maps the stage centre to the frame centre", () => {
        const rect = {x: 145, y: 372, width: 100, height: 100}
        const mapped = rectToFrame(rect, PHONE, video, false, frame)
        expect(mapped.x + mapped.width / 2).toBeCloseTo(frame.width / 2, 0)
        expect(mapped.y + mapped.height / 2).toBeCloseTo(frame.height / 2, 0)
    })

    it("accounts for the cropped margins of object-fit: cover", () => {
        const {scale, offsetX} = coverTransform(PHONE, video)
        const mapped = rectToFrame({x: 0, y: 0, width: 10, height: 10}, PHONE, video, false, frame)
        const k = frame.width / video.width
        expect(mapped.x).toBeCloseTo((-offsetX / scale) * k)
        expect(mapped.width).toBeCloseTo((10 / scale) * k)
    })

    it("mirrors horizontally for the front camera preview", () => {
        const rect = {x: 20, y: 300, width: 100, height: 60}
        const plain = rectToFrame(rect, PHONE, video, false, frame)
        const mirrored = rectToFrame(rect, PHONE, video, true, frame)
        expect(mirrored.y).toBeCloseTo(plain.y)
        expect(mirrored.width).toBeCloseTo(plain.width)
        expect(mirrored.x + mirrored.width).toBeCloseTo(frame.width - plain.x)
    })

    it("clamps a guide that leaves the frame", () => {
        const mapped = rectToFrame(
            {x: -100, y: -100, width: 1000, height: 2000},
            PHONE,
            video,
            false,
            frame
        )
        expect(mapped).toEqual({x: 0, y: 0, width: frame.width, height: frame.height})
    })
})

describe("ellipseToFrame", () => {
    it("scales the radii and mirrors the centre", () => {
        const video = {width: 1280, height: 720}
        const stage = {width: 640, height: 360}
        const frame = fitSize(video, 320)
        const oval = {cx: 200, cy: 180, rx: 80, ry: 104}
        const plain = ellipseToFrame(oval, stage, video, false, frame)
        expect(plain).toEqual({cx: 100, cy: 90, rx: 40, ry: 52})
        expect(ellipseToFrame(oval, stage, video, true, frame).cx).toBeCloseTo(220)
    })
})

describe("overlayFor", () => {
    it("places an ID-sized guide inside the free area, below the pill", () => {
        const overlay = overlayFor(CaptureStep.Front, PHONE, INSETS, StageLayout.Phone)
        const guide = overlay.guide!
        expect(guide.width / guide.height).toBeCloseTo(CARD_ASPECT)
        expect(guide.x).toBeGreaterThanOrEqual(0)
        expect(guide.x + guide.width).toBeLessThanOrEqual(PHONE.width)
        expect(overlay.pillTop).toBeGreaterThanOrEqual(INSETS.top)
        expect(overlay.pillTop).toBeLessThan(guide.y)
        expect(guide.y + guide.height).toBeLessThanOrEqual(PHONE.height - INSETS.bottom)
    })

    it("keeps the oval and its ring clear of the top bar and the sheet", () => {
        const overlay = overlayFor(CaptureStep.Face, PHONE, INSETS, StageLayout.Phone)
        const oval = overlay.oval!
        expect(oval.cx).toBeCloseTo(PHONE.width / 2)
        expect(oval.ry / oval.rx).toBeCloseTo(1.3)
        expect(oval.cy - oval.ry - RING_GAP).toBeGreaterThanOrEqual(INSETS.top)
        expect(overlay.pillTop).toBeGreaterThan(oval.cy + oval.ry)
        expect(overlay.pillTop).toBeLessThan(PHONE.height - INSETS.bottom)
    })

    it("adds the ID frame to the right of the face, below the eyes", () => {
        const overlay = overlayFor(CaptureStep.Video, PHONE, INSETS, StageLayout.Phone)
        const {oval, bracket} = overlay
        expect(oval!.cx).toBeLessThan(PHONE.width / 2)
        expect(bracket!.x).toBeGreaterThan(oval!.cx + oval!.rx / 2)
        expect(bracket!.y).toBeGreaterThan(oval!.cy)
        expect(bracket!.x + bracket!.width).toBeLessThanOrEqual(PHONE.width)
        expect(bracket!.width / bracket!.height).toBeCloseTo(CARD_ASPECT)
    })

    it("puts the pill at the bottom of the desktop camera", () => {
        const stage = {width: 692, height: 540}
        const overlay = overlayFor(
            CaptureStep.Face,
            stage,
            {top: 24, bottom: 0},
            StageLayout.Desktop
        )
        expect(overlay.pillTop).toBe(stage.height - 54)
        expect(overlay.oval!.cy + overlay.oval!.ry).toBeLessThan(overlay.pillTop)
    })

    it("shrinks to fit a short stage", () => {
        const short = {width: 390, height: 500}
        const overlay = overlayFor(CaptureStep.Face, short, INSETS, StageLayout.Phone)
        expect(overlay.oval!.ry).toBeGreaterThan(0)
        expect(overlay.oval!.cy - overlay.oval!.ry).toBeGreaterThanOrEqual(INSETS.top)
    })
})

// Share of the rectangle's area that falls inside the ellipse, sampled on a grid.
function overlapShare(rect: Rect, ellipse: Ellipse): number {
    const samples = 200
    let inside = 0
    for (let i = 0; i < samples; i += 1) {
        for (let j = 0; j < samples; j += 1) {
            const x = rect.x + ((i + 0.5) / samples) * rect.width
            const y = rect.y + ((j + 0.5) / samples) * rect.height
            const dx = (x - ellipse.cx) / ellipse.rx
            const dy = (y - ellipse.cy) / ellipse.ry
            if (dx * dx + dy * dy <= 1) inside += 1
        }
    }
    return inside / (samples * samples)
}

describe("video step analysis regions", () => {
    // Stages as laid out at 390×844 (full screen, below the top bar and above
    // the sheet) and at 1440×900 (the camera half of the desktop panel).
    const cases = [
        {
            name: "390×844 phone",
            stage: {width: 390, height: 844},
            insets: {top: 110, bottom: 230},
            layout: StageLayout.Phone,
            video: {width: 720, height: 1280},
        },
        {
            name: "1440×900 desktop",
            stage: {width: 692, height: 560},
            insets: {top: 24, bottom: 0},
            layout: StageLayout.Desktop,
            video: {width: 1280, height: 720},
        },
        {
            name: "320×568 small phone",
            stage: {width: 320, height: 568},
            insets: {top: 100, bottom: 200},
            layout: StageLayout.Phone,
            video: {width: 720, height: 1280},
        },
    ]

    for (const {name, stage, insets, layout, video} of cases) {
        it(`keeps the ID frame off the face oval at ${name}`, () => {
            const overlay = overlayFor(CaptureStep.Video, stage, insets, layout)
            const face = FACE_SIDE(video)
            const documentFrame = DOCUMENT_SIDE(video)
            const oval = ellipseToFrame(overlay.oval!, stage, video, true, face)
            const guide = rectToFrame(overlay.bracket!, stage, video, true, documentFrame)
            // Both regions in the same (face frame) coordinates.
            const k = face.width / documentFrame.width
            const guideInFace = {
                x: guide.x * k,
                y: guide.y * k,
                width: guide.width * k,
                height: guide.height * k,
            }
            expect(guide.width).toBeGreaterThan(0)
            expect(overlapShare(guideInFace, oval)).toBeLessThan(0.08)
            // The ID frame is entirely on screen.
            const bracket = overlay.bracket!
            expect(bracket.x).toBeGreaterThanOrEqual(0)
            expect(bracket.x + bracket.width).toBeLessThanOrEqual(stage.width)
            expect(overlay.oval!.cx - overlay.oval!.rx).toBeGreaterThanOrEqual(0)
        })
    }
})
