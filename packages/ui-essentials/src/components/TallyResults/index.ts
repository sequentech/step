// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export {TallyResultsView} from "./TallyResultsView"
export {
    ParticipationSummaryChart,
    CandidatesResultsCharts,
} from "./TallyResultsCharts"
export {TallyResultsCandidatesPlurality} from "./TallyResultsCandidatesPlurality"
export {TallyResultsCandidatesIRV} from "./TallyResultsCandidatesIRV"
export {winningPositionComparator} from "./utils"
export type {
    TallyCandidate,
    TallyParticipationSummary,
    TallyResultsViewModel,
    RunoffStatus,
    Round,
    CandidateReference,
    CandidateOutcome,
    CandidatesOutcomes,
    CandidatesStatus,
} from "./types"
export {ECandidateStatus} from "./types"
