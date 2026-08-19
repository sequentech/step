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
export {StartLayout} from "./StartLayout"
export type {IStartLayoutProps} from "./StartLayout"
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
// The ballot's own breadcrumb: the portal's four steps, its wording, and what both
// become when an event has one election and no list to choose from. `BreadCrumbSteps`
// above is the general component; this is the one a voter meets, and the one the
// portal's `Stepper` is now a redux shim over.
export {BallotSteps} from "./BallotSteps"
export type {IBallotStepsProps} from "./BallotSteps"
// The ballot list screen's own arrangement, class names included — see
// `ElectionListLayout` for why the tree and not just the components is the contract.
export {ElectionListLayout} from "./ElectionListLayout"
export type {IElectionListLayoutProps} from "./ElectionListLayout"
// The ballot screen: the tree the contests sit in, and the row of buttons under them.
// Both were the portal's `VotingScreen`; `BallotActions` is why the preview's Back has
// a chevron and a Clear button beside it.
export {BallotScreenLayout} from "./BallotScreenLayout"
export type {IBallotScreenLayoutProps} from "./BallotScreenLayout"
export {BallotActions} from "./BallotActions"
// The rows under the review and confirmation screens, lifted the same way: the portal's
// `ReviewScreen` and `ConfirmationScreen` drew them inline, so a preview could only
// approximate them — and did, with plain buttons and no icons.
export {ReviewActions} from "./ReviewActions"
export type {IReviewActionsProps} from "./ReviewActions"
export {ConfirmationActions} from "./ConfirmationActions"
export type {IConfirmationActionsProps} from "./ConfirmationActions"
export type {IBallotActionsProps} from "./BallotActions"

export {default as BlankAnswer} from "../components/BlankAnswer/BlankAnswer"
export {default as CandidatesList} from "../components/CandidatesList/CandidatesList"
export {default as WarnBox} from "../components/WarnBox/WarnBox"
// The card the ballot-list screen stacks, one per election. Exported so a preview can
// show that screen with the component a voter meets rather than a drawing of it.
export {default as SelectElection} from "../components/SelectElection/SelectElection"
export {theme} from "../services/theme"
