// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum EElectionScreenStep {
    START = "start",
    VOTING = "voting",
    REVIEW = "review",
    SUCCESS = "success",
    AUDIT = "audit",
}

export interface IElectionScreenBackground {
    step: EElectionScreenStep
    image_url: string
}

export interface IElectionPresentation {
    screens_background?: Array<IElectionScreenBackground>
}
