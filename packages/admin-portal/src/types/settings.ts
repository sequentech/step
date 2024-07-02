// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export type TVotingSetting = {
    [key: string]: boolean
}

export enum IVotingPortalCountdownPolicy {
    NO_COUNTDOWN = "no countdown",
    COUNTDOWN = "coutndown",
    COUNTDOWN_WITH_ALERT = "CountdownWithAlret"
}
