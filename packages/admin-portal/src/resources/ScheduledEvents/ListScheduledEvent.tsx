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
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {ThreeStateDatagridHeader} from "@/components/ThreeStateDatagridHeader"
import {useQuery} from "@apollo/client"
import {Chip, Stack} from "@mui/material"
import {ZonedDateTime, hasOffset} from "@/components/timezones/ZonedDateTime"
import {useTimeZoneContext} from "@/components/timezones/useTimeZoneContext"
import {
    EXPORT_SCHEDULE,
    GET_LIFECYCLE_SNAPSHOTS,
    GET_SCHEDULED_OUTCOMES,
    type ExportScheduleData,
    type GetLifecycleSnapshotsData,
    type GetScheduledOutcomesData,
} from "@/queries/Lifecycle"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {isVotingTransition} from "./CreateScheduledEvent"
import {
    ScheduleBanners,
    outcomeFilterIds,
    outcomesByRow,
    outcomesForElection,
    type EOutcomeFilter,
} from "./ScheduleBanners"
import {RowOutcome} from "./RowOutcome"
import {WARNING_TEXT} from "@/components/timezones/ScheduledOutcome"
import {ImportScheduleDrawer} from "./ImportScheduleDrawer"
import {unpublishedEventIds} from "./unpublishedChanges"

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
            id: isDeleteId,
            meta: {tenant_id: tenantId},
        },
        {
            enabled: !!isDeleteId,
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

    const electionIdOf = (scheduledEvent: Sequent_Backend_Scheduled_Event): string | null =>
        (scheduledEvent?.event_payload as IManageElectionDatePayload | undefined)?.election_id ??
        null

    // Event-wide rows have no election: they read "All elections".
    const getElectionName = (scheduledEvent: Sequent_Backend_Scheduled_Event): string => {
        const electionId = electionIdOf(scheduledEvent)
        if (!electionId) return t("lifecycle.schedule.allElections")
        const foundElection = elections?.find((item) => electionId === item.id)
        return (foundElection && aliasRenderer(foundElection)) || "-"
    }

    const zones = useTimeZoneContext(electionEventId)
    // The entered wall time and zone remain in cron_config for editing; the
    // list displays that same instant in its Post's zone (or the event primary).
    const zoneOfRow = (scheduledEvent: Sequent_Backend_Scheduled_Event) =>
        zones.zoneOf(electionIdOf(scheduledEvent))

    // What each future opening and closing will do (design §5c), and the published configuration.
    const {data: outcomesData, error: outcomesError} = useQuery<GetScheduledOutcomesData>(
        GET_SCHEDULED_OUTCOMES,
        {
            context: {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_READ}},
            variables: {electionEventId},
            skip: !electionEventId,
            pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )
    const outcomes = useMemo(
        () => outcomesByRow(outcomesData?.get_scheduled_outcomes?.outcomes ?? []),
        [outcomesData]
    )
    // Filtered to one election, the banners count only that election's transitions.
    const shownOutcomes = useMemo(
        () => outcomesForElection(outcomes, eventScreeElectionId),
        [outcomes, eventScreeElectionId]
    )
    const {data: snapshotData} = useQuery<GetLifecycleSnapshotsData>(GET_LIFECYCLE_SNAPSHOTS, {
        variables: {electionEventId},
        skip: !electionEventId,
    })
    const publications = useMemo(
        () => snapshotData?.get_lifecycle_snapshots?.snapshots ?? [],
        [snapshotData]
    )
    // Every active row of the event, for the banners' totals.
    const {data: allScheduledEvents} = useGetList<Sequent_Backend_Scheduled_Event>(
        "sequent_backend_scheduled_event",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
                archived_at: {format: "hasura-raw-query", value: {_is_null: true}},
            },
        },
        {enabled: !!electionEventId, refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS}
    )
    const unpublished = useMemo(
        () =>
            unpublishedEventIds(
                allScheduledEvents ?? [],
                publications,
                (elections ?? []).map(({id}) => String(id))
            ),
        [allScheduledEvents, publications, elections]
    )
    const [outcomeFilter, setOutcomeFilter] = useState<EOutcomeFilter | null>(null)
    const filteredIds = outcomeFilter ? outcomeFilterIds(shownOutcomes, outcomeFilter) : null
    const [openImport, setOpenImport] = useState(false)
    const [exportDocumentId, setExportDocumentId] = useState<string | null>(null)
    const [exportSchedule, {loading: exporting}] = useMutation<ExportScheduleData>(
        EXPORT_SCHEDULE,
        {context: {headers: {"x-hasura-role": IPermissions.SCHEDULED_EVENT_WRITE}}}
    )
    const doExport = async () => {
        try {
            const {data} = await exportSchedule({variables: {electionEventId}})
            const documentId = data?.export_schedule?.document_id
            if (!documentId) throw new Error("No document")
            setExportDocumentId(documentId)
        } catch (error) {
            notify(getGraphQLActionErrorReason(error) ?? t("lifecycle.schedule.exportError"), {
                type: "error",
            })
        }
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
                name: aliasRenderer(election),
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
            <ScheduleBanners
                electionEventId={electionEventId}
                outcomes={shownOutcomes}
                retainedCloses={(
                    outcomesData?.get_scheduled_outcomes?.retained_closes ?? []
                ).filter(
                    (close) => !eventScreeElectionId || close.election_id === eventScreeElectionId
                )}
                retainedUnavailable={!!outcomesError}
                electionNameOf={(electionId) => {
                    const election = elections?.find((item) => item.id === electionId)
                    return election ? aliasRenderer(election) : t("eventsScreen.fields.electionId")
                }}
                filter={outcomeFilter}
                onFilter={setOutcomeFilter}
                unpublishedCount={unpublished.size}
                published={publications.length > 0}
                offsetless={
                    (allScheduledEvents ?? []).filter((event) => {
                        const date = (event.cron_config as ICronConfig | undefined)?.scheduled_date
                        return !event.stopped_at && !!date && !hasOffset(date)
                    }).length
                }
                canApply={canWriteScheduledEvent}
                zoneOf={zones.zoneOf}
                scheduledEvents={allScheduledEvents ?? []}
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
                    ...(filteredIds
                        ? {id: {format: "hasura-raw-query", value: {_in: filteredIds}}}
                        : {}),
                }}
                filters={Filters}
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<Empty />}
                actions={
                    <ListActions
                        withColumns={showScheduledEventColumns}
                        withImport={canWriteScheduledEvent}
                        doImport={() => setOpenImport(true)}
                        withExport={canWriteScheduledEvent}
                        doExport={doExport}
                        isExportDisabled={exporting}
                        open={openCreateEvent}
                        setOpen={onOpenDrawer}
                        withAction={canCreateScheduledEvent}
                        doAction={onOpenDrawer}
                        actionLabel="common.label.add"
                    />
                }
                disableSyncWithLocation
            >
                <DatagridConfigurable
                    header={ThreeStateDatagridHeader}
                    bulkActionButtons={false}
                    omit={OMIT_FIELDS}
                >
                    <FilterWatcher
                        fieldName="event_payload"
                        onFilterChange={() => setEventScreenElectionId(null)}
                    />
                    <TextField source="id" />
                    <FunctionField
                        label={String(t("eventsScreen.fields.electionId"))}
                        source="event_payload.election_id"
                        sortable={false}
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
                        label={String(t("eventsScreen.fields.scheduledDate"))}
                        source="cron_config.scheduled_date"
                        sortable={false}
                        render={(record: Sequent_Backend_Scheduled_Event) => {
                            const date = (record.cron_config as ICronConfig | undefined)
                                ?.scheduled_date
                            if (!date) return "-"
                            return (
                                <Stack spacing={0.5} sx={{alignItems: "flex-start"}}>
                                    {hasOffset(date) ? (
                                        <ZonedDateTime instant={date} zone={zoneOfRow(record)} />
                                    ) : (
                                        <>
                                            <span>{date}</span>
                                            <Chip
                                                size="small"
                                                color="warning"
                                                sx={WARNING_TEXT.warning}
                                                label={t("lifecycle.schedule.noOffset")}
                                            />
                                        </>
                                    )}
                                    {unpublished.has(String(record.id)) ? (
                                        <Chip
                                            size="small"
                                            variant="outlined"
                                            label={t("lifecycle.schedule.unpublished")}
                                        />
                                    ) : null}
                                </Stack>
                            )
                        }}
                    />
                    <FunctionField
                        label={String(t("eventsScreen.fields.stoppedAt"))}
                        source="stopped_at"
                        render={(record: Sequent_Backend_Scheduled_Event) => (
                            <ZonedDateTime instant={record.stopped_at} zone={zoneOfRow(record)} />
                        )}
                    />
                    <FunctionField
                        label={String(t("lifecycle.schedule.outcome"))}
                        sortable={false}
                        render={(record: Sequent_Backend_Scheduled_Event) => {
                            const rowOutcomes = outcomes.get(String(record.id))
                            return isVotingTransition(record.event_processor) &&
                                !record.stopped_at &&
                                rowOutcomes?.length ? (
                                <RowOutcome outcomes={rowOutcomes} zone={zoneOfRow(record)} />
                            ) : (
                                "-"
                            )
                        }}
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
            {openImport ? (
                <ImportScheduleDrawer
                    electionEventId={electionEventId}
                    primary={zones.primary}
                    onClose={() => setOpenImport(false)}
                    onImported={() => {
                        setOpenImport(false)
                        refresh()
                    }}
                />
            ) : null}
            {exportDocumentId ? (
                <DownloadDocument
                    documentId={exportDocumentId}
                    electionEventId={electionEventId}
                    fileName={t("lifecycle.schedule.exportFileName")}
                    onDownload={() => setExportDocumentId(null)}
                />
            ) : null}
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
