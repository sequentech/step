// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {afterEach, describe, expect, it, vi} from "vitest"
import {LivenessCheck, LivenessOutcomeKind, frameGuidance} from "./liveness"
import {
    LivenessAbort,
    LivenessApiError,
    LivenessRejection,
    LivenessStatus,
    fetchLiveness,
    type LivenessApi,
} from "./livenessApi"
import {CaptureProblem} from "./types"

const frame = new Blob([new Uint8Array([0xff, 0xd8, 0xff])], {type: "image/jpeg"})
const SETTINGS = {url: "https://keycloak.example.com/biometric/liveness/", token: "t", caseId: "c"}

type Calls = {sessions: number; frames: string[]; completed: string[]; aborted: string[]}

// Answers every frame with the next code, repeating the last one.
function scriptedApi(
    codes: number[],
    options: {completion?: number; createSession?: () => Promise<string>} = {}
): LivenessApi & {calls: Calls} {
    const calls: Calls = {sessions: 0, frames: [], completed: [], aborted: []}
    let frameIndex = 0
    return {
        calls,
        createSession:
            options.createSession ??
            (async () => {
                calls.sessions += 1
                return `session-${calls.sessions}`
            }),
        checkFrame: async (session) => {
            calls.frames.push(session)
            const code = codes[Math.min(frameIndex, codes.length - 1)]
            frameIndex += 1
            return code
        },
        completeSession: async (session) => {
            calls.completed.push(session)
            return options.completion ?? LivenessStatus.SessionCompleted
        },
        abort: async (session) => {
            calls.aborted.push(session)
        },
    }
}

describe("frameGuidance", () => {
    it("turns the frame checks of Liveness Plus into face guidance", () => {
        expect(frameGuidance(LivenessStatus.TooManyFaces)).toBe("scanovateGuideMultipleFaces")
        expect(frameGuidance(LivenessStatus.FaceNotFound)).toBe("scanovateGuidePlaceFace")
        expect(frameGuidance(LivenessStatus.FaceTooSmall)).toBe("scanovateGuideFaceTooFar")
        expect(frameGuidance(LivenessStatus.FaceTooLarge)).toBe("scanovateGuideFaceTooClose")
        expect(frameGuidance(LivenessStatus.FaceNotCentered)).toBe("scanovateGuideFaceOffCenter")
        expect(frameGuidance(LivenessStatus.PitchTooLarge)).toBe("scanovateGuideTurnToCamera")
        expect(frameGuidance(LivenessStatus.FaceNotInFocus)).toBe("scanovateGuideFaceBlurry")
        expect(frameGuidance(LivenessStatus.BadlyLit)).toBe("scanovateGuideTooDark")
    })

    // A voter told to take off sunglasses they aren't wearing can't tell what to fix.
    it("tells sunglasses from face coverings", () => {
        expect(frameGuidance(LivenessStatus.SunglassesDetected)).toBe("scanovateGuideSunglasses")
        expect(frameGuidance(LivenessStatus.MaskDetected)).toBe("scanovateGuideFaceCovered")
    })

    it("has no guidance for other statuses", () => {
        expect(frameGuidance(LivenessStatus.ScanCompleted)).toBeUndefined()
        expect(frameGuidance(LivenessStatus.ServerError)).toBeUndefined()
    })
})

describe("LivenessCheck", () => {
    it("opens a session on the first frame and closes it once the scan completes", async () => {
        const api = scriptedApi([LivenessStatus.ScanCompleted])
        const outcome = await new LivenessCheck(api).check(frame)
        expect(outcome).toEqual({kind: LivenessOutcomeKind.Completed})
        expect(api.calls.frames).toEqual(["session-1"])
        expect(api.calls.completed).toEqual(["session-1"])
    })

    it("keeps the session while it asks for better frames", async () => {
        const api = scriptedApi([
            LivenessStatus.FaceTooSmall,
            LivenessStatus.Ok,
            LivenessStatus.ScanCompleted,
        ])
        const check = new LivenessCheck(api)
        expect(await check.check(frame)).toEqual({
            kind: LivenessOutcomeKind.Retry,
            guidance: "scanovateGuideFaceTooFar",
        })
        expect(await check.check(frame)).toEqual({kind: LivenessOutcomeKind.Retry, guidance: null})
        expect(await check.check(frame)).toEqual({kind: LivenessOutcomeKind.Completed})
        expect(api.calls.sessions).toBe(1)
    })

    it("opens a new session when the last one expired", async () => {
        const api = scriptedApi([LivenessStatus.SessionExpired, LivenessStatus.ScanCompleted])
        expect(await new LivenessCheck(api).check(frame)).toEqual({
            kind: LivenessOutcomeKind.Completed,
        })
        expect(api.calls.frames).toEqual(["session-1", "session-2"])
    })

    it("fails when the new session expires too", async () => {
        const api = scriptedApi([LivenessStatus.SessionExpired])
        expect(await new LivenessCheck(api).check(frame)).toEqual({
            kind: LivenessOutcomeKind.Problem,
            problem: CaptureProblem.LivenessFailed,
        })
    })

    it("fails when the session could not be completed", async () => {
        const api = scriptedApi([LivenessStatus.ScanCompleted], {
            completion: LivenessStatus.SessionExpired,
        })
        expect(await new LivenessCheck(api).check(frame)).toEqual({
            kind: LivenessOutcomeKind.Problem,
            problem: CaptureProblem.LivenessFailed,
        })
    })

    it("fails on server and session errors", async () => {
        for (const code of [
            LivenessStatus.CouldNotCheckPresentationAttack,
            LivenessStatus.TokenInvalid,
            LivenessStatus.SessionNotFound,
            LivenessStatus.ServerError,
        ]) {
            expect(await new LivenessCheck(scriptedApi([code])).check(frame)).toEqual({
                kind: LivenessOutcomeKind.Problem,
                problem: CaptureProblem.LivenessFailed,
            })
        }
    })

    it("asks to start over when Keycloak rejects the token", async () => {
        const api = scriptedApi([], {
            createSession: async () => {
                throw new LivenessApiError(LivenessRejection.InvalidToken)
            },
        })
        expect(await new LivenessCheck(api).check(frame)).toEqual({
            kind: LivenessOutcomeKind.Problem,
            problem: CaptureProblem.LivenessExpired,
        })
    })

    it("opens a new session after a problem", async () => {
        const api = scriptedApi([LivenessStatus.ServerError, LivenessStatus.ScanCompleted])
        const check = new LivenessCheck(api)
        await check.check(frame)
        await check.check(frame)
        expect(api.calls.frames).toEqual(["session-1", "session-2"])
    })

    it("ends an open session when the voter leaves", async () => {
        const api = scriptedApi([LivenessStatus.FaceNotFound])
        const check = new LivenessCheck(api)
        check.abort(LivenessAbort.LeftPage)
        expect(api.calls.aborted).toEqual([])
        await check.check(frame)
        check.abort(LivenessAbort.PressedClose)
        check.abort(LivenessAbort.PressedClose)
        expect(api.calls.aborted).toEqual(["session-1"])
    })
})

