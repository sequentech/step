import {Action, ActionsColumn} from "@/components/ActionButons"
import ElectionHeader from "@/components/ElectionHeader"
import {ListActions} from "@/components/ListActions"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {styled} from "@mui/material"
import React, {ReactElement, useContext, useState} from "react"
import {DatagridConfigurable, List, TextField, useSidebarState, WrapperField} from "react-admin"
import {useTranslation} from "react-i18next"

export const DataGridContainerStyle = styled(DatagridConfigurable)<{isOpenSideBar?: boolean}>`
    @media (min-width: ${({theme}) => theme.breakpoints.values.md}px) {
        overflow-x: auto;
        width: 100%;
        ${({isOpenSideBar}) =>
            `max-width: ${isOpenSideBar ? "calc(100vw - 355px)" : "calc(100vw - 108px)"};`}
        &  > div:first-child {
            position: absolute;
            width: 100%;
        }
    }
`

interface ListReportsProps {
    electionEventId: string
}

const ListReports: React.FC<ListReportsProps> = ({electionEventId}) => {
    const {t} = useTranslation()
    const [openCreateReport, setOpenCreateReport] = useState(false)
    const [isEditReport, setIsEditReport] = useState(false)
    const [isOpenSidebar] = useSidebarState()
    const [selectedReportId, setSelectedReportId] = useState<string | null>(null)
    const {globalSettings} = useContext(SettingsContext)
    const [tenantId] = useTenantStore()
    const onOpenDrawer = () => setOpenCreateReport(!openCreateReport)

    const editAction = (id: any) => {
        setOpenCreateReport(true)
        setIsEditReport(true)
        setSelectedReportId(id)
    }

    const handleClose = () => {
        setOpenCreateReport(false)
        setIsEditReport(false)
    }

    const Empty = () => {
        return <></>
    }
    console.log("tenjkvd")

    const OMIT_FIELDS: Array<string> = []

    const Filters: Array<ReactElement> = []

    const actions: Action[] = []

    return (
        <>
            <ElectionHeader title={t("eventsScreen.title")} subtitle="eventsScreen.subtitle" />
            <List
                resource="sequent_backend_report"
                filter={{
                    election_event_id: electionEventId || undefined,
                    tenant_id: tenantId,
                }}
                filters={Filters}
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<Empty />}
                actions={
                    <ListActions
                        withImport={false}
                        withExport={false}
                        open={openCreateReport}
                        setOpen={onOpenDrawer}
                        Component={
                            <></>
                            // <CreateEvent
                            //     electionEventId={electionEventId}
                            //     setIsOpenDrawer={setOpenCreateEvent}
                            //     getElectionName={getElectionName}
                            // />
                        }
                    />
                }
            >
                <DataGridContainerStyle
                    // bulkActionButtons={<BulkActionButtons />}
                    isOpenSideBar={isOpenSidebar}
                    omit={OMIT_FIELDS}
                >
                    <TextField source="report_type" />
                    <TextField source="template_alias" />

                    {/* <FunctionField
                        label={t("eventsScreen.fields.electionId")}
                        source="election_id"

                    />
                    <FunctionField
                        label={t("eventsScreen.fields.eventProcessor")}
                        source="event_processor"
                        render={(record: {event_processor: keyof typeof EventProcessors}) =>
                            t("eventsScreen.eventType." + record.event_processor)
                        }
                    />
                    <FunctionField
                        label={t("eventsScreen.fields.stoppedAt")}
                        source="stopped_at"
                        render={(record: Sequent_Backend_Scheduled_Event) =>
                            (record.stopped_at && new Date(record.stopped_at).toLocaleString()) ||
                            "-"
                        }
                    />
                    <FunctionField
                        label={t("eventsScreen.fields.scheduledDate")}
                        source="cron_config.scheduled_date"
                        render={(record: Sequent_Backend_Scheduled_Event) =>
                            ((record.cron_config as ICronConfig | undefined)?.scheduled_date &&
                                new Date(record.cron_config.scheduled_date).toLocaleString()) ||
                            "-"
                        }
                    /> */}
                    <WrapperField label={t("common.label.actions")}>
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DataGridContainerStyle>
            </List>
        </>
    )
}

export default ListReports
