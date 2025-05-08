// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {Box, MenuItem} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "../Widget"

interface ExportElectionBulletinBoardProps {
    electionEventId: string
    tallySessionId: string
    handleClose: () => void
}

export const ExportElectionBulletinBoard: React.FC<ExportElectionBulletinBoardProps> = ({
    electionEventId,
    tallySessionId,
    handleClose,
}) => {
    const {t} = useTranslation()
    const [documentId, setDocumentId] = useState<string | null>(null)

    // const [generateTemplate] = useMutation<GenerateTemplateMutation>(GENERATE_TEMPLATE, {
    //     context: {
    //         headers: {
    //             "x-hasura-role": IPermissions.,
    //         },
    //     },
    // })
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const onClick = async (e: React.MouseEvent<HTMLElement>) => {
        e.preventDefault()
        e.stopPropagation()
        setDocumentId(null)
        const currWidget: WidgetProps = addWidget(ETasksExecution.EXPORT_VERIFIABLE_BULLETIN_BOARD)
        // try {
        //     let {data} = await generateTemplate({
        //         variables: {
        //             tallySessionId: tallySessionId,
        //             electionId: electionId,
        //             electionEventId: electionEventId,
        //             type:
        //                 reportType === ETemplateType.BALLOT_IMAGES
        //                     ? "BallotImages"
        //                     : "VoteReceipts",
        //         },
        //     })
        //     let response = data?.generate_template
        //     let taskId = response?.task_execution?.id
        //     let generatedDocumentId = response?.document_id

        //     if (!generatedDocumentId) {
        //         updateWidgetFail(currWidget.identifier)
        //         setDocumentId(null)
        //         return
        //     }
        //     setDocumentId(generatedDocumentId)
        //     setWidgetTaskId(currWidget.identifier, taskId)
        // } catch (e) {
        //     updateWidgetFail(currWidget.identifier)
        // }
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
                    {/* {t("tally.generateReport", {
                        name: t(`template.type.${reportType}`),
                    })} */}
                    Export verifiable bulletin board
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
