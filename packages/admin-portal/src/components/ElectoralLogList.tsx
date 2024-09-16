// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect} from "react"
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
import {FormStyles} from "./styles/FormStyles"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {EXPORT_ELECTION_EVENT_LOGS} from "@/queries/ExportElectionEventLogs"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {ElectionStyles} from "./styles/ElectionStyles"
import {useLocation} from "react-router"

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
                "x-hasura-role": IPermissions.LOGS_READ,
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

export interface ElectoralLogListProps {
    aside?: ReactElement
    filterToShow?: ElectoralLogFilters
    filterValue?: string
    showActions?: boolean
}

export enum ElectoralLogFilters {
    ID = "id",
    STATEMENT_KIND = "statement_kind",
    USER_ID = "user_id",
}

export const ElectoralLogList: React.FC<ElectoralLogListProps> = ({
    aside,
    filterToShow,
    filterValue,
    showActions = true,
}) => {
    const [tenantId] = useTenantStore()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const location = useLocation()
    const params = new URLSearchParams(location.search)
    const user_id = params.get("user_id")
    const filters: Array<ReactElement> = []

    useEffect(() => {
        for (const filter of Object.values(ElectoralLogFilters)) {
            filters.push(<TextInput key={filter} source={filter} />)
        }
    }, [])
    const [openExport, setOpenExport] = React.useState(false)

    const handleExport = () => {
        console.log("EXPORT")
        setOpenExport(true)
    }

    const filterObject: {[key: string]: any} = {
        election_event_id: record?.id || undefined,
    }

    if (filterToShow) {
        filterObject[filterToShow] = filterValue || undefined
    }

    return (
        <>
            <List
                resource="electoral_log"
                actions={showActions && <ListActions withImport={false} doExport={handleExport} />}
                filters={filters}
                filter={filterObject}
                sort={{
                    field: "id",
                    order: "DESC",
                }}
                aside={aside}
            >
                <DatagridConfigurable bulkActionButtons={<></>}>
                    <NumberField source="id" />
                    <FunctionField
                        source="user_id"
                        render={(record: any) => {
                            const userId = record.user_id
                            return (
                                <span style={{display: "block", textAlign: "center"}}>
                                    {!userId || userId === "null" ? <span>-</span> : userId}
                                </span>
                            )
                        }}
                    />
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
