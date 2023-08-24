// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Dialog, downloadUrl} from "@sequentech/ui-essentials"
import {Button, MenuItem, Select, TextField, Typography} from "@mui/material"
import {useTenantStore} from "./CustomMenu"
import {
    CreateReportMutation,
    FetchDocumentQuery,
    Sequent_Backend_Document,
    Sequent_Backend_Election_Event,
} from "../gql/graphql"
import {useMutation, useQuery} from "@apollo/client"
import {CREATE_REPORT} from "../queries/CreateReport"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import {CircularProgress} from "@mui/material"
import {FETCH_DOCUMENT} from "../queries/FetchDocument"
import {useRecordContext} from "react-admin"

const Vertical = styled(Box)`
    display: flex;
    flex-direction: column;
`

interface FetchDocumentVariables {
    tenantId: string
    electionEventId: string
    documentId: string
}

export const ReportDialog: React.FC = () => {
    const [createReport] = useMutation<CreateReportMutation>(CREATE_REPORT)
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [showTemplateDialog, setShowTemplateDialog] = useState(false)
    const [tenantId] = useTenantStore()
    const [template, setTemplate] = useState("")
    const [format, setFormat] = useState("PDF")
    const [showProgress, setShowProgress] = useState(false)
    const [documentId, setDocumentId] = useState<string | null>(null)
    let reportName = "report.pdf"

    const {loading, error, data} = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            tenantId: tenantId,
            electionEventId: record.id,
            documentId: documentId,
        },
    })

    useEffect(() => {
        if (!loading && !error && data?.fetchDocument?.url) {
            downloadUrl(data.fetchDocument.url, reportName)
        }
    }, [documentId, loading, error, data?.fetchDocument?.url, reportName])

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
                electionEventId: record.id,
                name: reportName,
                format: format,
            },
        })
        setShowProgress(false)
        if (errors) {
            console.log(`errors ${errors}`)
            return
        }
        setShowTemplateDialog(false)
        let report = data?.renderReport
        if (report) {
            setDocumentId(report.id)
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
