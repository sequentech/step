// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the ballot tracker's design boards share: the palette, the monospace
// face of the IDs, and the widths at which the layout narrows.

export const PALETTE = {
    navy: "#0F054C",
    text: "#191D23",
    muted: "#475569",
    quiet: "#64748B",
    faint: "#94A3B8",
    border: "#E7EAEE",
    tint: "#F7F9FE",
    track: "#EEF1F5",
    pending: "#CBD5E1",
    green: "#047857",
    greenTint: "#ECFDF5",
    redInk: "#B91C1C",
    redText: "#991B1B",
    redBorder: "#DC2626",
} as const

export const MONO = "ui-monospace,SFMono-Regular,Menlo,Consolas,monospace"

export const PHONE = "@media (max-width:750px)"
export const TABLET = "@media (max-width:970px)"
