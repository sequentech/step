// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The voter's ballot, on its own.
 *
 * The entry point for `dist/ballot.js`, built by `yarn build:ballot`. It exists so a
 * consumer outside this repository can draw the real ballot without taking the whole
 * library: the package barrel reaches `TallyResults`, and through it `apexcharts`
 * and `@mui/x-data-grid`, and it reaches `@sequentech/ui-core`, whose barrel loads
 * the compiled encoder. None of that is needed to put a contest on screen.
 *
 * What a host must supply, and there are exactly two things:
 *
 * - a `BallotEngine`, the four compiled calls a ballot cannot draw itself without,
 *   from whichever build of `sequent-core` it already loads;
 * - a `BallotSelectionPort`, where the voter's marks live.
 *
 * Everything else is in the bundle.
 */

export {Question} from "./Question"
export type {IQuestionProps} from "./Question"
export {Answer} from "./Answer"
export {AnswersList} from "./AnswersList"
export {InvalidErrorsList} from "./InvalidErrorsList"

export {BallotEngineProvider, useBallotEngine} from "./engine"
export type {BallotEngine} from "./engine"
export {BallotSelectionProvider, useBallotSelection} from "./selection"
export type {BallotSelectionPort, ContestSelection, VoteChoice} from "./selection"

export type {IBallotStyle} from "./types"
export {IInvalidPlaintextErrorType} from "./errors"
export * from "./presentation"

// The row itself and its neighbours, which a preview needs directly: a ballot list
// screen draws `Candidate` without a `Question` around it.
export {default as Candidate} from "../components/Candidate/Candidate"
export {default as BlankAnswer} from "../components/BlankAnswer/BlankAnswer"
export {default as CandidatesList} from "../components/CandidatesList/CandidatesList"
export {default as WarnBox} from "../components/WarnBox/WarnBox"
// The card the ballot-list screen stacks, one per election. Exported so a preview can
// show that screen with the component a voter meets rather than a drawing of it.
export {default as SelectElection} from "../components/SelectElection/SelectElection"
export {theme} from "../services/theme"
