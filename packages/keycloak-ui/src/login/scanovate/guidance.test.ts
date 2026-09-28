// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {scanovateEnglish} from "./messages"
import {Tone, canTakePhoto, documentGuidance, faceGuidance, videoGuidance} from "./guidance"
import {CaptureStep, DocumentStatus, FaceStatus} from "./types"

describe("documentGuidance", () => {
    it("has an existing message for every status", () => {
        for (const status of Object.values(DocumentStatus)) {
            const {message} = documentGuidance(CaptureStep.Front, status)
            expect(scanovateEnglish[message as keyof typeof scanovateEnglish]).toBeTruthy()
        }
    })

    it("is green only when the frame is good", () => {
        for (const status of Object.values(DocumentStatus)) {
            const good = status === DocumentStatus.Ready || status === DocumentStatus.HoldStill
            expect(documentGuidance(CaptureStep.Front, status).tone).toBe(
                good ? Tone.Ok : Tone.Guide
            )
        }
    })

    it("asks to turn the ID over for the back", () => {
        expect(documentGuidance(CaptureStep.Back, DocumentStatus.NoDocument).message).toBe(
            "scanovateGuideTurnDocument"
        )
        expect(documentGuidance(CaptureStep.Front, DocumentStatus.NoDocument).message).toBe(
            "scanovateGuidePlaceDocument"
        )
    })

    it("gives distinct advice for distinct problems", () => {
        const problems = [
            DocumentStatus.TooFar,
            DocumentStatus.TooClose,
            DocumentStatus.NotAligned,
            DocumentStatus.TooDark,
            DocumentStatus.TooBright,
            DocumentStatus.Glare,
            DocumentStatus.Blurry,
        ]
        const messages = problems.map((status) => documentGuidance(CaptureStep.Front, status))
        expect(new Set(messages.map(({message}) => message)).size).toBe(problems.length)
    })

    it("waits for the first analysis", () => {
        expect(documentGuidance(CaptureStep.Front, null).message).toBe("scanovateGuideLoading")
    })
})

describe("faceGuidance", () => {
    it("has an existing message for every status", () => {
        for (const status of Object.values(FaceStatus)) {
            const {message, tone} = faceGuidance(status)
            expect(scanovateEnglish[message as keyof typeof scanovateEnglish]).toBeTruthy()
            const good = status === FaceStatus.Ready || status === FaceStatus.HoldStill
            expect(tone).toBe(good ? Tone.Ok : Tone.Guide)
        }
    })
})

describe("videoGuidance", () => {
    it("guides the face first, then the ID", () => {
        expect(videoGuidance(FaceStatus.OffCenter, null, false).message).toBe(
            "scanovateGuideFaceOffCenter"
        )
        expect(videoGuidance(FaceStatus.Ready, DocumentStatus.NoDocument, false)).toEqual({
            message: "scanovateGuideHoldDocument",
            tone: Tone.Guide,
        })
        expect(videoGuidance(FaceStatus.Ready, DocumentStatus.TooFar, false).tone).toBe(Tone.Ok)
    })

    it("warns while recording when something leaves the view", () => {
        expect(videoGuidance(FaceStatus.Ready, DocumentStatus.Ready, true)).toEqual({
            message: "scanovateGuideRecording",
            tone: Tone.Ok,
        })
        expect(videoGuidance(FaceStatus.NoFace, DocumentStatus.Ready, true)).toEqual({
            message: "scanovateGuideKeepInView",
            tone: Tone.Guide,
        })
    })
})

describe("canTakePhoto", () => {
    it("allows the shutter only for a good frame", () => {
        expect(canTakePhoto(DocumentStatus.Ready)).toBe(true)
        expect(canTakePhoto(DocumentStatus.HoldStill)).toBe(true)
        expect(canTakePhoto(FaceStatus.HoldStill)).toBe(true)
        expect(canTakePhoto(DocumentStatus.Glare)).toBe(false)
        expect(canTakePhoto(FaceStatus.NoFace)).toBe(false)
        expect(canTakePhoto(null)).toBe(false)
    })
})
