// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * This package's WebAssembly-free surface.
 *
 * `index.tsx` re-exports `./services/wasm`, whose first statement is a value import
 * from `sequent-core` — the compiled encoder. That makes the barrel and the encoder
 * the same thing: importing `translate` to put a candidate's name on screen loads
 * four megabytes of WebAssembly, and a consumer that has its *own* build of the same
 * Rust ends up with two.
 *
 * That is exactly the Election Architect's position. Its ballot preview draws the
 * voter's real components out of `@sequentech/ui-essentials`, and it already loads
 * `sequent-election-config` — the same crate, compiled with the feature set the
 * configuration tools need. It needs this package's *pure* helpers and none of its
 * compiled ones, which it supplies through `BallotEngine` instead.
 *
 * So this file is that surface, written out rather than inferred: eight leaf modules
 * that between them import nothing compiled. It is not a new API and it copies
 * nothing — every line is a re-export, so there is one definition of each of these
 * and this file only offers a door to them that does not pass the encoder.
 *
 * **The rule for adding to this file:** the module you re-export from must not
 * import `sequent-core`, directly or transitively. `services/wasm` and anything that
 * reaches it do not belong here. Tree-shaking will not save you — a top-level value
 * import is a side effect a bundler must keep.
 */

// Turning a contest's candidates into the groups a ballot draws. Pure array work;
// its only `sequent-core` reference is an `import type`, which compiles away.
export {
    categorizeCandidates,
    getShuffledCategories,
    isCategoryListSelected,
    isChoiceSelected,
    shouldShowCategoryCandidateOnReview,
    showCategoryOnReview,
    sortCategoryEntries,
} from "./services/categoryService"
export type {CategoriesMap, ICategorizedCandidates, ICategory} from "./services/categoryService"

// What a candidate's presentation flags mean.
export {checkIsCategoryList, checkIsExplicitBlankVote} from "./services/candidatePresentation"

// Text.
export {translate} from "./services/translate"
export {stringToHtml} from "./services/stringToHtml"
export {normalizeWriteInText} from "./services/normalizeWriteInText"

// Small helpers, kept here rather than reached for from lodash.
export {isString, isUndefined} from "./utils/typechecks"
export {keyBy, splitList} from "./utils/array"

// The platform's value spaces. Enums, so these are runtime objects and not erasable.
export {
    CandidatesOrder,
    EBlankVotePolicy,
    ECandidatesIconCheckboxPolicy,
    ECandidatesSelectionPolicy,
    ECollapsibleLists,
    EEnableCheckableLists,
    EInvalidVotePolicy,
    EOverVotePolicy,
    EUnderVotePolicy,
} from "./types/ContestPresentation"
export type {IContestPresentation} from "./types/ContestPresentation"
export {EElectionEventContestEncryptionPolicy} from "./types/ElectionEventPresentation"
export type {IElectionEventPresentation} from "./types/ElectionEventPresentation"

// The documents themselves. Types only — erased, so they cost nothing at runtime.
export type {
    IBallotStyle,
    ICandidate,
    IContest,
    ICountingAlgorithm,
    IElection,
} from "./types/CoreTypes"
export type {IElectionPresentation} from "./types/ElectionPresentation"
export type {IAreaPresentation} from "./types/AreaPresentation"

// The encoder's own shapes, as types. `services/wasm` re-exports these from
// `sequent-core`; taken here with `import type` so no value import is emitted.
export type {
    BallotSelection,
    IDecodedVoteChoice,
    IDecodedVoteContest,
    IInvalidPlaintextError,
} from "./services/wasm"
