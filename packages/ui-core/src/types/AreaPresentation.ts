// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum EEarlyVotingPolicy {
    ALLOW_EARLY_VOTING = "allow_early_voting",
    NO_EARLY_VOTING = "no_early_voting",
}

export interface IAreaPresentation {
    allow_early_voting: EEarlyVotingPolicy
}