describe("fetchLiveness", () => {
    afterEach(() => {
        vi.unstubAllGlobals()
    })

    function stubFetch(...responses: Response[]) {
        const fetchMock = vi.fn()
        for (const response of responses) fetchMock.mockResolvedValueOnce(response)
        vi.stubGlobal("fetch", fetchMock)
        return fetchMock
    }

    const json = (body: unknown, status = 200) =>
        new Response(JSON.stringify(body), {status, headers: {"content-type": "application/json"}})

    it("creates a session with Keycloak's token and case", async () => {
        const fetchMock = stubFetch(json({session_token: "s", client_config: {}}))
        await expect(fetchLiveness(SETTINGS).createSession()).resolves.toBe("s")
        const [url, init] = fetchMock.mock.calls[0]
        expect(url).toBe("https://keycloak.example.com/biometric/liveness/create_session")
        expect(init.method).toBe("POST")
        expect(JSON.parse(init.body)).toEqual({token: "t", case_id: "c"})
    })

    it("tells a rejected token from other failures", async () => {
        stubFetch(json({detail: "invalid token"}, 401), json({success: false}, 500))
        const api = fetchLiveness(SETTINGS)
        await expect(api.createSession()).rejects.toEqual(
            new LivenessApiError(LivenessRejection.InvalidToken)
        )
        await expect(api.createSession()).rejects.toEqual(
            new LivenessApiError(LivenessRejection.Failed)
        )
    })

    it("fails when the network does", async () => {
        vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("offline")))
        await expect(fetchLiveness(SETTINGS).createSession()).rejects.toEqual(
            new LivenessApiError(LivenessRejection.Failed)
        )
    })

    it("sends frames as a plain JPEG with the session token", async () => {
        const fetchMock = stubFetch(json({status: {code: 30, message: "face too small"}}))
        await expect(fetchLiveness(SETTINGS).checkFrame("s", frame)).resolves.toBe(30)
        const [url, init] = fetchMock.mock.calls[0]
        expect(url).toBe("https://keycloak.example.com/biometric/liveness/check_liveness")
        expect(init.headers["service-session-token"]).toBe("s")
        const body = init.body as FormData
        expect((body.get("encrypted_file") as File).name).toBe("frame.jpg")
        expect(body.get("frame_id")).toMatch(/^[0-9a-f-]{36}$/)
        expect(Number.isNaN(Date.parse(body.get("timestamp") as string))).toBe(false)
    })

    it("completes the session", async () => {
        const fetchMock = stubFetch(json({status: {code: 1, message: "completed"}}))
        await expect(fetchLiveness(SETTINGS).completeSession("s")).resolves.toBe(1)
        const [url, init] = fetchMock.mock.calls[0]
        expect(url).toBe("https://keycloak.example.com/biometric/liveness/client_session_data")
        expect(init.headers["service-session-token"]).toBe("s")
    })

    it("fails on an answer without a status", async () => {
        stubFetch(json({}))
        await expect(fetchLiveness(SETTINGS).completeSession("s")).rejects.toEqual(
            new LivenessApiError(LivenessRejection.Failed)
        )
    })

    it("reports why the session ended, ignoring failures", async () => {
        const fetchMock = vi.fn().mockRejectedValue(new TypeError("offline"))
        vi.stubGlobal("fetch", fetchMock)
        await expect(
            fetchLiveness(SETTINGS).abort("s", LivenessAbort.LeftPage)
        ).resolves.toBeUndefined()
        const [url, init] = fetchMock.mock.calls[0]
        expect(url).toBe("https://keycloak.example.com/biometric/liveness/client_error")
        expect(JSON.parse(init.body)).toEqual({error: "user left page"})
        expect(init.keepalive).toBe(true)
    })
})
