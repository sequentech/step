// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {afterEach, describe, expect, it} from "vitest"
import {CAPTURE_PARTS, captureFile, populateCaptureForm} from "./form"
import {CaptureStep} from "./types"

const jpeg = new Blob([new Uint8Array([0xff, 0xd8, 0xff])], {type: "image/jpeg"})
const mp4 = new Blob([new Uint8Array([0, 0, 0, 0x18])], {type: "video/mp4"})

function captureForm(): HTMLFormElement {
    const form = document.createElement("form")
    form.method = "post"
    form.enctype = "multipart/form-data"
    const action = document.createElement("input")
    action.type = "hidden"
    action.name = "action"
    action.value = "capture"
    form.append(action)
    for (const part of Object.values(CAPTURE_PARTS)) {
        const input = document.createElement("input")
        input.type = "file"
        input.name = part
        form.append(input)
    }
    document.body.append(form)
    return form
}

afterEach(() => {
    document.body.replaceChildren()
})

describe("captureFile", () => {
    it("names every part so the server treats it as a file", () => {
        expect(captureFile(CaptureStep.Front, jpeg).name).toBe("front.jpg")
        expect(captureFile(CaptureStep.Video, mp4).name).toBe("video.mp4")
        const webm = captureFile(CaptureStep.Video, new Blob([], {type: "video/webm;codecs=vp8"}))
        expect(webm.name).toBe("video.webm")
        expect(webm.type).toBe("video/webm")
    })
})

describe("populateCaptureForm", () => {
    it("sends the photos and the video as multipart files", () => {
        const form = captureForm()
        populateCaptureForm(form, {
            [CaptureStep.Front]: jpeg,
            [CaptureStep.Back]: jpeg,
            [CaptureStep.Face]: jpeg,
            [CaptureStep.Video]: mp4,
        })
        const data = new FormData(form)
        expect(data.get("action")).toBe("capture")
        for (const [part, name, type] of [
            ["front", "front.jpg", "image/jpeg"],
            ["back", "back.jpg", "image/jpeg"],
            ["face", "face.jpg", "image/jpeg"],
            ["video", "video.mp4", "video/mp4"],
        ]) {
            const file = data.get(part)
            expect(file).toBeInstanceOf(File)
            expect((file as File).name).toBe(name)
            expect((file as File).type).toBe(type)
        }
    })

    it("leaves out the back when the document has none", () => {
        const form = captureForm()
        populateCaptureForm(form, {
            [CaptureStep.Front]: jpeg,
            [CaptureStep.Face]: jpeg,
            [CaptureStep.Video]: mp4,
        })
        const data = new FormData(form)
        expect(data.has("back")).toBe(false)
        expect(data.has("front")).toBe(true)
    })
})
