// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    TallySheetVotingChannel,
    VotingStatusChannel,
    type KnownParticipationChannel,
    type ParticipationChannel,
} from "@sequentech/ui-core"

export type NumericValue = number | string | null | undefined
export type VotesByChannel = Readonly<Partial<Record<ParticipationChannel, NumericValue>>>
export type ParticipationChannelNames = Record<KnownParticipationChannel, string>

export interface ResultsParticipationSummary {
    id?: string | number
    eligibleCensus?: NumericValue
    totalAuditableVotes?: NumericValue
    totalAuditableVotesPercent?: NumericValue
    totalVotes?: NumericValue
    totalVotesPercent?: NumericValue
    totalValidVotes?: NumericValue
    totalValidVotesPercent?: NumericValue
    totalInvalidVotes?: NumericValue
    totalInvalidVotesPercent?: NumericValue
    explicitInvalidVotes?: NumericValue
    explicitInvalidVotesPercent?: NumericValue
    implicitInvalidVotes?: NumericValue
    implicitInvalidVotesPercent?: NumericValue
    blankVotes?: NumericValue
    blankVotesPercent?: NumericValue
    explicitBlankVotes?: NumericValue
    explicitBlankVotesPercent?: NumericValue
    implicitBlankVotes?: NumericValue
    implicitBlankVotesPercent?: NumericValue
    weight?: NumericValue
    votesByChannel?: VotesByChannel
}

export interface CandidateResultRow {
    id: string
    name: string
    castVotes?: NumericValue
    castVotesPercent?: NumericValue
    winningPosition?: NumericValue
}

export interface CandidateReference {
    id: string
    name: string
}

export interface CandidateOutcome {
    name: string
    wins: number
    transference: number
    percentage: number
}

export type CandidatesOutcomes = Record<string, CandidateOutcome>

export interface PreferentialRound {
    winner: CandidateReference | null
    candidates_wins: CandidatesOutcomes
    eliminated_candidates: CandidateReference[] | null
    active_candidates_count: number
    active_ballots_count: number
    exhausted_ballots_count: number
}

export interface PreferentialProcessResults {
    candidates_status?: Record<string, string>
    name_references: CandidateReference[]
    round_count: number
    rounds: PreferentialRound[]
    max_rounds: number
}

export interface ResultsAndParticipationLabels {
    participationSummary: string
    candidateResults: string
    total: string
    turnout: string
    eligibleCensus: string
    totalAuditableVotes: string
    totalVotesCounted: string
    totalValidVotes: string
    totalInvalidVotes: string
    explicitInvalidVotes: string
    implicitInvalidVotes: string
    blankVotes: string
    explicitBlankVotes: string
    implicitBlankVotes: string
    blankVotesChart: string
    weight: string
    options: string
    castVotes: string
    castVotesPercent: string
    winningPosition: string
    votesForCandidates: string
    invalidVotes: string
    nonVoters: string
    others: string
    candidate: string
    round: string
    winner: string
    eliminated: string
    empty: string
    previousRounds: string
    nextRounds: string
    participationByChannel: string
    channel: string
    channelNames: ParticipationChannelNames
}

export type ResultsAndParticipationLabelOverrides = Omit<
    Partial<ResultsAndParticipationLabels>,
    "channelNames"
> & {
    channelNames?: Partial<ParticipationChannelNames>
}

export interface ResultsAndParticipationProps {
    chartName: string
    summary?: ResultsParticipationSummary | null
    candidates: CandidateResultRow[]
    labels?: ResultsAndParticipationLabelOverrides
    showWeight?: boolean
    processResults?: PreferentialProcessResults | null
    preferential?: boolean
}

export const defaultResultsAndParticipationLabels: ResultsAndParticipationLabels = {
    participationSummary: "Participation Summary",
    candidateResults: "Candidate Results",
    total: "Total",
    turnout: "%",
    eligibleCensus: "Eligible Voters",
    totalAuditableVotes: "Total Auditable Votes",
    totalVotesCounted: "Total Votes Counted",
    totalValidVotes: "Total Valid Votes",
    totalInvalidVotes: "Total Invalid Votes",
    explicitInvalidVotes: "Explicitly Invalid Votes",
    implicitInvalidVotes: "Implicitly Invalid Votes",
    blankVotes: "Blank Votes",
    explicitBlankVotes: "Explicit Blank Votes",
    implicitBlankVotes: "Implicit Blank Votes",
    blankVotesChart: "Blank Votes",
    weight: "Weight",
    options: "Options",
    castVotes: "Number of Votes",
    castVotesPercent: "Percent of Votes",
    winningPosition: "Winning position",
    votesForCandidates: "Votes For Candidates",
    invalidVotes: "Invalid Votes",
    nonVoters: "Non Voters",
    others: "Others",
    candidate: "Candidate",
    round: "Round",
    winner: "Winner",
    eliminated: "Eliminated",
    empty: "No items",
    previousRounds: "Navigate to previous rounds",
    nextRounds: "Navigate to next rounds",
    participationByChannel: "Participation by channel",
    channel: "Channel",
    channelNames: {
        [VotingStatusChannel.Online]: "Online",
        [VotingStatusChannel.Kiosk]: "Kiosk",
        [VotingStatusChannel.EarlyVoting]: "Early voting",
        [VotingStatusChannel.Telephone]: "Telephone",
        [TallySheetVotingChannel.Paper]: "Paper",
        [TallySheetVotingChannel.Postal]: "Postal",
        [TallySheetVotingChannel.InPerson]: "In person",
    },
}
