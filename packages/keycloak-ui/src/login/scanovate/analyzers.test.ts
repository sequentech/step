// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {beforeAll, describe, expect, it} from "vitest"
import {loadAnalyzers} from "./analyzers"
import {DOCUMENT_ANALYSIS_SIDE, FACE_ANALYSIS_SIDE} from "./frames"
import {
    CARD_ASPECT,
    StageLayout,
    ellipseToFrame,
    fitSize,
    overlayFor,
    rectToFrame,
} from "./geometry"
import {
    CaptureStep,
    DocumentStatus,
    FaceStatus,
    type Analyzers,
    type Rect,
    type Size,
} from "./types"

const STAGE: Size = {width: 390, height: 844}
const VIDEO: Size = {width: 720, height: 1280}
const INSETS = {top: 100, bottom: 260}
const FRAMES = 12

function canvasOf(size: Size): CanvasRenderingContext2D {
    const canvas = document.createElement("canvas")
    canvas.width = size.width
    canvas.height = size.height
    const context = canvas.getContext("2d", {willReadFrequently: true})
    if (context === null) {
        throw new Error("Canvas 2D is not available")
    }
    context.fillStyle = "#26343c"
    context.fillRect(0, 0, size.width, size.height)
    return context
}

function paintCard(context: CanvasRenderingContext2D, card: Rect) {
    context.fillStyle = "#e4edf2"
    context.fillRect(card.x, card.y, card.width, card.height)
    context.fillStyle = "#8a9ba6"
    context.fillRect(
        card.x + card.width * 0.06,
        card.y + card.height * 0.2,
        card.width * 0.26,
        card.height * 0.6
    )
    context.fillStyle = "#1f3340"
    for (let line = 0; line < 6; line += 1) {
        context.fillRect(
            card.x + card.width * 0.38,
            card.y + card.height * (0.18 + line * 0.11),
            card.width * (0.52 - (line % 3) * 0.08),
            card.height * 0.04
        )
    }
}

function analyzeDocument(analyzers: Analyzers, card: ((guide: Rect) => Rect) | null) {
    const overlay = overlayFor(CaptureStep.Front, STAGE, INSETS, StageLayout.Phone)
    if (overlay.guide === undefined) {
        throw new Error("The front step has no guide")
    }
    const size = fitSize(VIDEO, DOCUMENT_ANALYSIS_SIDE)
    const guide = rectToFrame(overlay.guide, STAGE, VIDEO, false, size)
    const context = canvasOf(size)
    if (card !== null) {
        paintCard(context, card(guide))
    }
    const image = context.getImageData(0, 0, size.width, size.height)
    analyzers.document.reset()
    let status: DocumentStatus | undefined
    for (let frame = 0; frame < FRAMES; frame += 1) {
        status = analyzers.document.analyze(
            image.data,
            image.width,
            image.height,
            guide.x,
            guide.y,
            guide.width,
            guide.height
        ).status
    }
    return status
}

describe("id-capture analyzers on the capture geometry", () => {
    let analyzers: Analyzers

    beforeAll(async () => {
        analyzers = await loadAnalyzers()
    })

    it("takes a still card that fills the on-screen guide", () => {
        expect(analyzeDocument(analyzers, (guide) => guide)).toBe(DocumentStatus.Ready)
    })

    it("asks to come closer when the card is half the guide", () => {
        const status = analyzeDocument(analyzers, (guide) => {
            const width = guide.width * 0.6
            const height = width / CARD_ASPECT
            return {
                x: guide.x + (guide.width - width) / 2,
                y: guide.y + (guide.height - height) / 2,
                width,
                height,
            }
        })
        expect(status).toBe(DocumentStatus.TooFar)
    })

    it("finds no document in an empty frame", () => {
        expect(analyzeDocument(analyzers, null)).toBe(DocumentStatus.NoDocument)
    })

    it("finds no face in an empty frame", () => {
        const overlay = overlayFor(CaptureStep.Face, STAGE, INSETS, StageLayout.Phone)
        if (overlay.oval === undefined) {
            throw new Error("The face step has no oval")
        }
        const size = fitSize(VIDEO, FACE_ANALYSIS_SIDE)
        const oval = ellipseToFrame(overlay.oval, STAGE, VIDEO, true, size)
        const image = canvasOf(size).getImageData(0, 0, size.width, size.height)
        const frame = analyzers.face.analyze(
            image.data,
            image.width,
            image.height,
            oval.cx,
            oval.cy,
            oval.rx,
            oval.ry
        )
        expect(frame.status).toBe(FaceStatus.NoFace)
    })
})
