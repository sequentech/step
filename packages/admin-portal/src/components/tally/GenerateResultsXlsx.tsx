// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Box, MenuItem} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useMutation, useLazyQuery} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {ExportTallyResultsMutation, GetLastTallySessionExecutionQuery} from "@/gql/graphql"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "../Widget"
import {EXPORT_TALLY_RESULTS} from "@/queries/ExportTallyResults"
import {GET_LAST_TALLY_SESSION_EXECUTION} from "@/queries/GetLastTallySessionExecution"

interface GenerateResultsXlsxProps {
    electionEventId: string
    tallySessionId: string
    tenantId: string
    handleClose: () => void
}

export const GenerateResultsXlsx: React.FC<GenerateResultsXlsxProps> = ({
    electionEventId,
    tallySessionId,
    tenantId,
    handleClose,
}) => {
    const {t} = useTranslation()
    const [documentId, setDocumentId] = useState<string | null>(null)

    const [exportTallyResults] = useMutation<ExportTallyResultsMutation>(EXPORT_TALLY_RESULTS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TALLY_RESULTS_READ,
            },
        },
    })

    const [getLastTallySessionExecution] = useLazyQuery<GetLastTallySessionExecutionQuery>(
        GET_LAST_TALLY_SESSION_EXECUTION
    )

    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const onClick = async (e: React.MouseEvent<HTMLElement>) => {
        e.preventDefault()
        e.stopPropagation()
        setTimeout(() => handleClose(), 0)
        setDocumentId(null)
        let tallySessionExecution = await getLastTallySessionExecution({
            variables: {
                tallySessionId: tallySessionId,
                tenantId: tenantId,
            },
        })

        let documents =
            tallySessionExecution.data?.sequent_backend_tally_session_execution?.[0]?.documents
        let resultsXlsxDocument = documents?.xlsx ?? null

        if (resultsXlsxDocument) {
            setDocumentId(resultsXlsxDocument)
        } else {
            const currWidget: WidgetProps = addWidget(ETasksExecution.GENERATE_REPORT)
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
                    setDocumentId(null)
                    return
                }
                setDocumentId(generatedDocumentId)
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
                    Export results to xlsx
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
