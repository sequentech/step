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
// The review screen's arrangement, so a preview shows that screen rather than a
// drawing of it. The portal renders the same component; see `ReviewLayout`.
export {SupportMaterialsLayout, SupportMaterialCard} from "./SupportMaterialsLayout"
export type {
    ISupportMaterialsLayoutProps,
    ISupportMaterialCardProps,
} from "./SupportMaterialsLayout"
export {ConfirmationLayout} from "./ConfirmationLayout"
export type {IConfirmationLayoutProps} from "./ConfirmationLayout"
export {StartLayout, START_WORDING_EN} from "./StartLayout"
export type {IStartLayoutProps, IStartWording} from "./StartLayout"
export {ReviewLayout} from "./ReviewLayout"
export type {IReviewLayoutProps} from "./ReviewLayout"
// The telephone call, which is a ballot too — one a voter is read rather than
// shown. Same argument as the three layouts above: the Election Architect lets
// somebody design a call flow, and the only honest preview of one is the emulator
// the Admin Portal already runs.
export {IvrCall, IvrPromptLine} from "./IvrCall"
export type {
    IIvrCallProps,
    IvrAction,
    IvrCallDriver,
    IvrCallStatus,
    IvrExpectedInput,
    IvrPrompt,
} from "./IvrCall"
export {forgetIvrEmulator, IvrEmulatorError, loadIvrEmulator} from "./ivrEmulator"
export type {
    ImportModule,
    IvrEmulatorApi,
    IvrEmulatorConfig,
    IvrEmulatorFailure,
} from "./ivrEmulator"
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

/*
 * The chrome a voter sees around the ballot: the header, the stepper and the footer.
 *
 * Exported here so the wizard's Ballot Preview can draw the portal's own frame rather
 * than a drawing of it. All three were already shared components in `ui-essentials`
 * rather than portal-local — the portal's `Stepper` is a 26-line shim that reads one
 * flag out of redux and hands the rest to `BreadCrumbSteps` — so this is an export and
 * not a lift. Nothing about them is redux-aware: `Header` takes its version, hash,
 * user and language list as props, and `Footer` takes none at all.
 *
 * A preview passes sample values for the version and the hash, and has to say so on
 * screen: those describe a deployment, and before an election is deployed there is no
 * true value for either.
 */
export {default as Header} from "../components/Header/Header"
export type {HeaderProps, IExpiryCountdown} from "../components/Header/Header"
export {default as Footer} from "../components/Footer/Footer"
export {default as BreadCrumbSteps} from "../components/BreadCrumbSteps/BreadCrumbSteps"
export {BreadCrumbStepsVariant} from "../components/BreadCrumbSteps/BreadCrumbSteps"

export {default as BlankAnswer} from "../components/BlankAnswer/BlankAnswer"
export {default as CandidatesList} from "../components/CandidatesList/CandidatesList"
export {default as WarnBox} from "../components/WarnBox/WarnBox"
// The card the ballot-list screen stacks, one per election. Exported so a preview can
// show that screen with the component a voter meets rather than a drawing of it.
export {default as SelectElection} from "../components/SelectElection/SelectElection"
export {theme} from "../services/theme"
