// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {Button, styled, Typography} from "@mui/material"
import React, {ReactElement, useContext, useState} from "react"
import moment from "moment-timezone"
import {
    DatagridConfigurable,
    FunctionField,
    List,
    TextField,
    useGetList,
    useGetOne,
    useNotify,
    useRefresh,
    useSidebarState,
    WrapperField,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"

import {
    ManageElectionDatesMutation,
    ManageElectionDatesMutationVariables,
    Sequent_Backend_Election,
    Sequent_Backend_Scheduled_Event,
} from "@/gql/graphql"
import CreateEvent, {EventProcessors} from "./CreateScheduledEvent"
import {Dialog} from "@sequentech/ui-essentials"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {IPermissions} from "@/types/keycloak"
import {useMutation} from "@apollo/client"
import {MANAGE_ELECTION_DATES} from "@/queries/ManageElectionDates"
import {ICronConfig, IManageElectionDatePayload} from "@/types/scheduledEvents"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import ElectionHeader from "@/components/ElectionHeader"
import {useScheduledEventPermissions} from "../ElectionEvent/useScheduledEventPermissions"

export const DataGridContainerStyle = styled(DatagridConfigurable)<{isOpenSideBar?: boolean}>`
    @media (min-width: ${({theme}) => theme.breakpoints.values.md}px) {
        width: 100%;
        ${({isOpenSideBar}) =>
            `max-width: ${isOpenSideBar ? "calc(100vw - 355px)" : "calc(100vw - 108px)"};`}
        &  > div:first-child {
            position: absolute;
            width: 100%;
        }
    }
`

interface EditEventsProps {
    electionEventId: string
}
const ListScheduledEvents: React.FC<EditEventsProps> = ({electionEventId}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const [isOpenSidebar] = useSidebarState()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const notify = useNotify()
    const [isDeleteModalOpen, setIsDeleteModalOpen] = useState(false)
    const [isDeleteId, setIsDeleteId] = useState<string | undefined>()
    const [isEditEvent, setIsEditEvent] = useState(false)
    const [openCreateEvent, setOpenCreateEvent] = useState(false)
    const [selectedEventId, setSelectedEventId] = useState<string | undefined>()
    const aliasRenderer = useAliasRenderer()

    const {
        canWriteScheduledEvent,
        canCreateScheduledEvent,
        canDeleteScheduledEvent,
        showScheduledEventColumns,
    } = useScheduledEventPermissions()

    const [manageElectionDates] = useMutation<ManageElectionDatesMutation>(MANAGE_ELECTION_DATES, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.SCHEDULED_EVENT_WRITE,
            },
        },
    })

    const {data: scheduledEventToDelete} = useGetOne<Sequent_Backend_Scheduled_Event>(
        "sequent_backend_scheduled_event",
        {
            id: isDeleteId ?? tenantId,
            meta: {tenant_id: tenantId},
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchIntervalInBackground: true,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )
    const {data: elections} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            pagination: {page: 1, perPage: 100},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
                archived_at: {
                    format: "hasura-raw-query",
                    value: {_is_null: true},
                },
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const getElectionName = (scheduledEvent: Sequent_Backend_Scheduled_Event): string => {
        let electionId = (scheduledEvent?.event_payload as IManageElectionDatePayload | undefined)
            ?.election_id
        const foundElection = elections?.find((item) => electionId === item.id)
        return (foundElection && aliasRenderer(foundElection)) || "-"
    }

    const OMIT_FIELDS: Array<string> = ["id"]

    const Filters: Array<ReactElement> = []

    const editAction = (id: any) => {
        setOpenCreateEvent(true)
        setIsEditEvent(true)
        setSelectedEventId(id)
    }

    const handleClose = () => {
        setOpenCreateEvent(false)
        setIsEditEvent(false)
    }

    const confirmDeleteAction = async () => {
        if (scheduledEventToDelete) {
            let payload = scheduledEventToDelete.event_payload as
                | IManageElectionDatePayload
                | undefined
            if (
                scheduledEventToDelete.election_event_id &&
                scheduledEventToDelete.event_processor
            ) {
                try {
                    let variables: ManageElectionDatesMutationVariables = {
                        electionEventId: scheduledEventToDelete.election_event_id,
                        electionId: payload?.election_id,
                        scheduledDate: undefined, // to archive, set date to undefined
                        eventProcessor: scheduledEventToDelete.event_processor,
                    }
                    const {errors} = await manageElectionDates({
                        variables,
                    })
                    if (errors) {
                        console.error(errors)
                        notify(t("eventsScreen.messages.editError"), {type: "error"})
                    }
                } catch (error) {
                    console.error(error)
                    notify(t("eventsScreen.messages.editError"), {type: "error"})
                }
            }
        }
        refresh()
        setIsDeleteModalOpen(false)
    }

    const deleteAction = (id: any) => {
        setIsDeleteId(id as string)
        setIsDeleteModalOpen(true)
        setOpenCreateEvent(false)
    }

    const actions: Action[] = [
        {
            icon: <EditIcon className="edit-voter-icon" />,
            action: (id) => editAction(id),
            showAction: () => canWriteScheduledEvent,
        },
        {
            icon: <DeleteIcon className="delete-voter-icon" />,
            action: (id) => deleteAction(id),
            showAction: () => canDeleteScheduledEvent,
            label: t(`common.label.delete`),
            className: "delete-voter-icon",
        },
    ]

    const onOpenDrawer = () => {
        setOpenCreateEvent(!openCreateEvent)
    }

    const userTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t(`eventsScreen.empty.header`)}
            </Typography>
            {canCreateScheduledEvent ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t(`eventsScreen.empty.body`)}
                    </Typography>
                    <ResourceListStyles.EmptyButtonList className="voter-add-button">
                        <Button onClick={() => setOpenCreateEvent(true)}>
                            <ResourceListStyles.CreateIcon icon={faPlus} />
                            {t(`eventsScreen.empty.button`)}
                        </Button>
                    </ResourceListStyles.EmptyButtonList>
                </>
            ) : null}
        </ResourceListStyles.EmptyBox>
    )
    return (
        <>
            <ElectionHeader title={t("eventsScreen.title")} subtitle="eventsScreen.subtitle" />
            <List
                resource="sequent_backend_scheduled_event"
                filter={{
                    election_event_id: electionEventId || undefined,
                    tenant_id: tenantId,
                    archived_at: {
                        format: "hasura-raw-query",
                        value: {_is_null: true},
                    },
                }}
                filters={Filters}
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<Empty />}
                actions={
                    <ListActions
                        withColumns={showScheduledEventColumns}
                        withImport={false}
                        withExport={false}
                        open={openCreateEvent}
                        setOpen={onOpenDrawer}
                        Component={
                            <CreateEvent
                                electionEventId={electionEventId}
                                setIsOpenDrawer={setOpenCreateEvent}
                                getElectionName={getElectionName}
                            />
                        }
                        withAction={canCreateScheduledEvent}
                    />
                }
                disableSyncWithLocation
            >
                <DatagridConfigurable bulkActionButtons={false} omit={OMIT_FIELDS}>
                    <TextField source="id" />

                    <FunctionField
                        label={t("eventsScreen.fields.electionId")}
                        source="event_payload.election_id"
                        render={getElectionName}
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
                            (record.stopped_at &&
                                moment
                                    .tz(new Date(record.stopped_at), userTimeZone)
                                    .toLocaleString()) ||
                            "-"
                        }
                    />
                    <FunctionField
                        label={t("eventsScreen.fields.scheduledDate")}
                        source="cron_config.scheduled_date"
                        render={(record: Sequent_Backend_Scheduled_Event) =>
                            ((record.cron_config as ICronConfig | undefined)?.scheduled_date &&
                                moment
                                    .tz(new Date(record.cron_config.scheduled_date), userTimeZone)
                                    .toLocaleString()) ||
                            "-"
                        }
                    />
                    <WrapperField label={t("common.label.actions")}>
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DatagridConfigurable>
            </List>
            <ResourceListStyles.Drawer anchor="right" open={openCreateEvent} onClose={handleClose}>
                <CreateEvent
                    electionEventId={electionEventId}
                    setIsOpenDrawer={setOpenCreateEvent}
                    isEditEvent={isEditEvent}
                    selectedEventId={selectedEventId}
                    getElectionName={getElectionName}
                />
            </ResourceListStyles.Drawer>
            <Dialog
                variant="warning"
                open={isDeleteModalOpen}
                ok={t("common.label.delete")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={async (result: boolean) => {
                    if (result) {
                        await confirmDeleteAction()
                    }
                    setIsDeleteModalOpen(false)
                }}
            >
                {t(`eventsScreen.edit.delete`)}
            </Dialog>
        </>
    )
}

export default ListScheduledEvents
