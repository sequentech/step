// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {LivenessEventType, livenessFrameUrl, livenessProblem, parseLivenessEvent} from "./liveness"
import {CaptureProblem} from "./types"

const URL_BASE = "https://liveness.example.com/liveness/?ui_theme=sequent_ui&token=t"

describe("livenessFrameUrl", () => {
    it("asks for the voter's language when Liveness Plus has it", () => {
        expect(livenessFrameUrl(URL_BASE, ["en", "es"], "es")).toBe(
            `${URL_BASE}&translation_language=es`
        )
        expect(livenessFrameUrl(URL_BASE, ["en", "es"], "es-PH")).toBe(
            `${URL_BASE}&translation_language=es`
        )
    })

    it("leaves the service default for other languages", () => {
        expect(livenessFrameUrl(URL_BASE, ["en", "es"], "tl")).toBe(URL_BASE)
    })
})

describe("parseLivenessEvent", () => {
    it("reads object and JSON string messages", () => {
        expect(parseLivenessEvent({type: "done", service_session_id: "s"})).toEqual({
            type: LivenessEventType.Done,
            errorCode: null,
        })
        expect(
            parseLivenessEvent(
                JSON.stringify({type: "error", error_code: "1003", error_message: "x"})
            )
        ).toEqual({type: LivenessEventType.Error, errorCode: 1003})
        expect(parseLivenessEvent({type: "init"})).toEqual({
            type: LivenessEventType.Init,
            errorCode: null,
        })
    })

    it("ignores anything else", () => {
        expect(parseLivenessEvent("not json")).toBeNull()
        expect(parseLivenessEvent({type: "other"})).toBeNull()
        expect(parseLivenessEvent(null)).toBeNull()
        expect(parseLivenessEvent(42)).toBeNull()
    })
})

describe("livenessProblem", () => {
    it("reuses the camera problems of our own capture", () => {
        expect(livenessProblem(1003)).toBe(CaptureProblem.CameraDenied)
        expect(livenessProblem(1013)).toBe(CaptureProblem.CameraDenied)
        expect(livenessProblem(1004)).toBe(CaptureProblem.CameraNotFound)
        expect(livenessProblem(1005)).toBe(CaptureProblem.CameraFailed)
    })

    it("treats closing the check as stopping", () => {
        expect(livenessProblem(1009)).toBeNull()
        expect(livenessProblem(1010)).toBeNull()
        expect(livenessProblem(1015)).toBeNull()
    })

    it("asks to start over once the one-time token is used up", () => {
        expect(livenessProblem(1014)).toBe(CaptureProblem.LivenessExpired)
    })

    it("offers to try again after other errors", () => {
        expect(livenessProblem(1006)).toBe(CaptureProblem.LivenessFailed)
        expect(livenessProblem(1011)).toBe(CaptureProblem.LivenessFailed)
        expect(livenessProblem(null)).toBe(CaptureProblem.LivenessFailed)
    })
})
