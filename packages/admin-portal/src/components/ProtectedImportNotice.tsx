// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Alert} from "@mui/material"
export function ProtectedImportNotice() {
    return (
        <Alert severity="info" sx={{width: "100%", mb: 2}}>
            Election data is read-only here. Change the source configuration and use its file
            import. The election board must approve the exact file before it is applied.
        </Alert>
    )
}
