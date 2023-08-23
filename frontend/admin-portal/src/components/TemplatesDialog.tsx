// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Dialog, downloadUrl} from "@sequentech/ui-essentials"
import {Button, MenuItem, Select, TextField, Typography} from "@mui/material"
import {useTenantStore} from "./CustomMenu"
import {CreateReportMutation} from "../gql/graphql"
import {useMutation} from "@apollo/client"
import {CREATE_REPORT} from "../queries/CreateReport"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import {CircularProgress} from "@mui/material"

const Vertical = styled(Box)`
    display: flex;
    flex-direction: column;
`

export const TemplatesDialog: React.FC = () => {
    const [createReport] = useMutation<CreateReportMutation>(CREATE_REPORT)
    const [showTemplateDialog, setShowTemplateDialog] = useState(false)
    const [tenantId] = useTenantStore()
    const [template, setTemplate] = useState("")
    const [format, setFormat] = useState("PDF")
    const [showProgress, setShowProgress] = useState(false)

    const handleClose = async (value: boolean) => {
        if (!value) {
            setShowTemplateDialog(false)
            return
        }
        setShowProgress(true)
        const {data, errors} = await createReport({
            variables: {
                template: template,
                tenantId: tenantId,
                format: format,
            },
        })
        setShowProgress(false)
        if (errors) {
            console.log(`errors ${errors}`)
            return
        }
        setShowTemplateDialog(false)
        if (data?.renderTemplate?.url) {
            await downloadUrl(data.renderTemplate.url, "report.pdf")
        }
    }

    return (
        <>
            <Button onClick={() => setShowTemplateDialog(true)}>Use Template</Button>
            <Dialog
                handleClose={handleClose}
                open={showTemplateDialog}
                title="Template Dialog"
                ok="OK"
                cancel="Cancel"
                variant="info"
            >
                <Typography variant="body1">Generate PDF from template + variables</Typography>
                <Vertical>
                    <TextField
                        label="Template"
                        placeholder="Template"
                        multiline
                        maxRows={10}
                        value={template}
                        onChange={(event) => setTemplate(event.target.value)}
                    />

                    <Select
                        label="Format"
                        value={format}
                        onChange={(event) => setFormat(event.target.value)}
                    >
                        <MenuItem value={"PDF"}>PDF</MenuItem>
                        <MenuItem value={"TEXT"}>Text</MenuItem>
                    </Select>
                    {showProgress ? <CircularProgress /> : null}
                </Vertical>
            </Dialog>
        </>
    )
}
