// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {CapturePart, captureParts} from "./form"
import {CaptureStep, FaceCheck} from "./types"

const front = new Blob(["front"], {type: "image/jpeg"})
const back = new Blob(["back"], {type: "image/jpeg"})
const face = new Blob(["face"], {type: "image/jpeg"})
const video = new Blob(["video"], {type: "video/mp4"})

describe("captureParts", () => {
    it("sends the photos and the video with the photo face capture", () => {
        expect(
            captureParts(
                {
                    [CaptureStep.Front]: front,
                    [CaptureStep.Back]: back,
                    [CaptureStep.Face]: face,
                    [CaptureStep.Video]: video,
                },
                FaceCheck.Photo
            )
        ).toEqual([
            {part: CapturePart.Front, blob: front},
            {part: CapturePart.Back, blob: back},
            {part: CapturePart.Face, blob: face},
            {part: CapturePart.Video, blob: video},
        ])
    })

    it("leaves out the back when the document has none", () => {
        const parts = captureParts(
            {[CaptureStep.Front]: front, [CaptureStep.Face]: face, [CaptureStep.Video]: video},
            FaceCheck.Photo
        )
        expect(parts.map(({part}) => part)).toEqual([
            CapturePart.Front,
            CapturePart.Face,
            CapturePart.Video,
        ])
    })

    it("sends the ID and the photo holding it with liveness, not the face", () => {
        expect(
            captureParts(
                {
                    [CaptureStep.Front]: front,
                    [CaptureStep.Back]: back,
                    [CaptureStep.Face]: face,
                    [CaptureStep.Video]: face,
                },
                FaceCheck.Liveness
            )
        ).toEqual([
            {part: CapturePart.Front, blob: front},
            {part: CapturePart.Back, blob: back},
            {part: CapturePart.Holding, blob: face},
        ])
    })
})
