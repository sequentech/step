// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {ScanovateSide} from "../KcContext"
import {CameraFacing, CaptureProblem, CaptureStep} from "./types"

// Without a photo after this long, the step's tips open on their own.
export const STRUGGLE_MS = 20000

export enum Phase {
    Intro = "INTRO",
    Starting = "STARTING",
    Capturing = "CAPTURING",
    Problem = "PROBLEM",
    Checking = "CHECKING",
}

export type Captures = Partial<Record<CaptureStep, Blob>>

export interface CaptureState {
    phase: Phase
    steps: CaptureStep[]
    stepIndex: number
    captures: Captures
    problem: CaptureProblem | null
    helpOpen: boolean
    helpOpenedAutomatically: boolean
    stopConfirmOpen: boolean
    stepStartedAt: number | null
    justCaptured: CaptureStep | null
}

export type CaptureAction =
    | {type: "start"}
    | {type: "cameraReady"; at: number}
    | {type: "problem"; problem: CaptureProblem}
    | {type: "uploadFailed"; problem: CaptureProblem}
    | {type: "retry"}
    | {type: "captured"; step: CaptureStep; blob: Blob; at: number}
    | {type: "flashDone"}
    | {type: "tick"; at: number}
    | {type: "openHelp"}
    | {type: "closeHelp"}
    | {type: "requestStop"}
    | {type: "cancelStop"}
    | {type: "confirmStop"}

export function stepsFor(sides: ScanovateSide[]): CaptureStep[] {
    return [
        CaptureStep.Front,
        ...(sides.includes(ScanovateSide.Back) ? [CaptureStep.Back] : []),
        CaptureStep.Face,
        CaptureStep.Video,
    ]
}

// The ID is photographed with the rear camera, the voter with the front one.
export function facingFor(step: CaptureStep): CameraFacing {
    return step === CaptureStep.Front || step === CaptureStep.Back
        ? CameraFacing.Environment
        : CameraFacing.User
}

export function currentStep(state: CaptureState): CaptureStep {
    return state.steps[Math.min(state.stepIndex, state.steps.length - 1)]
}

export function initialCaptureState(steps: CaptureStep[]): CaptureState {
    return {
        phase: Phase.Intro,
        steps,
        stepIndex: 0,
        captures: {},
        problem: null,
        helpOpen: false,
        helpOpenedAutomatically: false,
        stopConfirmOpen: false,
        stepStartedAt: null,
        justCaptured: null,
    }
}

// Starts at a later step with earlier ones done; used by previews and tests.
export function stateAtStep(steps: CaptureStep[], step: CaptureStep, blob: Blob): CaptureState {
    const index = Math.max(steps.indexOf(step), 0)
    return {
        ...initialCaptureState(steps),
        phase: Phase.Starting,
        stepIndex: index,
        captures: Object.fromEntries(steps.slice(0, index).map((done) => [done, blob])),
    }
}

const stepReset = {
    helpOpen: false,
    helpOpenedAutomatically: false,
    stopConfirmOpen: false,
    stepStartedAt: null,
}

export function captureReducer(state: CaptureState, action: CaptureAction): CaptureState {
    switch (action.type) {
        case "start":
            if (state.phase !== Phase.Intro) return state
            return {
                ...initialCaptureState(state.steps),
                phase: Phase.Starting,
            }
        case "cameraReady":
            if (state.phase !== Phase.Starting) return state
            return {...state, phase: Phase.Capturing, stepStartedAt: action.at}
        case "problem":
            if (state.phase === Phase.Checking) return state
            return {...state, ...stepReset, phase: Phase.Problem, problem: action.problem}
        case "uploadFailed":
            if (state.phase !== Phase.Checking) return state
            return {...state, ...stepReset, phase: Phase.Problem, problem: action.problem}
        case "retry":
            if (state.phase !== Phase.Problem) return state
            // A failed upload sends the same captures again.
            if (state.problem === CaptureProblem.UploadFailed) {
                return {...state, phase: Phase.Checking, problem: null}
            }
            return {...state, phase: Phase.Starting, problem: null}
        case "captured": {
            if (state.phase !== Phase.Capturing || action.step !== currentStep(state)) {
                return state
            }
            const captures = {...state.captures, [action.step]: action.blob}
            const stepIndex = state.stepIndex + 1
            if (stepIndex >= state.steps.length) {
                return {
                    ...state,
                    ...stepReset,
                    phase: Phase.Checking,
                    captures,
                    stepIndex: state.steps.length - 1,
                    justCaptured: action.step,
                }
            }
            const sameCamera = facingFor(state.steps[stepIndex]) === facingFor(action.step)
            return {
                ...state,
                ...stepReset,
                phase: sameCamera ? Phase.Capturing : Phase.Starting,
                stepStartedAt: sameCamera ? action.at : null,
                captures,
                stepIndex,
                justCaptured: action.step,
            }
        }
        case "flashDone":
            return {...state, justCaptured: null}
        case "tick":
            if (
                state.phase !== Phase.Capturing ||
                state.helpOpenedAutomatically ||
                state.stepStartedAt === null ||
                action.at - state.stepStartedAt < STRUGGLE_MS
            ) {
                return state
            }
            return {...state, helpOpen: true, helpOpenedAutomatically: true}
        case "openHelp":
            return {...state, helpOpen: true}
        case "closeHelp":
            return {...state, helpOpen: false}
        case "requestStop":
            return {...state, stopConfirmOpen: true}
        case "cancelStop":
            return {...state, stopConfirmOpen: false}
        case "confirmStop":
            return initialCaptureState(state.steps)
    }
}
