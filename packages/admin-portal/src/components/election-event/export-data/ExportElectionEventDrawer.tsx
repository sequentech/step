// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {Alert, Typography} from "@mui/material"
import {ExportElectionEventMutation} from "@/gql/graphql"
import {EXPORT_ELECTION_EVENT} from "@/queries/ExportElectionEvent"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {IPermissions} from "@/types/keycloak"
import {Dialog} from "@sequentech/ui-essentials"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "@/components/Widget"
import {DecryptHelp, PasswordDialog} from "./PasswordDialog"
import {decryptionCommand} from "@/resources/Reports/ListReports"

interface ExportWrapperProps {
    electionEventId: string
    openExport: boolean
    setOpenExport: (val: boolean) => void
    setLoadingExport: (val: boolean) => void
}

export const ExportElectionEventDrawer: React.FC<ExportWrapperProps> = ({
    electionEventId,
    openExport,
    setOpenExport,
    setLoadingExport,
}) => {
    const {t} = useTranslation()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [password, setPassword] = useState<string>("")
    const [openPasswordDialog, setOpenPasswordDialog] = useState<boolean>(false)

    const [exportElectionEvent] = useMutation<ExportElectionEventMutation>(EXPORT_ELECTION_EVENT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EXECUTION_EXPORT,
            },
        },
    })

    const resetState = () => {
        setPassword("")
        setOpenPasswordDialog(false)
    }

    const confirmExportAction = async () => {
        console.log("CONFIRM EXPORT")
        setOpenExport(false)
        const currWidget: WidgetProps = addWidget(ETasksExecution.EXPORT_ELECTION_EVENT, true)
        setLoadingExport(true)
        const isEncrypted = true

        try {
            const {data: exportElectionEventData, errors} = await exportElectionEvent({
                variables: {
                    electionEventId,
                    exportConfigurations: {
                        is_encrypted: isEncrypted,
                        encrypt_with_password: true,
                        include_voters: true,
                        activity_logs: true,
                        bulletin_board: true,
                        publications: true,
                        s3_files: true,
                        scheduled_events: true,
                        reports: true,
                        applications: true,
                        tally: true,
                        include_certificates: true,
                    },
                },
            })

            const generatedPassword = exportElectionEventData?.export_election_event?.password
            generatedPassword && setPassword(generatedPassword)
            // Keep state until the generated-password dialog closes for every
            // export that is encrypted, including formats that require it.
            !isEncrypted && resetState()

            const documentId = exportElectionEventData?.export_election_event?.document_id

            if (errors || !documentId) {
                updateWidgetFail(currWidget.identifier)
                console.log(`Error exporting users: ${errors}`)
                setLoadingExport(false)
                return
            }

            const task_id = exportElectionEventData?.export_election_event?.task_execution.id
            setWidgetTaskId(currWidget.identifier, task_id)
            setLoadingExport(false)

            if (generatedPassword) {
                setOpenPasswordDialog(true)
            }
        } catch (e) {
            updateWidgetFail(currWidget.identifier)
            setLoadingExport(false)
        }
    }

    return (
        <>
            <Dialog
                variant="info"
                open={openExport}
                ok={String(t("common.label.export"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("electionEventScreen.export.title"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    } else {
                        setOpenExport(false)
                    }
                }}
            >
                {t("electionEventScreen.export.subtitle")}

                <Alert severity="info" sx={{mt: 2, mb: 2}}>
                    Export is available in Post-processing and requires the configured
                    election-board approvals. The complete archive is encrypted and signed. Save the
                    generated password separately.
                </Alert>
                <Typography component="div" variant="body2">
                    The archive includes the voter register, electoral log, ballot boxes,
                    publications, election rules and dates, tally evidence, reports, applications
                    and public certificates. Trustee and protocol-manager private keys are excluded.
                </Typography>
            </Dialog>
            {openPasswordDialog && password && (
                <PasswordDialog password={password} onClose={resetState}>
                    <DecryptHelp decryptionCommand={decryptionCommand} />
                </PasswordDialog>
            )}
        </>
    )
}
