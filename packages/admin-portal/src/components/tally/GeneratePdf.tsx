// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {Box, MenuItem} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ETemplateType} from "@/types/templates"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "../Widget"
import {RENDER_DOCUMENT_PDF} from "@/queries/RenderDocumentPDF"
import {RenderDocumentPdfMutation} from "@/gql/graphql"
import {EExportFormat, IResultDocuments} from "@/types/results"
import {DownloadDocument} from "@/resources/User/DownloadDocument"

interface GenerateReportProps {
    documents: IResultDocuments
    name: string
    electionEventId: string
    tallySessionId: string
    handleClose: () => void
}

export const GeneratePDF: React.FC<GenerateReportProps> = ({
    documents,
    name,
    electionEventId,
    tallySessionId,
    handleClose,
}) => {
    const {t} = useTranslation()
    const [documentId, setDocumentId] = useState<string | null>(null)
    const [outputDocumentId, setOutputDocumentId] = useState<string | null>(null)

    const [generateDocumentPdf] = useMutation<RenderDocumentPdfMutation>(RENDER_DOCUMENT_PDF, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.REPORT_READ,
            },
        },
    })

    useEffect(() => {
        if (!documentId && documents?.[EExportFormat.HTML]) {
            setDocumentId(documents?.[EExportFormat.HTML])
        }
    }, [documents?.[EExportFormat.PDF]])

    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const onClick = async (e: React.MouseEvent<HTMLElement>) => {
        e.preventDefault()
        e.stopPropagation()
        handleClose()
        if (!documentId) {
            return
        }
        setOutputDocumentId(null)
        const currWidget: WidgetProps = addWidget(ETasksExecution.RENDER_DOCUMENT_PDF, true)
        try {
            let {data} = await generateDocumentPdf({
                variables: {
                    documentId,
                    tallySessionId,
                    electionEventId,
                },
            })
            let response = data?.render_document_pdf
            let taskId = response?.task_execution?.id
            let generatedDocumentId = response?.document_id

            if (!generatedDocumentId) {
                updateWidgetFail(currWidget.identifier)
                setDocumentId(null)
                return
            }
            setOutputDocumentId(generatedDocumentId)
            setWidgetTaskId(currWidget.identifier, taskId)
        } catch (e) {
            updateWidgetFail(currWidget.identifier)
        }
    }

    if (!documentId) {
        return null
    }

    return (
        <MenuItem
            onClick={onClick}
            className="generate-pdf-item"
            key={EExportFormat.PDF + "-" + documentId}
        >
            <Box
                sx={{
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                }}
            >
                <span>
                    {t("common.label.exportFormat", {
                        item: name,
                        format: EExportFormat.PDF.toUpperCase(),
                    })}
                    {outputDocumentId ? (
                        <DownloadDocument
                            documentId={outputDocumentId}
                            electionEventId={electionEventId}
                            fileName={null}
                            onDownload={() => {
                                setOutputDocumentId(null)
                            }}
                        />
                    ) : null}
                </span>
            </Box>
        </MenuItem>
    )
}
