// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Tooltip} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "../Widget"
import {EXPORT_VERIFIABLE_BULLETIN_BOARD} from "@/queries/ExportVerifiableBulletinBoard"
import DownloadIcon from "@mui/icons-material/Download"
import {StyledIconButton} from "../ActionButons"
import {ExportVerifiableBulletinBoardMutation} from "@/gql/graphql"
import {PasswordDialog, DecryptHelp} from "../election-event/export-data/PasswordDialog"
import {decryptionCommand} from "@/resources/Reports/ListReports"

interface ExportVerifiableBulletinBoardProps {
    electionEventId: string
    tallySessionId: string
    tenantId: string | null
}

export const ExportVerifiableBulletinBoard: React.FC<ExportVerifiableBulletinBoardProps> = ({
    electionEventId,
    tallySessionId,
    tenantId,
}) => {
    const {t} = useTranslation()
    const [documentId, setDocumentId] = useState<string | null>(null)
    const [password, setPassword] = useState<string>("")
    const [openPasswordDialog, setOpenPasswordDialog] = useState<boolean>(false)

    const [exportVerifiableBulletinBoard] = useMutation<ExportVerifiableBulletinBoardMutation>(
        EXPORT_VERIFIABLE_BULLETIN_BOARD,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.EXPORT_VERIFIABLE_BULLETIN_BOARD,
                },
            },
        }
    )
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const onClick = async (e: React.MouseEvent<HTMLElement>) => {
        e.preventDefault()
        e.stopPropagation()
        setDocumentId(null)
        const currWidget: WidgetProps = addWidget(ETasksExecution.EXPORT_VERIFIABLE_BULLETIN_BOARD)
        try {
            const {data, errors} = await exportVerifiableBulletinBoard({
                variables: {
                    tenantId: tenantId,
                    electionEventId: electionEventId,
                    tallySessionId: tallySessionId,
                },
            })

            let response = data?.export_verifiable_bulletin_board
            let taskId = response?.task_execution?.id
            let generatedDocumentId = response?.document_id

            if (!generatedDocumentId || errors) {
                updateWidgetFail(currWidget.identifier)
                setDocumentId(null)
                return
            }
            console.log({generatedDocumentId})
            const generatedPassword = response?.password || "";
            setPassword(generatedPassword)

            setDocumentId(generatedDocumentId)
            setWidgetTaskId(currWidget.identifier, taskId)
        } catch (e) {
            updateWidgetFail(currWidget.identifier)
        }
    }

    const onDownloadSuccess = () => {
        if (password) {
            setOpenPasswordDialog(true)
        }
    }

    const resetState = () => {
        setPassword("")
        setOpenPasswordDialog(false)
        setDocumentId(null)
    }

    return (
        <>
            <Tooltip title={"Export verifiable bulletin board"}>
                <StyledIconButton
                    className=""
                    key={"export-verifiable-bulletin-board"}
                    onClick={onClick}
                >
                    <DownloadIcon />
                </StyledIconButton>
            </Tooltip>
            {documentId ? (
                <DownloadDocument
                    documentId={documentId}
                    electionEventId={electionEventId}
                    fileName={null}
                    onDownload={() => {
                        setDocumentId(null)
                    }}
                    onSucess={onDownloadSuccess}
                />
            ) : null}
            {openPasswordDialog && password && (
                <PasswordDialog password={password} onClose={resetState}>
                    <DecryptHelp decryptionCommand={decryptionCommand} />
                </PasswordDialog>
            )}
        </>
    )
}
