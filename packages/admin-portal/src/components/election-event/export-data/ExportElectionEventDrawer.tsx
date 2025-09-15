// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {ExportElectionEventMutation} from "@/gql/graphql"
import {EXPORT_ELECTION_EVENT} from "@/queries/ExportElectionEvent"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {IPermissions} from "@/types/keycloak"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../../../resources/User/DownloadDocument"
import {Dialog} from "@sequentech/ui-essentials"
import {Checkbox, FormControlLabel, FormGroup} from "@mui/material"
import {styled} from "@mui/styles"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "@/components/Widget"
import {DecryptHelp, PasswordDialog} from "./PasswordDialog"
import {decryptionCommand} from "@/resources/Reports/ListReports"

const StyledCheckbox = styled(Checkbox)({
    size: "small",
})

interface ExportWrapperProps {
    electionEventId: string
    openExport: boolean
    setOpenExport: (val: boolean) => void
    exportDocumentId: string | undefined
    setExportDocumentId: (val: string | undefined) => void
    setLoadingExport: (val: boolean) => void
}

export const ExportElectionEventDrawer: React.FC<ExportWrapperProps> = ({
    electionEventId,
    openExport,
    setOpenExport,
    exportDocumentId,
    setExportDocumentId,
    setLoadingExport,
}) => {
    const {t} = useTranslation()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [encryptWithPassword, setEncryptWithPassword] = useState(false)
    const [includeVoters, setIncludeVoters] = useState(false)
    const [activityLogs, setActivityLogs] = useState(false)
    const [bulletinBoard, setBulletinBoard] = useState(false)
    const [publications, setPublications] = useState(false)
    const [s3Files, setS3Files] = useState(false)
    const [scheduledEvents, setScheduledEvents] = useState(false)
    const [password, setPassword] = useState<string>("")
    const [openPasswordDialog, setOpenPasswordDialog] = useState<boolean>(false)
    const [reports, setReports] = useState(false)
    const [applications, setApplications] = useState(false)
    const [tally, setTally] = useState(false)

    const [exportElectionEvent] = useMutation<ExportElectionEventMutation>(EXPORT_ELECTION_EVENT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EVENT_READ,
            },
        },
    })

    const resetState = () => {
        setEncryptWithPassword(false)
        setPassword("")
        setIncludeVoters(false)
        setActivityLogs(false)
        setBulletinBoard(false)
        setPublications(false)
        setS3Files(false)
        setScheduledEvents(false)
        setOpenPasswordDialog(false)
        setReports(false)
        setApplications(false)
        setTally(false)
    }

    const confirmExportAction = async () => {
        console.log("CONFIRM EXPORT")
        setOpenExport(false)
        const currWidget: WidgetProps = addWidget(ETasksExecution.EXPORT_ELECTION_EVENT)
        setLoadingExport(true)
        const isEncrypted = encryptWithPassword || bulletinBoard || reports || applications

        try {
            const {data: exportElectionEventData, errors} = await exportElectionEvent({
                variables: {
                    electionEventId,
                    exportConfigurations: {
                        is_encrypted: isEncrypted,
                        include_voters: includeVoters,
                        activity_logs: activityLogs,
                        bulletin_board: bulletinBoard,
                        publications: publications,
                        s3_files: s3Files,
                        scheduled_events: scheduledEvents,
                        reports: reports,
                        applications: applications,
                        tally: tally,
                    },
                },
            })

            const generatedPassword = exportElectionEventData?.export_election_event?.password
            generatedPassword && setPassword(generatedPassword)
            //if encrypt with password false reset state immediately otherwise wait until after password dialog is closed
            !encryptWithPassword && resetState()

            const documentId = exportElectionEventData?.export_election_event?.document_id

            if (errors || !documentId) {
                updateWidgetFail(currWidget.identifier)
                console.log(`Error exporting users: ${errors}`)
                setLoadingExport(false)
                return
            }

            const task_id = exportElectionEventData?.export_election_event?.task_execution.id
            setWidgetTaskId(currWidget.identifier, task_id)
            setExportDocumentId(documentId)
        } catch (e) {
            updateWidgetFail(currWidget.identifier)
            setLoadingExport(false)
        }
    }

    const onDownloadSuccess = () => {
        setLoadingExport(false)
        if (password) {
            setOpenPasswordDialog(true)
        }
    }

    const toggleCheckBoxWithPassword = (setter: (val: boolean) => void, newValue: boolean) => {
        setter(newValue)
        if (newValue) {
            setEncryptWithPassword(newValue)
        }
    }

    const toggleBulletinBoard = (newValue: boolean) => {
        setBulletinBoard(newValue)
        if (newValue) {
            toggleCheckBoxWithPassword(setBulletinBoard, newValue)
        }
        // Tally has to be exported with bulletin board
        if (!newValue && tally) {
            setTally(false)
        }
    }

    const toggleTallyCheckbox = (newValue: boolean) => {
        setTally(newValue)
        if (newValue) {
            toggleCheckBoxWithPassword(setBulletinBoard, newValue)
            setS3Files(newValue)
        }
    }
    const toggleS3FilesCheckbox = (newValue: boolean) => {
        setS3Files(newValue)
        if (!newValue && tally) {
            setTally(false)
        }
    }

    return (
        <>
            <Dialog
                variant="info"
                open={openExport}
                ok={t("common.label.export")}
                cancel={t("common.label.cancel")}
                title={t("electionEventScreen.export.title")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    } else {
                        setOpenExport(false)
                    }
                }}
            >
                {t("electionEventScreen.export.subtitle")}

                <FormGroup>
                    <FormControlLabel
                        control={
                            <Checkbox
                                disabled={bulletinBoard || reports || applications}
                                checked={encryptWithPassword}
                                onChange={() => setEncryptWithPassword(!encryptWithPassword)}
                            />
                        }
                        label={t("electionEventScreen.export.encryptWithPassword")}
                    />
                    <FormControlLabel
                        control={
                            <StyledCheckbox
                                checked={includeVoters}
                                onChange={() => setIncludeVoters(!includeVoters)}
                            />
                        }
                        label={t("electionEventScreen.export.includeVoters")}
                    />
                    <FormControlLabel
                        control={
                            <StyledCheckbox
                                checked={activityLogs}
                                onChange={() => setActivityLogs(!activityLogs)}
                            />
                        }
                        label={t("electionEventScreen.export.activityLogs")}
                    />
                    <FormControlLabel
                        control={
                            <StyledCheckbox
                                checked={bulletinBoard}
                                onChange={() => toggleBulletinBoard(!bulletinBoard)}
                            />
                        }
                        label={t("electionEventScreen.export.bulletinBoard")}
                    />
                    <FormControlLabel
                        control={
                            <StyledCheckbox
                                checked={publications}
                                onChange={() => setPublications(!publications)}
                            />
                        }
                        label={t("electionEventScreen.export.publications")}
                    />
                    <FormControlLabel
                        control={
                            <StyledCheckbox
                                checked={tally}
                                onChange={() => toggleTallyCheckbox(!tally)}
                            />
                        }
                        label={t("electionEventScreen.export.tally")}
                    />
                    <FormControlLabel
                        control={
                            <StyledCheckbox
                                checked={s3Files}
                                onChange={() => toggleS3FilesCheckbox(!s3Files)}
                            />
                        }
                        label={t("electionEventScreen.export.s3Files")}
                    />
                    <FormControlLabel
                        control={
                            <StyledCheckbox
                                checked={scheduledEvents}
                                onChange={() => setScheduledEvents(!scheduledEvents)}
                            />
                        }
                        label={t("electionEventScreen.export.scheduledEvents")}
                    />
                    <FormControlLabel
                        control={
                            <StyledCheckbox
                                checked={reports}
                                onChange={() => toggleCheckBoxWithPassword(setReports, !reports)}
                            />
                        }
                        label={t("electionEventScreen.export.reports")}
                    />
                    <FormControlLabel
                        control={
                            <StyledCheckbox
                                checked={applications}
                                onChange={() =>
                                    toggleCheckBoxWithPassword(setApplications, !applications)
                                }
                            />
                        }
                        label={t("electionEventScreen.export.applications")}
                    />
                </FormGroup>
            </Dialog>
            {exportDocumentId && (
                <>
                    <FormStyles.ShowProgress />
                    <DownloadDocument
                        documentId={exportDocumentId}
                        electionEventId={electionEventId ?? ""}
                        fileName={null}
                        onDownload={() => {
                            console.log("onDownload called")
                            setExportDocumentId(undefined)
                            setOpenExport(false)
                        }}
                        onSuccess={onDownloadSuccess}
                    />
                </>
            )}
            {openPasswordDialog && password && (
                <PasswordDialog password={password} onClose={resetState}>
                    <DecryptHelp decryptionCommand={decryptionCommand} />
                </PasswordDialog>
            )}
        </>
    )
}
