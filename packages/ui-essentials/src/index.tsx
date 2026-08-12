// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
export {default as useTemplate} from "./useTemplate"
export {default as theme, adminTheme} from "./services/theme"
export {default as Header, HeaderErrorVariant} from "./components/Header/Header"
export {default as Dialog} from "./components/Dialog/Dialog"
export {default as CustomDropFile} from "./components/CustomDropFile/CustomDropFile"
export {default as DropFile} from "./components/DropFile/DropFile"
export {default as Footer} from "./components/Footer/Footer"
export {default as Icon} from "./components/Icon/Icon"
export {default as IconButton} from "./components/IconButton/IconButton"
export {default as InfoDataBox} from "./components/InfoDataBox/InfoDataBox"
export {default as LanguageMenu} from "./components/LanguageMenu/LanguageMenu"
export {default as LanguageSetter} from "./components/LanguageSetter/LanguageSetter"
export {default as LinkBehavior} from "./components/LinkBehavior/LinkBehavior"
export {default as LogoutButton} from "./components/LogoutButton/LogoutButton"
export {default as PageBanner} from "./components/PageBanner/PageBanner"
export {default as PageLimit} from "./components/PageLimit/PageLimit"
export {default as Version} from "./components/Version/Version"
export {default as VerticalBox} from "./components/VerticalBox/VerticalBox"
export {default as WarnBox, warnIdToClassName} from "./components/WarnBox/WarnBox"
export {
    default as BreadCrumbSteps,
    BreadCrumbStepsVariant,
} from "./components/BreadCrumbSteps/BreadCrumbSteps"
export {default as Candidate} from "./components/Candidate/Candidate"
export {getOrdinalSuffix} from "./components/Candidate/ordinalUtils"
export {default as BallotHash} from "./components/BallotHash/BallotHash"
export {default as QRCode} from "./components/QRCode/QRCode"
export {default as CandidatesList} from "./components/CandidatesList/CandidatesList"
export {default as SelectElection} from "./components/SelectElection/SelectElection"
export {default as Tree} from "./components/Tree/Tree"
export {NotFoundScreen} from "./components/NotFoundScreen"
export {default as BlankAnswer} from "./components/BlankAnswer/BlankAnswer"
export {default as CustomAutocompleteArrayInput} from "./components/CustomAutocompleteArrayInput/CustomAutocompleteArrayInput"
export {default as Loader} from "./components/Loader/Loader"
export {default as ExpandableText} from "./components/ExpandableText/ExpandableText"
export {ActionsContainer, StyledButton} from "./components/ConfirmationActions/ConfirmationActions"
export {PlaintextVoteContest} from "./components/PlaintextVoteContest/PlaintextVoteContest"
export type {PlaintextVoteContestProps} from "./components/PlaintextVoteContest/PlaintextVoteContest"
export {
    CandidateResults,
    CandidateResultsChart,
    default as ResultsAndParticipation,
    defaultResultsAndParticipationLabels,
    ParticipationSummary,
    ParticipationSummaryChart,
    ParticipationByChannel,
    PreferentialCandidateResults,
    sortCandidateResults,
    TALLY_RESULTS_PIE_HEIGHT,
    TALLY_RESULTS_PIE_PANEL_WIDTH,
} from "./components/TallyResults/TallyResults"
export {
    default as ResultsSelectorTabs,
    ResultsSelectorTabs as ResultsSelectorTabsComponent,
} from "./components/TallyResults/ResultsSelectorTabs"
export type {
    CandidateReference,
    CandidateResultRow,
    ParticipationChannelNames,
    ResultsAndParticipationLabelOverrides,
    ResultsAndParticipationLabels,
    ResultsAndParticipationProps,
    ResultsParticipationSummary,
    VotesByChannel,
    PreferentialProcessResults,
    PreferentialRound,
} from "./components/TallyResults/TallyResults"
export type {
    ResultsSelectorAreaOption,
    ResultsSelectorLabels,
    ResultsSelectorOption,
    ResultsSelectorSelection,
    ResultsSelectorTabsProps,
} from "./components/TallyResults/ResultsSelectorTabs"
export {default as ReviewChangesTable} from "./components/ReviewChangesTable/ReviewChangesTable"
export type {
    ReviewChangesRow,
    ReviewChangesTableProps,
} from "./components/ReviewChangesTable/ReviewChangesTable"

// The ballot a voter marks, and the port a host implements to hold their marks.
//
// One implementation, two consumers: the voting portal renders these over its
// redux store, and the Election Architect's preview renders the same components
// over local state. Previously the wizard drew byte-identical *copies* of the
// candidate row, kept honest by a parity checker, and could not draw a contest at
// all — so a preview could differ from the ballot, which for an election is not a
// cosmetic problem.
export {Question} from "./ballot/Question"
export type {IQuestionProps} from "./ballot/Question"
export {Answer} from "./ballot/Answer"
export {AnswersList} from "./ballot/AnswersList"
export {InvalidErrorsList} from "./ballot/InvalidErrorsList"
export {BallotSelectionProvider, useBallotSelection} from "./ballot/selection"
export type {
    BallotSelectionPort,
    ContestSelection,
    VoteChoice,
} from "./ballot/selection"
export type {IBallotStyle as IBallotStyleRow} from "./ballot/types"
export * from "./ballot/presentation"
export {IInvalidPlaintextErrorType} from "./ballot/errors"
