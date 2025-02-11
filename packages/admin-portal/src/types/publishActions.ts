// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum EPublishActions {
    PENDING_START_VOTING = "pendingStartVotingPeriod",
    PENDING_PAUSE_VOTING = "pendingPauseVotingPeriod",
    PENDING_STOP_VOTING = "pendingStopVotingPeriod",
    PENDING_PUBLISH_ACTION = "pendingPublishAction",
    PENDING_STOP_KIOSK_ACTION = "pendingStopKioskAction",
    PENDING_GENERATE_INITIALIZATION_REPORT = "pendingGenerateInitializationReport",
}
