// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {Box, MenuItem} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ETemplateType} from "@/types/templates"
import {useMutation} from "@apollo/client"
import {GENERATE_TEMPLATE} from "@/queries/GenerateTemplate"
import {IPermissions} from "@/types/keycloak"
import {GenerateTemplateMutation} from "@/gql/graphql"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "../Widget"

interface GenerateReportProps {
    electionEventId: string
    electionId: string | null
    tallySessionId: string
    reportType: ETemplateType
    handleClose: () => void
}

export const GenerateReport: React.FC<GenerateReportProps> = ({
    electionEventId,
    electionId,
    tallySessionId,
    reportType,
    handleClose,
}) => {
    const {t} = useTranslation()
    const [documentId, setDocumentId] = useState<string | null>(null)

    const [generateTemplate] = useMutation<GenerateTemplateMutation>(GENERATE_TEMPLATE, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.REPORT_READ,
            },
        },
    })
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const onClick = async (e: React.MouseEvent<HTMLElement>) => {
        e.preventDefault()
        e.stopPropagation()
        setDocumentId(null)
        handleClose()
        const currWidget: WidgetProps = addWidget(ETasksExecution.GENERATE_REPORT, undefined)
        try {
            let {data} = await generateTemplate({
                variables: {
                    tallySessionId: tallySessionId,
                    electionId: electionId,
                    electionEventId: electionEventId,
                    type: "BallotImages",
                },
            })
            let response = data?.generate_template
            let taskId = response?.task_execution?.id
            let generatedDocumentId = response?.document_id

            if (!generatedDocumentId) {
                updateWidgetFail(currWidget.identifier)
                setDocumentId(null)
                return
            }
            setDocumentId(generatedDocumentId)
            setWidgetTaskId(currWidget.identifier, taskId)
        } catch (e) {
            updateWidgetFail(currWidget.identifier)
        }
    }

    return (
        <MenuItem onClick={onClick} key={reportType}>
            <Box
                sx={{
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                }}
            >
                <span>
                    {t("tally.generateReport", {
                        name: t(`template.type.${reportType}`),
                    })}
                    {documentId ? (
                        <DownloadDocument
                            documentId={documentId}
                            electionEventId={electionEventId}
                            fileName={null}
                            onDownload={() => {
                                setDocumentId(null)
                            }}
                        />
                    ) : null}
                </span>
            </Box>
        </MenuItem>
    )
}
