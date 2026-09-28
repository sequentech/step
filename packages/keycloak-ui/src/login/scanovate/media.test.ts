// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {
    InsecureContextError,
    VIDEO_BITS_PER_SECOND,
    VIDEO_MAX_BYTES,
    cameraProblem,
    pickMimeType,
    videoBitsPerSecond,
} from "./media"
import {CaptureProblem} from "./types"

describe("cameraProblem", () => {
    it("tells the voter what went wrong with the camera", () => {
        const problem = (name: string) => cameraProblem(new DOMException("", name))
        expect(problem("NotAllowedError")).toBe(CaptureProblem.CameraDenied)
        expect(problem("SecurityError")).toBe(CaptureProblem.CameraDenied)
        expect(problem("NotFoundError")).toBe(CaptureProblem.CameraNotFound)
        expect(problem("OverconstrainedError")).toBe(CaptureProblem.CameraNotFound)
        expect(problem("NotReadableError")).toBe(CaptureProblem.CameraInUse)
        expect(cameraProblem(new InsecureContextError())).toBe(CaptureProblem.InsecureContext)
    })

    it("falls back to a generic problem", () => {
        expect(cameraProblem(new DOMException("", "TypeError"))).toBe(CaptureProblem.CameraFailed)
        expect(cameraProblem("not an error")).toBe(CaptureProblem.CameraFailed)
        expect(cameraProblem(undefined)).toBe(CaptureProblem.CameraFailed)
    })
})

describe("pickMimeType", () => {
    it("prefers WebM and falls back to MP4 for Safari", () => {
        expect(pickMimeType(() => true)).toBe("video/webm;codecs=vp8")
        expect(pickMimeType((type) => type.startsWith("video/mp4"))).toBe("video/mp4;codecs=avc1")
        expect(pickMimeType((type) => type === "video/mp4")).toBe("video/mp4")
    })

    it("reports when the browser cannot record", () => {
        expect(pickMimeType(() => false)).toBeNull()
    })
})

describe("videoBitsPerSecond", () => {
    it("records at 1.5 Mbps for short videos", () => {
        expect(videoBitsPerSecond(5)).toBe(VIDEO_BITS_PER_SECOND)
    })

    it("keeps long videos within the upload limit", () => {
        for (const seconds of [15, 30, 60]) {
            expect((videoBitsPerSecond(seconds) * seconds) / 8).toBeLessThan(VIDEO_MAX_BYTES)
        }
    })

    it("does not divide by zero", () => {
        expect(videoBitsPerSecond(0)).toBe(VIDEO_BITS_PER_SECOND)
    })
})
