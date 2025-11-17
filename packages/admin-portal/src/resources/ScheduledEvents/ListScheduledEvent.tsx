// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {Button, styled, Typography} from "@mui/material"
import React, {ReactElement, useContext, useEffect, useMemo, useRef, useState} from "react"
import moment from "moment-timezone"
import {
    DatagridConfigurable,
    FunctionField,
    List,
    SelectInput,
    TextField,
    TextInput,
    useGetList,
    useGetOne,
    useListContext,
    useNotify,
    useRefresh,
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

const FilterWatcher = ({
    fieldName,
    onFilterChange,
}: {
    fieldName: string
    onFilterChange: () => void
}) => {
    const {filterValues} = useListContext()
    const previousFilters = useRef(filterValues)

    useEffect(() => {
        // Check which filters were removed
        Object.keys(previousFilters.current).forEach((key) => {
            if (!(key in filterValues)) {
                if (key === fieldName) {
                    onFilterChange()
                }
            }
        })

        previousFilters.current = filterValues
    }, [filterValues])

    return null
}

interface EditEventsProps {
    electionEventId: string
}
const ListScheduledEvents: React.FC<EditEventsProps> = ({electionEventId}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
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
            pagination: {page: 1, perPage: 500},
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

    const [eventScreeElectionId, setEventScreenElectionId] = useState<string | null>(null)

    const electionIds = useMemo(() => {
        let electionsList = elections?.map((election) => election.id) ?? []
        if (eventScreeElectionId) {
            electionsList = electionsList.filter((item) => item === eventScreeElectionId)
        }
        return electionsList
    }, [elections, eventScreeElectionId])

    const getElectionName = (scheduledEvent: Sequent_Backend_Scheduled_Event): string => {
        let electionId = (scheduledEvent?.event_payload as IManageElectionDatePayload | undefined)
            ?.election_id
        const foundElection = elections?.find((item) => electionId === item.id)
        return (foundElection && aliasRenderer(foundElection)) || "-"
    }

    const OMIT_FIELDS: Array<string> = ["id"]

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
                            <ResourceListStyles.CreateIcon icon={faPlus as any} />
                            {t(`eventsScreen.empty.button`)}
                        </Button>
                    </ResourceListStyles.EmptyButtonList>
                </>
            ) : null}
        </ResourceListStyles.EmptyBox>
    )

    // Define the filters as an array of elements
    const Filters: Array<ReactElement> = [
        <SelectInput
            source="event_processor"
            key="event_processor_filter"
            label={String(t("eventsScreen.fields.eventProcessor"))}
            choices={Object.values(EventProcessors).map((eventType) => ({
                id: eventType,
                name: t(`eventsScreen.eventType.${eventType}`),
            }))}
        />,
        <SelectInput
            source="event_payload.election_id"
            key="election_id_filter"
            label={String(t("eventsScreen.fields.electionId"))}
            choices={elections?.map((election) => ({
                id: election.id,
                name: election.alias || election.name || "-",
            }))}
            onChange={(e: any) => {
                setEventScreenElectionId(e.target.value)
            }}
        />,
        <TextInput key="id_filter" source="id" label={"id"} />,
    ]

    return (
        <>
            <ElectionHeader
                title={String(t("eventsScreen.title"))}
                subtitle="eventsScreen.subtitle"
            />
            <List
                resource="sequent_backend_scheduled_event"
                filter={{
                    election_event_id: electionEventId || undefined,
                    tenant_id: tenantId,
                    archived_at: {
                        format: "hasura-raw-query",
                        value: {_is_null: true},
                    },
                    event_payload: {
                        format: "hasura-raw-query",
                        value: {
                            _contains: {election_id: electionIds},
                        },
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
                        withComponent={canCreateScheduledEvent}
                    />
                }
                disableSyncWithLocation
            >
                <DatagridConfigurable bulkActionButtons={false} omit={OMIT_FIELDS}>
                    <FilterWatcher
                        fieldName="event_payload"
                        onFilterChange={() => setEventScreenElectionId(null)}
                    />
                    <TextField source="id" />
                    <FunctionField
                        label={String(t("eventsScreen.fields.electionId"))}
                        source="event_payload.election_id"
                        render={getElectionName}
                    />
                    <FunctionField
                        label={String(t("eventsScreen.fields.eventProcessor"))}
                        source="event_processor"
                        render={(record: {event_processor: keyof typeof EventProcessors}) =>
                            t("eventsScreen.eventType." + record.event_processor)
                        }
                    />
                    <FunctionField
                        label={String(t("eventsScreen.fields.stoppedAt"))}
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
                        label={String(t("eventsScreen.fields.scheduledDate"))}
                        source="cron_config.scheduled_date"
                        render={(record: Sequent_Backend_Scheduled_Event) =>
                            ((record.cron_config as ICronConfig | undefined)?.scheduled_date &&
                                moment
                                    .tz(new Date(record.cron_config.scheduled_date), userTimeZone)
                                    .toLocaleString()) ||
                            "-"
                        }
                    />
                    <WrapperField label={String(t("common.label.actions"))}>
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
                ok={String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
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
