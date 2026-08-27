// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"

// Text that is available to assistive technology but not painted on screen.
// Used to name controls whose visible label is an icon, is hidden at small
// breakpoints, or would be redundant for sighted users.
//
// Built on Box so callers can pick the element with `component`, e.g.
// `component="legend"` to name a fieldset without showing the legend.
//
// Note this is deliberately not `display: none` or `visibility: hidden`: those
// remove the element from the accessibility tree, which is the opposite of what
// is wanted here. The element stays rendered but is clipped to a single pixel.
const VisuallyHidden = styled(Box)<{component?: React.ElementType}>({
    border: 0,
    clip: "rect(0 0 0 0)",
    height: "1px",
    margin: "-1px",
    overflow: "hidden",
    padding: 0,
    position: "absolute",
    whiteSpace: "nowrap",
    width: "1px",
})

export default VisuallyHidden
