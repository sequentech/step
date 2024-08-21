// SPDX-FileCopyrightText: 2024 Félix Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    List,
    TextInput,
    DateField,
    FunctionField,
    TextField,
    DatagridConfigurable,
    useNotify,
    Identifier,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {Visibility} from "@mui/icons-material"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {StatusChip} from "@/components/StatusChip"
import {Dialog} from "@sequentech/ui-essentials"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
import {DownloadDocument} from "../User/DownloadDocument"
import {FormStyles} from "@/components/styles/FormStyles"
import {IPermissions} from "@/types/keycloak"
import {useMutation} from "@apollo/client"
import {EXPORT_ELECTION_EVENT_LOGS} from "@/queries/ExportElectionEventLogs"

interface ExportWrapperProps {
    electionEventId: string
    openExport: boolean
    setOpenExport: (val: boolean) => void
}
const ExportWrapper: React.FC<ExportWrapperProps> = ({
    electionEventId,
    openExport,
    setOpenExport,
}) => {
    const [exportDocumentId, setExportDocumentId] = React.useState<string | undefined>()
    const [exportElectionEvent] = useMutation(EXPORT_ELECTION_EVENT_LOGS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TASKS_READ,
            },
        },
    })
    const notify = useNotify()
    const {t} = useTranslation()

    const confirmExportAction = async () => {
        try {
            const {data: exportElectionEventData, errors} = await exportElectionEvent({
                variables: {
                    electionEventId,
                },
            })
            let documentId = exportElectionEventData?.export_election_event_logs?.document_id
            if (errors || !documentId) {
                setOpenExport(false)
                notify(t(`electionEventScreen.exportError`), {type: "error"})
                console.log(`Error exporting: ${errors}`)
                return
            }
            setExportDocumentId(documentId)
        } catch (error) {
            notify(t(`electionEventScreen.exportError`), {type: "error"})
            setOpenExport(false)
        }
    }

    return (
        <Dialog
            variant="info"
            open={openExport}
            ok={t("common.label.export")}
            cancel={t("common.label.cancel")}
            title={t("common.label.export")}
            handleClose={(result: boolean) => {
                if (result) {
                    confirmExportAction()
                } else {
                    setOpenExport(false)
                    setExportDocumentId(undefined)
                }
            }}
        >
            <ElectionStyles.Container>
                {t("common.export")}
                {exportDocumentId ? (
                    <>
                        <FormStyles.ShowProgress sx={{alignSelf: "center"}} />
                        <DownloadDocument
                            documentId={exportDocumentId}
                            electionEventId={electionEventId ?? ""}
                            fileName={`election-event-logs-${electionEventId}-export.csv`}
                            onDownload={() => {
                                console.log("onDownload called")
                                setExportDocumentId(undefined)
                                setOpenExport(false)
                            }}
                        />
                    </>
                ) : null}
            </ElectionStyles.Container>
        </Dialog>
    )
}

export interface ListTasksProps {
    onViewTask: (id: Identifier) => void
    electionEventRecord: Sequent_Backend_Election_Event
    aside?: ReactElement
}
export const ListTasks: React.FC<ListTasksProps> = ({onViewTask, electionEventRecord, aside}) => {
    const {t} = useTranslation()
    const [openExport, setOpenExport] = React.useState(false)
    const OMIT_FIELDS: string[] = []

    const filters: Array<ReactElement> = [
        <TextInput source="id" key="id_filter" label={t("filters.id")} />,
        <TextInput
            source="statement_kind"
            key="statement_kind_filter"
            label={t("filters.statementKind")}
        />,
    ]

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewTask,
        },
    ]

    const handleExport = () => {
        console.log("EXPORT")
        setOpenExport(true)
    }

    return (
        <>
            <List
                actions={<ListActions withImport={false} doExport={handleExport} />}
                resource="sequent_backend_tasks_execution"
                filters={filters}
                filter={{election_event_id: electionEventRecord?.id || undefined}}
                sort={{field: "start_at", order: "DESC"}} //TODO: organize
                // aside={aside}
                perPage={10}
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <TextField source="id" />
                    <TextField source="type" />
                    <DateField
                        source="start_at"
                        showTime={true}
                        label={t("tasksScreen.column.start_at")}
                    />
                    <FunctionField
                        label={t("tasksScreen.column.execution_status")}
                        render={(record: any) => <StatusChip status={record.execution_status} />}
                    />
                    <ActionsColumn actions={actions} label={t("common.label.actions")} />
                </DatagridConfigurable>
            </List>
            <ExportWrapper
                electionEventId={electionEventRecord.id}
                openExport={openExport}
                setOpenExport={setOpenExport}
            />
        </>
    )
}
