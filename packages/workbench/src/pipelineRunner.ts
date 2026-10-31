// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {runBallotPipeline, type PipelineRun} from "voting-portal/src/preview/ballotPipeline"

/** Where a pipeline's results come from; only the WASM runner exercises sequent-core. */
export enum ComputationKind {
    WASM = "wasm",
    MOCK = "mock",
}

export interface PipelineRunner {
    kind: ComputationKind
    run: PipelineRun
}

export const WASM_RUNNER: PipelineRunner = {kind: ComputationKind.WASM, run: runBallotPipeline}
