// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {CapturePart, captureParts} from "./form"
import {CaptureStep} from "./types"

const front = new Blob(["front"], {type: "image/jpeg"})
const back = new Blob(["back"], {type: "image/jpeg"})
const face = new Blob(["face"], {type: "image/jpeg"})
const holding = new Blob(["holding"], {type: "image/jpeg"})

describe("captureParts", () => {
    it("sends the ID and the photo holding it, not the face", () => {
        expect(
            captureParts({
                [CaptureStep.Front]: front,
                [CaptureStep.Back]: back,
                [CaptureStep.Face]: face,
                [CaptureStep.Video]: holding,
            })
        ).toEqual([
            {part: CapturePart.Front, blob: front},
            {part: CapturePart.Back, blob: back},
            {part: CapturePart.Holding, blob: holding},
        ])
    })

    it("leaves out the back when the document has none", () => {
        const parts = captureParts({
            [CaptureStep.Front]: front,
            [CaptureStep.Face]: face,
            [CaptureStep.Video]: holding,
        })
        expect(parts.map(({part}) => part)).toEqual([CapturePart.Front, CapturePart.Holding])
    })
})
