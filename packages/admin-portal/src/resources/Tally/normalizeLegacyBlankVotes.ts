// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

type BlankVoteValue = number | null

interface CurrentBlankVoteColumns {
    total_blank_votes?: BlankVoteValue
    total_blank_votes_percent?: BlankVoteValue
    explicit_blank_votes?: BlankVoteValue
    explicit_blank_votes_percent?: BlankVoteValue
    implicit_blank_votes?: BlankVoteValue
    implicit_blank_votes_percent?: BlankVoteValue
}

export interface LegacyBlankVoteColumns {
    blank_votes?: BlankVoteValue
    blank_votes_percent?: BlankVoteValue
}

const hasLegacyBlankVoteColumns = (row: LegacyBlankVoteColumns) =>
    Object.prototype.hasOwnProperty.call(row, "blank_votes") ||
    Object.prototype.hasOwnProperty.call(row, "blank_votes_percent")

const isMissing = (value: BlankVoteValue | undefined) => value === undefined || value === null

const shouldUseTotalAsImplicit = (
    total: BlankVoteValue | undefined,
    explicit: BlankVoteValue | undefined,
    implicit: BlankVoteValue | undefined,
    hasLegacyColumns: boolean
) => {
    const hasNoSplit = isMissing(explicit) && isMissing(implicit)
    if ((hasLegacyColumns || total !== undefined) && hasNoSplit) {
        return true
    }

    return (
        total !== undefined &&
        total !== null &&
        total > 0 &&
        (explicit ?? 0) + (implicit ?? 0) === 0
    )
}

export const normalizeLegacyBlankVotes = <T extends CurrentBlankVoteColumns>(
    row: T & LegacyBlankVoteColumns
) => {
    const hasLegacyColumns = hasLegacyBlankVoteColumns(row)
    const totalBlankVotes =
        row.total_blank_votes === undefined ? row.blank_votes : row.total_blank_votes
    const totalBlankVotesPercent =
        row.total_blank_votes_percent === undefined
            ? row.blank_votes_percent
            : row.total_blank_votes_percent

    const useTotalAsImplicit = shouldUseTotalAsImplicit(
        totalBlankVotes,
        row.explicit_blank_votes,
        row.implicit_blank_votes,
        hasLegacyColumns
    )
    const useTotalPercentAsImplicit = shouldUseTotalAsImplicit(
        totalBlankVotesPercent,
        row.explicit_blank_votes_percent,
        row.implicit_blank_votes_percent,
        hasLegacyColumns
    )

    if (!hasLegacyColumns && !useTotalAsImplicit && !useTotalPercentAsImplicit) {
        return row
    }

    return {
        ...row,
        total_blank_votes: totalBlankVotes,
        total_blank_votes_percent: totalBlankVotesPercent,
        explicit_blank_votes: useTotalAsImplicit ? 0 : row.explicit_blank_votes,
        explicit_blank_votes_percent: useTotalPercentAsImplicit
            ? 0
            : row.explicit_blank_votes_percent,
        implicit_blank_votes: useTotalAsImplicit ? totalBlankVotes : row.implicit_blank_votes,
        implicit_blank_votes_percent: useTotalPercentAsImplicit
            ? totalBlankVotesPercent
            : row.implicit_blank_votes_percent,
    }
}
