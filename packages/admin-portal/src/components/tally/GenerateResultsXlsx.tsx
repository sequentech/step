// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Box, MenuItem} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useMutation, useLazyQuery} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {ExportTallyResultsMutation, GetTallySessionExecutionQuery} from "@/gql/graphql"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "../Widget"
import {EXPORT_TALLY_RESULTS} from "@/queries/ExportTallyResults"
import {GET_TALLY_SESSION_EXECUTION} from "@/queries/GetTallySessionExecution"
import {DownloadExistingDocument} from "@/resources/User/DownloadExistingDocument"

interface GenerateResultsXlsxProps {
    eventName: string
    electionEventId: string
    tallySessionId: string
    tenantId: string
    resultsEventId: string
    handleClose: () => void
}

interface DownloadDocumentData {
    id: string
    isTask: boolean
}

export const GenerateResultsXlsx: React.FC<GenerateResultsXlsxProps> = ({
    eventName,
    electionEventId,
    tallySessionId,
    tenantId,
    resultsEventId,
    handleClose,
}) => {
    const {t} = useTranslation()
    const [document, setDocument] = useState<DownloadDocumentData | null>(null)
    const [exportTallyResults] = useMutation<ExportTallyResultsMutation>(EXPORT_TALLY_RESULTS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TALLY_RESULTS_READ,
            },
        },
    })

    const [getTallySessionExecution] = useLazyQuery<GetTallySessionExecutionQuery>(
        GET_TALLY_SESSION_EXECUTION,
        {
            fetchPolicy: "network-only",
        }
    )

    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const onClick = async (e: React.MouseEvent<HTMLElement>) => {
        e.preventDefault()
        e.stopPropagation()
        setDocument(null)
        setTimeout(() => handleClose(), 0)
        let {data} = await getTallySessionExecution({
            variables: {
                tallySessionId: tallySessionId,
                tenantId: tenantId,
                resultsEventId: resultsEventId,
            },
        })
        let documents = data?.sequent_backend_tally_session_execution?.[0]?.documents
        let resultsXlsxDocument = documents?.xlsx ?? null

        if (resultsXlsxDocument) {
            setDocument({id: resultsXlsxDocument, isTask: false})
        } else {
            const currWidget: WidgetProps = addWidget(ETasksExecution.EXPORT_TALLY_RESULTS_XLSX)
            try {
                let {data} = await exportTallyResults({
                    variables: {
                        electionEventId: electionEventId,
                        tallySessionId: tallySessionId,
                    },
                })
                let response = data?.export_tally_results
                let taskId = response?.task_execution?.id
                let generatedDocumentId = response?.document_id

                if (!generatedDocumentId) {
                    updateWidgetFail(currWidget.identifier)
                    setDocument(null)
                    return
                }
                setDocument({id: generatedDocumentId, isTask: true})
                setWidgetTaskId(currWidget.identifier, taskId)
            } catch (e) {
                updateWidgetFail(currWidget.identifier)
            }
        }
    }

    return (
        <MenuItem onClick={onClick}>
            <Box
                sx={{
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                }}
            >
                <span>
                    <span title={"XLSX"}>
                        {t("common.label.exportFormat", {
                            item: eventName,
                            format: "XLSX",
                        })}
                    </span>
                    {document ? (
                        document.isTask ? (
                            <DownloadDocument
                                documentId={document.id}
                                electionEventId={electionEventId}
                                fileName={null}
                                onDownload={() => {
                                    setDocument(null)
                                }}
                            />
                        ) : (
                            <DownloadExistingDocument
                                documentId={document.id}
                                electionEventId={electionEventId}
                                fileName={null}
                                onDownload={() => {
                                    setDocument(null)
                                }}
                            />
                        )
                    ) : null}
                </span>
            </Box>
        </MenuItem>
    )
}
