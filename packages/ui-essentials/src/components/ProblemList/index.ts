// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * How a problem with an imported file is shown, wherever it is imported.
 *
 * The component, the sentence it builds, and the types it takes. The sentences
 * themselves are in `ui-core`'s catalogue under `problems.*`.
 */
export {ProblemList} from "./ProblemList"
export type {ProblemListProps} from "./ProblemList"
export {problemKey, problemSentence} from "./sentence"
export type {ProblemSentence, ProblemTranslate} from "./sentence"
export {errorsOf, hasErrors, readProblems, warningsOf} from "./types"
export type {Problem, ProblemCode, ProblemReport, ProblemSeverity} from "./types"
