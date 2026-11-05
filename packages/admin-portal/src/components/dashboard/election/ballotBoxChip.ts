// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {darken, type Theme} from "@mui/material/styles"

/**
 * An outlined warning chip whose text is readable: the theme's warning
 * orange has a 3.1:1 contrast on white, under the 4.5:1 that small text
 * needs, so the label uses a darker shade and the border keeps the orange.
 */
export const outlinedWarningChipSx = (theme: Theme) => ({
    color: darken(theme.palette.warning.main, 0.35),
    borderColor: theme.palette.warning.main,
})

/** A status chip whose label wraps on a phone instead of widening the table. */
export const wrappingChipSx = {
    "height": {xs: "auto", sm: 24},
    "minHeight": 24,
    "& .MuiChip-label": {
        whiteSpace: {xs: "normal", sm: "nowrap"},
        paddingTop: {xs: "2px", sm: 0},
        paddingBottom: {xs: "2px", sm: 0},
    },
} as const
