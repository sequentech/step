// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum IVotingType {
    NON_PREFERENTIAL = "non-preferential",
    PREFERENTIAL = "preferential",
}

export enum ICountingAlgorithm {
    PLURALITY_AT_LARGE = "plurality-at-large",
    INSTANT_RUNOFF = "instant-runoff",
}
