// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Checks the voter's face frames with Liveness Plus. It only tells the page
// whether to move on, keep capturing or show a problem: the verdict reaches
// Keycloak server to server and is never taken from the browser.
import type {MessageKey} from "../i18n"
import {
    LivenessApiError,
    LivenessRejection,
    LivenessStatus,
    type LivenessAbort,
    type LivenessApi,
} from "./livenessApi"
import {CaptureProblem} from "./types"

export enum LivenessOutcomeKind {
    Completed = "COMPLETED",
    Retry = "RETRY",
    Problem = "PROBLEM",
}

export type LivenessOutcome =
    | {kind: LivenessOutcomeKind.Completed}
    | {kind: LivenessOutcomeKind.Retry; guidance: MessageKey | null}
    | {kind: LivenessOutcomeKind.Problem; problem: CaptureProblem}

// Frame checks of Liveness Plus that the voter can fix.
const GUIDANCE: Partial<Record<number, MessageKey>> = {
    [LivenessStatus.TooManyFaces]: "scanovateGuideMultipleFaces",
    [LivenessStatus.FaceNotFound]: "scanovateGuidePlaceFace",
    [LivenessStatus.FaceTooSmall]: "scanovateGuideFaceTooFar",
    [LivenessStatus.FaceTooLarge]: "scanovateGuideFaceTooClose",
    [LivenessStatus.FaceTooCloseToBorder]: "scanovateGuideFaceOffCenter",
    [LivenessStatus.FaceNotCentered]: "scanovateGuideFaceOffCenter",
    [LivenessStatus.YawTooLarge]: "scanovateGuideTurnToCamera",
    [LivenessStatus.PitchTooLarge]: "scanovateGuideTurnToCamera",
    [LivenessStatus.RollTooLarge]: "scanovateGuideTurnToCamera",
    [LivenessStatus.FaceNotInFocus]: "scanovateGuideFaceBlurry",
    [LivenessStatus.BadlyLit]: "scanovateGuideTooDark",
    [LivenessStatus.SunglassesDetected]: "scanovateIntroTipCoverings",
    [LivenessStatus.MaskDetected]: "scanovateIntroTipCoverings",
}

export function frameGuidance(code: number): MessageKey | undefined {
    return GUIDANCE[code]
}

const COMPLETED: LivenessOutcome = {kind: LivenessOutcomeKind.Completed}
const failed = (problem = CaptureProblem.LivenessFailed): LivenessOutcome => ({
    kind: LivenessOutcomeKind.Problem,
    problem,
})

// One liveness session at a time, opened on the first frame and renewed once if it expired.
export class LivenessCheck {
    private session: string | null = null

    constructor(private readonly api: LivenessApi) {}

    async check(frame: Blob): Promise<LivenessOutcome> {
        try {
            for (let renewed = false; ; renewed = true) {
                this.session ??= await this.api.createSession()
                const code = await this.api.checkFrame(this.session, frame)
                if (code === LivenessStatus.SessionExpired && !renewed) {
                    this.session = null
                    continue
                }
                return await this.afterFrame(this.session, code)
            }
        } catch (error) {
            this.session = null
            const rejected =
                error instanceof LivenessApiError &&
                error.rejection === LivenessRejection.InvalidToken
            return failed(rejected ? CaptureProblem.LivenessExpired : CaptureProblem.LivenessFailed)
        }
    }

    abort(reason: LivenessAbort): void {
        if (this.session === null) return
        void this.api.abort(this.session, reason)
        this.session = null
    }

    private async afterFrame(session: string, code: number): Promise<LivenessOutcome> {
        if (code === LivenessStatus.Ok) return {kind: LivenessOutcomeKind.Retry, guidance: null}
        const guidance = frameGuidance(code)
        if (guidance !== undefined) return {kind: LivenessOutcomeKind.Retry, guidance}
        this.session = null
        if (code !== LivenessStatus.ScanCompleted) return failed()
        const completion = await this.api.completeSession(session)
        return completion === LivenessStatus.SessionCompleted ? COMPLETED : failed()
    }
}
