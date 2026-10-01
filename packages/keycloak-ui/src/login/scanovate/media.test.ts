// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {StageLayout} from "./geometry"
import {InsecureContextError, cameraProblem, previewMirrored} from "./media"
import {CameraFacing} from "./types"
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

describe("previewMirrored", () => {
    const streamFacing = (facingMode?: string) =>
        ({
            getVideoTracks: () => [{getSettings: () => (facingMode ? {facingMode} : {})}],
        }) as unknown as MediaStream

    it("follows the camera the browser opened", () => {
        expect(
            previewMirrored(CameraFacing.Environment, streamFacing("user"), StageLayout.Phone)
        ).toBe(true)
        expect(
            previewMirrored(CameraFacing.User, streamFacing("environment"), StageLayout.Desktop)
        ).toBe(false)
    })

    // A laptop opens its webcam even when asked for the back camera, without saying which way it
    // faces: unmirrored, the document would move against the voter's hand.
    it("mirrors an unreported camera on a desktop", () => {
        expect(previewMirrored(CameraFacing.Environment, streamFacing(), StageLayout.Desktop)).toBe(
            true
        )
    })

    it("keeps the requested facing for an unreported camera on a phone", () => {
        expect(previewMirrored(CameraFacing.Environment, streamFacing(), StageLayout.Phone)).toBe(
            false
        )
        expect(previewMirrored(CameraFacing.User, streamFacing(), StageLayout.Phone)).toBe(true)
        expect(previewMirrored(CameraFacing.User, null, StageLayout.Phone)).toBe(true)
    })
})
