// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The colours of a hub's pages (design boards HubHeader and HubAudit), and
// the ones they share with the voting portal's ballot tracker
// (voting-portal components/BallotLocator/trackerStyle.ts).

export const HUB_COLORS = {
    /** The hub's own colour: its mark, the short ID, its links. */
    key: "#0E4F55",
    /** The header's strip and the hub's name. */
    deep: "#0B3B3F",
    /** Text on the strip. */
    stripText: "#E6F4F1",
} as const

export const PALETTE = {
    navy: "#0F054C",
    text: "#191D23",
    muted: "#475569",
    quiet: "#64748B",
    border: "#E7EAEE",
    tint: "#F7F9FE",
    track: "#EEF1F5",
    green: "#047857",
    greenTint: "#ECFDF5",
    redInk: "#B91C1C",
    redText: "#991B1B",
    redBorder: "#DC2626",
} as const

export const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"

export const FONT = "'Helvetica Neue', Helvetica, Arial, sans-serif"

/** Where the page turns into its phone layout. */
export const PHONE = "@media (max-width: 750px)"
