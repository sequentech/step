// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createTheme} from "@mui/material"

/** Plain, dense controls; the portal preview keeps the production theme. */
export const workbenchTheme = createTheme({
    typography: {fontSize: 13},
    components: {
        MuiButton: {defaultProps: {size: "small", disableElevation: true}},
        MuiTextField: {defaultProps: {size: "small"}},
    },
})
