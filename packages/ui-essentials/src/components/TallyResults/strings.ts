// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Lifted from admin-portal/src/translations/en.ts (tally.* subkeys).
// See packages/workbench/LIFTING-TALLY.md adaptation L1: this module substitutes
// react-i18next so the lifted components can stay near-verbatim copies of
// admin-portal source.

const TALLY_STRINGS: Record<string, string> = {
    "tally.chart.votesForCandidates": "Votes For Candidates",
    "tally.chart.blankVotes": "Blank Votes",
    "tally.chart.invalidVotes": "Invalid Votes",
    "tally.chart.nonVoters": "Non Voters",
    "tally.table.candidates": "Candidate Results",
    "tally.table.options": "Options",
    "tally.table.cast_votes": "Number of Votes",
    "tally.table.cast_votes_percent": "Percent of Votes",
    "tally.table.winning_position": "Winning position",
    "tally.table.preferential.candidate": "Candidate",
    "tally.table.preferential.round": "Round",
    "tally.table.preferential.winner": "Winner",
    "tally.table.preferential.eliminated": "Eliminated",
}

/** Drop-in replacement for `useTranslation().t` for tally.* keys. */
export const t = (key: string): string => TALLY_STRINGS[key] ?? key
