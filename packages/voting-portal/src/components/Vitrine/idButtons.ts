// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The round buttons next to a Tracking ID, to copy it and to ask about it
// (design board BallotId): 34 px targets that tint on hover.

export const ID_BUTTON_SX = {
    "width": 34,
    "height": 34,
    "padding": 0,
    "paddingBottom": 0,
    "&:hover": {backgroundColor: "rgba(0, 0, 0, 0.04)", paddingBottom: 0},
} as const
