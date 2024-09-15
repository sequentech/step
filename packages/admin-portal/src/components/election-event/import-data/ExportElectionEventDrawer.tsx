import React, {useState} from "react"
import {ExportElectionEventMutation} from "@/gql/graphql"
import {EXPORT_ELECTION_EVENT} from "@/queries/ExportElectionEvent"
import {useMutation} from "@apollo/client"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {IPermissions} from "@/types/keycloak"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../../../resources/User/DownloadDocument"
import {Dialog} from "@sequentech/ui-essentials"
import {Checkbox, FormControlLabel, FormGroup} from "@mui/material"
import {styled} from "@mui/styles"

const StyledCheckbox = styled(Checkbox)({
    size: "small",
})

interface ExportWrapperProps {
    electionEventId: string
    openExport: boolean
    setOpenExport: (val: boolean) => void
    exportDocumentId: string | undefined
    setExportDocumentId: (val: string | undefined) => void
}

export const ExportElectionEventDrawer: React.FC<ExportWrapperProps> = ({
    electionEventId,
    openExport,
    setOpenExport,
    exportDocumentId,
    setExportDocumentId,
}) => {
    const notify = useNotify()
    const {t} = useTranslation()
    const [exportElectionEvent] = useMutation<ExportElectionEventMutation>(EXPORT_ELECTION_EVENT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EVENT_READ,
            },
        },
    })

    const [encryptWithPassword, setEncryptWithPassword] = useState(false)
    const [includeVoters, setIncludeVoters] = useState(false)
    const [activityLogs, setActivityLogs] = useState(false)
    const [ballotingBoard, setBallotingBoard] = useState(false)
    const [publications, setPublications] = useState(false)
    const [s3Files, setS3Files] = useState(false)

    const confirmExportAction = async () => {
        console.log("CONFIRM EXPORT")

        const {data: exportElectionEventData, errors} = await exportElectionEvent({
            variables: {
                electionEventId,
                options: {
                    encryptWithPassword,
                    includeVoters,
                    activityLogs,
                    ballotingBoard,
                    publications,
                    s3Files,
                },
            },
        })
        let documentId = exportElectionEventData?.export_election_event?.document_id
        if (errors || !documentId) {
            setOpenExport(false)
            notify(t(`electionEventScreen.export.exportError`), {type: "error"})
            console.log(`Error exporting users: ${errors}`)
            return
        }
        setExportDocumentId(documentId)
    }

    return (
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
                        <Checkbox
                            checked={activityLogs}
                            onChange={() => setActivityLogs(!activityLogs)}
                        />
                    }
                    label={t("electionEventScreen.export.activityLogs")}
                />
                <FormControlLabel
                    control={
                        <Checkbox
                            checked={ballotingBoard}
                            onChange={() => setBallotingBoard(!ballotingBoard)}
                        />
                    }
                    label={t("electionEventScreen.export.ballotingBoard")}
                />
                <FormControlLabel
                    control={
                        <Checkbox
                            checked={publications}
                            onChange={() => setPublications(!publications)}
                        />
                    }
                    label={t("electionEventScreen.export.publications")}
                />
                <FormControlLabel
                    control={<Checkbox checked={s3Files} onChange={() => setS3Files(!s3Files)} />}
                    label={t("electionEventScreen.export.s3Files")}
                />
            </FormGroup>

            {/* Show progress and download document */}
            {exportDocumentId && (
                <>
                    <FormStyles.ShowProgress />
                    <DownloadDocument
                        documentId={exportDocumentId}
                        electionEventId={electionEventId ?? ""}
                        fileName={`election-event-${electionEventId}-export.json`}
                        onDownload={() => {
                            console.log("onDownload called")
                            setExportDocumentId(undefined)
                            setOpenExport(false)
                        }}
                    />
                </>
            )}
        </Dialog>
    )
}
