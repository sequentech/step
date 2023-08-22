// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { useState } from "react"
import {Dialog} from "@sequentech/ui-essentials"
import { Button, TextField, Typography } from "@mui/material"

export const TemplatesDialog: React.FC = () => {
    const [showTemplateDialog, setShowTemplateDialog] = useState(false)
    const [template, setTemplate] = useState("")
    const [variables, setVariables] = useState("")

    return <>
        <Button onClick={() => setShowTemplateDialog(true)}>Use Template</Button>
        <Dialog
            handleClose={() => setShowTemplateDialog(false)}
            open={showTemplateDialog}
            title="Template Dialog"
            ok="OK"
            cancel="Cancel"
            variant="info"
        >  
            <Typography variant="body1">
                Generate PDF from template + variables
            </Typography>
            <TextField
                label="Template"
                placeholder="Template"
                multiline
                maxRows={10}
                value={template}
                onChange={(event) => setTemplate(event.target.value)}
            />

            <TextField
                label="Variables"
                placeholder="Variables"
                multiline
                maxRows={10}
                value={variables}
                onChange={(event) => setVariables(event.target.value)}
            />
        </Dialog>
    </>
}