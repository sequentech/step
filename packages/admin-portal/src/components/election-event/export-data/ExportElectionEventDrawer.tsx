// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useNotify} from "react-admin"
import {ExportElectionEventMutation} from "@/gql/graphql"
import {EXPORT_ELECTION_EVENT} from "@/queries/ExportElectionEvent"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {IPermissions} from "@/types/keycloak"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../../../resources/User/DownloadDocument"
import {Dialog} from "@sequentech/ui-essentials"
import {Checkbox, FormControlLabel, FormGroup, IconButton, TextField, Tooltip} from "@mui/material"
import {styled} from "@mui/styles"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetProps} from "@/components/Widget"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"

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

// Helper function to generate a random password
const generateRandomPassword = (length = 12) => {
    const charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_."
    let password = ""
    for (let i = 0; i < length; i++) {
        password += charset.charAt(Math.floor(Math.random() * charset.length))
    }
    return password
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

    const [exportElectionEvent] = useMutation<ExportElectionEventMutation>(EXPORT_ELECTION_EVENT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EVENT_READ,
            },
        },
    })

    const confirmExportAction = async () => {
        console.log("CONFIRM EXPORT")
        setOpenExport(false)
        const currWidget: WidgetProps = addWidget(ETasksExecution.EXPORT_ELECTION_EVENT)
        setLoadingExport(true)

        const generatedPassword = encryptWithPassword ? generateRandomPassword() : password
        generatedPassword && setPassword(generatedPassword)

        try {
            const {data: exportElectionEventData, errors} = await exportElectionEvent({
                variables: {
                    electionEventId,
                    exportConfigurations: {
                        password: generatedPassword,
                        include_voters: includeVoters,
                        activity_logs: activityLogs,
                        bulletin_board: bulletinBoard,
                        publications: publications,
                        s3_files: s3Files,
                        scheduled_events: scheduledEvents,
                        reports: reports,
                    },
                },
            })

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

    const toggleBulletinBoard = () => {
        let newValue = !bulletinBoard
        setBulletinBoard(newValue)
        if (newValue) {
            setEncryptWithPassword(newValue)
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
                                disabled={bulletinBoard}
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
                                onChange={toggleBulletinBoard}
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
                                checked={s3Files}
                                onChange={() => setS3Files(!s3Files)}
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
                                onChange={() => setReports(!reports)}
                            />
                        }
                        label={t("electionEventScreen.export.reports")}
                    />
                </FormGroup>
            </Dialog>
            {exportDocumentId && (
                <>
                    <FormStyles.ShowProgress />
                    <DownloadDocument
                        documentId={exportDocumentId}
                        electionEventId={electionEventId ?? ""}
                        fileName={`election-event-${electionEventId}-export.${
                            encryptWithPassword ? "ezip" : "zip"
                        }`}
                        onDownload={() => {
                            console.log("onDownload called")
                            setExportDocumentId(undefined)
                            setOpenExport(false)
                        }}
                        onSucess={onDownloadSuccess}
                    />
                </>
            )}
            {openPasswordDialog && password && (
                <PasswordDialog password={password} onClose={() => setOpenPasswordDialog(false)} />
            )}
        </>
    )
}

const PasswordDialog: React.FC<{password: string; onClose: () => void}> = ({password, onClose}) => {
    const {t} = useTranslation()
    const notify = useNotify()

    const handleCopyPassword = () => {
        navigator.clipboard
            .writeText(password)
            .then(() => {
                notify(t("electionEventScreen.export.copiedSuccess"), {
                    type: "success",
                })
            })
            .catch((err) => {
                notify(t("electionEventScreen.export.copiedError"), {
                    type: "error",
                })
            })
    }

    return (
        <Dialog
            variant="info"
            open={true}
            handleClose={onClose}
            aria-labelledby="password-dialog-title"
            title={t("electionEventScreen.export.passwordTitle")}
            ok={"Ok"}
        >
            {t("electionEventScreen.export.passwordDescription")}
            <TextField
                fullWidth
                margin="normal"
                value={password}
                InputProps={{
                    readOnly: true,
                    endAdornment: (
                        <Tooltip
                            title={t("electionEventScreen.import.passwordDialog.copyPassword")}
                        >
                            <IconButton onClick={handleCopyPassword}>
                                <ContentCopyIcon />
                            </IconButton>
                        </Tooltip>
                    ),
                }}
            />
        </Dialog>
    )
}
