// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {ScanovateSide} from "../KcContext"
import {
    Phase,
    STRUGGLE_MS,
    captureReducer,
    currentStep,
    facingFor,
    initialCaptureState,
    stateAtStep,
    stepsFor,
    type CaptureAction,
    type CaptureState,
} from "./controller"
import {CameraFacing, CaptureProblem, CaptureStep} from "./types"

const photo = new Blob(["jpeg"], {type: "image/jpeg"})
const BOTH = stepsFor([ScanovateSide.Front, ScanovateSide.Back])

function run(state: CaptureState, ...actions: CaptureAction[]): CaptureState {
    return actions.reduce(captureReducer, state)
}

function capturing(steps = BOTH): CaptureState {
    return run(initialCaptureState(steps), {type: "start"}, {type: "cameraReady", at: 0})
}

describe("stepsFor", () => {
    it("asks for the back only when the document has one", () => {
        expect(BOTH).toEqual([
            CaptureStep.Front,
            CaptureStep.Back,
            CaptureStep.Face,
            CaptureStep.Video,
        ])
        expect(stepsFor([ScanovateSide.Front])).toEqual([
            CaptureStep.Front,
            CaptureStep.Face,
            CaptureStep.Video,
        ])
    })

    it("always starts with the front, even without sides", () => {
        expect(stepsFor([])[0]).toBe(CaptureStep.Front)
    })
})

describe("facingFor", () => {
    it("uses the rear camera for the ID and the front camera for the voter", () => {
        expect(facingFor(CaptureStep.Front)).toBe(CameraFacing.Environment)
        expect(facingFor(CaptureStep.Back)).toBe(CameraFacing.Environment)
        expect(facingFor(CaptureStep.Face)).toBe(CameraFacing.User)
        expect(facingFor(CaptureStep.Video)).toBe(CameraFacing.User)
    })
})

describe("captureReducer", () => {
    it("opens the camera on start and captures once it is ready", () => {
        const starting = run(initialCaptureState(BOTH), {type: "start"})
        expect(starting.phase).toBe(Phase.Starting)
        const ready = run(starting, {type: "cameraReady", at: 5})
        expect(ready.phase).toBe(Phase.Capturing)
        expect(ready.stepStartedAt).toBe(5)
    })

    it("keeps the camera from the front to the back", () => {
        const state = run(capturing(), {
            type: "captured",
            step: CaptureStep.Front,
            blob: photo,
            at: 10,
        })
        expect(state.phase).toBe(Phase.Capturing)
        expect(currentStep(state)).toBe(CaptureStep.Back)
        expect(state.captures[CaptureStep.Front]).toBe(photo)
        expect(state.justCaptured).toBe(CaptureStep.Front)
        expect(state.stepStartedAt).toBe(10)
    })

    it("switches cameras between the back and the face", () => {
        const state = run(
            capturing(),
            {type: "captured", step: CaptureStep.Front, blob: photo, at: 1},
            {type: "captured", step: CaptureStep.Back, blob: photo, at: 2}
        )
        expect(state.phase).toBe(Phase.Starting)
        expect(currentStep(state)).toBe(CaptureStep.Face)
    })

    it("submits after the video", () => {
        const state = run(
            capturing(stepsFor([ScanovateSide.Front])),
            {type: "captured", step: CaptureStep.Front, blob: photo, at: 1},
            {type: "cameraReady", at: 2},
            {type: "captured", step: CaptureStep.Face, blob: photo, at: 3},
            {type: "captured", step: CaptureStep.Video, blob: photo, at: 4}
        )
        expect(state.phase).toBe(Phase.Checking)
        expect(Object.keys(state.captures)).toEqual([
            CaptureStep.Front,
            CaptureStep.Face,
            CaptureStep.Video,
        ])
        expect(currentStep(state)).toBe(CaptureStep.Video)
    })

    it("ignores a capture for another step or outside capturing", () => {
        const state = capturing()
        expect(
            captureReducer(state, {type: "captured", step: CaptureStep.Face, blob: photo, at: 1})
        ).toBe(state)
        const intro = initialCaptureState(BOTH)
        expect(
            captureReducer(intro, {type: "captured", step: CaptureStep.Front, blob: photo, at: 1})
        ).toBe(intro)
    })

    it("opens the tips once when the voter struggles", () => {
        const early = run(capturing(), {type: "tick", at: STRUGGLE_MS - 1})
        expect(early.helpOpen).toBe(false)
        const struggling = run(early, {type: "tick", at: STRUGGLE_MS})
        expect(struggling.helpOpen).toBe(true)
        const closed = run(struggling, {type: "closeHelp"}, {type: "tick", at: STRUGGLE_MS * 2})
        expect(closed.helpOpen).toBe(false)
    })

    it("gives every step its own struggle timer", () => {
        const state = run(
            capturing(),
            {type: "tick", at: STRUGGLE_MS},
            {type: "captured", step: CaptureStep.Front, blob: photo, at: STRUGGLE_MS + 1}
        )
        expect(state.helpOpen).toBe(false)
        expect(run(state, {type: "tick", at: STRUGGLE_MS * 2}).helpOpen).toBe(false)
        expect(run(state, {type: "tick", at: STRUGGLE_MS * 2 + 1}).helpOpen).toBe(true)
    })

    it("returns to the intro only after the voter confirms", () => {
        const asked = run(capturing(), {type: "requestStop"})
        expect(asked.stopConfirmOpen).toBe(true)
        const kept = run(asked, {type: "cancelStop"})
        expect(kept.phase).toBe(Phase.Capturing)
        const stopped = run(
            asked,
            {type: "captured", step: CaptureStep.Front, blob: photo, at: 1},
            {type: "confirmStop"}
        )
        expect(stopped).toEqual(initialCaptureState(BOTH))
    })

    it("shows a camera problem and retries the camera", () => {
        const problem = run(capturing(), {type: "problem", problem: CaptureProblem.CameraDenied})
        expect(problem.phase).toBe(Phase.Problem)
        expect(problem.problem).toBe(CaptureProblem.CameraDenied)
        const retried = run(problem, {type: "retry"})
        expect(retried.phase).toBe(Phase.Starting)
        expect(retried.problem).toBeNull()
    })

    it("does not interrupt the submission with a late problem", () => {
        const checking: CaptureState = {...initialCaptureState(BOTH), phase: Phase.Checking}
        expect(
            captureReducer(checking, {type: "problem", problem: CaptureProblem.CameraFailed})
        ).toBe(checking)
    })

    it("can start at a later step with the earlier ones done", () => {
        const state = stateAtStep(BOTH, CaptureStep.Face, photo)
        expect(state.phase).toBe(Phase.Starting)
        expect(currentStep(state)).toBe(CaptureStep.Face)
        expect(Object.keys(state.captures)).toEqual([CaptureStep.Front, CaptureStep.Back])
    })
})
