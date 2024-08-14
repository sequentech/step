// SPDX-FileCopyrightText: 2024 Félix Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    FunctionField,
    TextInput,
    NumberField,
    useRecordContext,
    useNotify,
} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ListActions} from "@/components/ListActions"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
import {EXPORT_ELECTION_EVENT_TASKS} from "@/queries/ExportElectionEventTasks"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../User/DownloadDocument"

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
    const [exportElectionEvent] = useMutation(EXPORT_ELECTION_EVENT_TASKS, {
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
            //TODO: - search export_election_event_logs and copy
            let documentId = exportElectionEventData?.export_election_event_tasks?.document_id
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
                            fileName={`election-event-tasks-${electionEventId}-export.csv`}
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
    aside?: ReactElement
}

export const ListTasks: React.FC<ListTasksProps> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const filters: Array<ReactElement> = [
        <TextInput source="id" key={0} />,
        <TextInput source="statement_kind" key={1} />,
    ]
    const [openExport, setOpenExport] = React.useState(false)

    const handleExport = () => {
        console.log("EXPORT")
        setOpenExport(true)
    }

    return (
        <>
            <List
                resource="tasks-execution"
                actions={<ListActions withImport={false} doExport={handleExport} />}
                filters={filters}
                filter={{
                    election_event_id: record?.id || undefined,
                }}
                sort={{
                    field: "id",
                    order: "DESC",
                }}
                aside={aside}
            >
                <DatagridConfigurable bulkActionButtons={<></>}>
                    <NumberField source="id" />
                    <FunctionField
                        source="created"
                        render={(record: any) => new Date(record.created * 1000).toUTCString()}
                    />
                    <FunctionField
                        source="statement_timestamp"
                        render={(record: any) =>
                            new Date(record.statement_timestamp * 1000).toUTCString()
                        }
                    />
                    <TextField source="statement_kind" />
                    <TextField source="message" sx={{wordBreak: "break-word"}} />
                </DatagridConfigurable>
            </List>

            <ExportWrapper
                electionEventId={record.id}
                openExport={openExport}
                setOpenExport={setOpenExport}
            />
        </>
    )
}
