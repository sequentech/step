// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {StageLayout} from "./geometry"
import {InsecureContextError, cameraProblem, previewMirrored} from "./media"
import {CameraFacing, CaptureProblem, CaptureStep} from "./types"

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

    it("follows the camera the browser opened for the voter", () => {
        expect(
            previewMirrored(
                CaptureStep.Face,
                CameraFacing.Environment,
                streamFacing("user"),
                StageLayout.Phone
            )
        ).toBe(true)
        expect(
            previewMirrored(
                CaptureStep.Video,
                CameraFacing.User,
                streamFacing("environment"),
                StageLayout.Desktop
            )
        ).toBe(false)
    })

    it("mirrors an unreported camera on a desktop for the voter", () => {
        expect(
            previewMirrored(
                CaptureStep.Face,
                CameraFacing.User,
                streamFacing(),
                StageLayout.Desktop
            )
        ).toBe(true)
    })

    it("keeps the requested facing for an unreported camera on a phone", () => {
        expect(
            previewMirrored(CaptureStep.Face, CameraFacing.User, streamFacing(), StageLayout.Phone)
        ).toBe(true)
        expect(previewMirrored(CaptureStep.Video, CameraFacing.User, null, StageLayout.Phone)).toBe(
            true
        )
    })

    // Mirrored, the document and its text read backwards, whichever camera shows it.
    it("never mirrors the document", () => {
        for (const step of [CaptureStep.Front, CaptureStep.Back]) {
            for (const layout of [StageLayout.Phone, StageLayout.Desktop]) {
                for (const stream of [streamFacing("user"), streamFacing(), null]) {
                    expect(previewMirrored(step, CameraFacing.Environment, stream, layout)).toBe(
                        false
                    )
                }
            }
        }
    })
})
