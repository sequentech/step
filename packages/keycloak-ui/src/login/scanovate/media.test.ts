// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {InsecureContextError, cameraProblem} from "./media"
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
