// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {
    acceptsMessage,
    captureReducer,
    initialCaptureState,
    parseScanovateMessage,
    problemFor,
    type CaptureState,
} from "./capture"
import {CaptureProblem, CaptureStage, ScanovateErrorCode, ScanovateMessageKind} from "./types"

const done = {
    type: "done",
    service_session_id: "session",
    message: "process completed successfully",
    result_path: "/result",
    session_data: {},
}

const error = (code: unknown) => ({
    type: "error",
    service_session_id: "session",
    message: "error",
    error_code: code,
    error_message: "message",
    result_path: "/result",
})

describe("parseScanovateMessage", () => {
    it("reads done and error messages", () => {
        expect(parseScanovateMessage(done)).toEqual({kind: ScanovateMessageKind.Done})
        expect(parseScanovateMessage(error(1003))).toEqual({
            kind: ScanovateMessageKind.Error,
            code: 1003,
        })
    })

    it("reads error codes sent as text and errors without a code", () => {
        expect(parseScanovateMessage(error("1014"))).toEqual({
            kind: ScanovateMessageKind.Error,
            code: 1014,
        })
        expect(parseScanovateMessage(error(undefined))).toEqual({
            kind: ScanovateMessageKind.Error,
            code: null,
        })
        expect(parseScanovateMessage(error("broken"))).toEqual({
            kind: ScanovateMessageKind.Error,
            code: null,
        })
    })

    it("ignores the loading messages, init and anything else", () => {
        for (const data of [
            "js-load-success",
            "js-load-error",
            "css-load-success",
            "css-load-error",
            "iframe-loaded",
            "ui-loaded",
            "hide-iframe",
            {type: "init", service_session_id: "session", message: "init"},
            {type: "unknown"},
            null,
            undefined,
            42,
            [],
        ]) {
            expect(parseScanovateMessage(data)).toBeNull()
        }
    })
})

describe("problemFor", () => {
    it("maps camera codes to the camera help", () => {
        for (const code of [
            ScanovateErrorCode.CameraPermissionBlocked,
            ScanovateErrorCode.CameraNotFound,
            ScanovateErrorCode.CouldNotOpenCamera,
            ScanovateErrorCode.CameraPermissionDismissed,
        ]) {
            expect(problemFor(code)).toBe(CaptureProblem.Camera)
        }
    })

    it("maps expired sessions, invalid tokens and closing", () => {
        expect(problemFor(ScanovateErrorCode.SessionExpired)).toBe(CaptureProblem.Expired)
        expect(problemFor(ScanovateErrorCode.InvalidToken)).toBe(CaptureProblem.InvalidToken)
        expect(problemFor(ScanovateErrorCode.UserPressedClose)).toBe(CaptureProblem.Closed)
    })

    it("falls back to a generic problem", () => {
        expect(problemFor(ScanovateErrorCode.ServerError)).toBe(CaptureProblem.Generic)
        expect(problemFor(9999)).toBe(CaptureProblem.Generic)
        expect(problemFor(null)).toBe(CaptureProblem.Generic)
    })
})

describe("captureReducer", () => {
    const start = initialCaptureState()

    it("starts on the document scan", () => {
        expect(start).toEqual({stage: CaptureStage.Document, problem: null, attempt: 0})
    })

    it("moves from the document to the face and then to checking", () => {
        const face = captureReducer(start, {type: "done"})
        expect(face.stage).toBe(CaptureStage.Face)
        const checking = captureReducer(face, {type: "done"})
        expect(checking.stage).toBe(CaptureStage.Checking)
        expect(captureReducer(checking, {type: "done"})).toBe(checking)
    })

    it("shows the problem of an error and stays on the step", () => {
        const failed = captureReducer(start, {
            type: "error",
            code: ScanovateErrorCode.CameraNotFound,
        })
        expect(failed).toEqual({
            stage: CaptureStage.Document,
            problem: CaptureProblem.Camera,
            attempt: 0,
        })
    })

    it("ignores messages while a problem is shown or once checking", () => {
        const failed = captureReducer(start, {type: "error", code: null})
        expect(captureReducer(failed, {type: "done"})).toBe(failed)
        expect(captureReducer(failed, {type: "error", code: 1003})).toBe(failed)
        const checking: CaptureState = {...start, stage: CaptureStage.Checking}
        expect(captureReducer(checking, {type: "error", code: 1003})).toBe(checking)
    })

    it("mounts the same step again on try again", () => {
        const face = captureReducer(start, {type: "done"})
        const failed = captureReducer(face, {
            type: "error",
            code: ScanovateErrorCode.UserPressedClose,
        })
        expect(captureReducer(failed, {type: "tryAgain"})).toEqual({
            stage: CaptureStage.Face,
            problem: null,
            attempt: 1,
        })
    })
})

describe("acceptsMessage", () => {
    const frame = {} as Window
    const other = {} as Window
    const page = "https://keycloak.example"
    const url = "https://keycloak.example/biometric/ocr/?token=t&case_id=c"

    it("accepts the current frame on the page's origin or the frame's", () => {
        expect(acceptsMessage({source: frame, origin: page}, frame, page, url)).toBe(true)
        expect(
            acceptsMessage(
                {source: frame, origin: "https://scan.example"},
                frame,
                page,
                "https://scan.example/ocr/"
            )
        ).toBe(true)
    })

    it("resolves relative frame URLs against the page", () => {
        expect(
            acceptsMessage({source: frame, origin: page}, frame, page, "/biometric/liveness/")
        ).toBe(true)
    })

    it("rejects other windows, other origins and missing frames", () => {
        expect(acceptsMessage({source: other, origin: page}, frame, page, url)).toBe(false)
        expect(
            acceptsMessage({source: frame, origin: "https://evil.example"}, frame, page, url)
        ).toBe(false)
        expect(acceptsMessage({source: null, origin: page}, null, page, url)).toBe(false)
        expect(acceptsMessage({source: frame, origin: page}, frame, page, "http://[")).toBe(true)
        expect(
            acceptsMessage({source: frame, origin: "https://scan.example"}, frame, page, "http://[")
        ).toBe(false)
    })
})
