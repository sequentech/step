// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"

// Wraps a purely decorative Icon that used to be rendered as an IconButton with
// no click handler, which put a do-nothing button in the tab order. It
// reproduces the footprint MuiIconButton gets from the theme — a transparent 2px
// border, and `flex: 0 0 auto` so it does not shrink inside a flex row — so
// swapping the button for a plain icon leaves the layout unchanged.
const DecorativeIconBox = styled(Box)`
    display: inline-flex;
    align-items: center;
    flex: 0 0 auto;
    border: 2px solid transparent;
`

export default DecorativeIconBox
