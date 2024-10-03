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
import {
    DatagridConfigurable,
    FunctionField,
    List,
    TextInput,
    useDelete,
    useGetList,
    useRecordContext,
    useSidebarState,
    WrapperField,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"

import {Sequent_Backend_Election} from "@/gql/graphql"
import CreateEvent, {EventProcessors} from "./CreateScheduledEvent"
import {Dialog} from "@sequentech/ui-essentials"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

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
export enum EventProcessorsToLabel {
    START_ELECTION = "Start Election",
    END_ELECTION = "End Election",
}

interface EditEventsProps {
    electionEventId: string
}
const ListEvents: React.FC<EditEventsProps> = ({electionEventId}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const record = useRecordContext()
    const [isOpenSidebar] = useSidebarState()
    const [tenantId] = useTenantStore()
    const [isDeleteModalOpen, setIsDeleteModalOpen] = useState(false)
    const [isDeleteId, setIsDeleteId] = useState<string | undefined>()
    const [isEditEvent, setIsEditEvent] = useState(false)
    const [deleteOne] = useDelete()
    const [openCreateEvent, setOpenCreateEvent] = useState(false)
    const [selectedEventId, setSelectedEventId] = useState<string | undefined>()
    const [selectedElectionId, setselectedElectionId] = useState<string | undefined>()
    //"6630a00f-74ed-4280-803b-6e9d26485d00"
    const authContext = useContext(AuthContext)

    const {data: elections} = useGetList<Sequent_Backend_Election>("sequent_backend_election")

    const getElectionName = (election: any) => {
        const electionName = elections?.find(
            (item) => election?.event_payload?.election_id === item.id
        )?.name
        return election.id ? electionName : "-"
    }

    const canEdit = authContext.isAuthorized(true, authContext.tenantId, IPermissions.EVENTS_EDIT)
    const canCreate = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.EVENTS_CREATE
    )

    const OMIT_FIELDS: Array<string> = []

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

    const confirmDeleteAction = () => {
        deleteOne("sequent_backend_scheduled_event", {id: isDeleteId?.toString()})
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
            showAction: () => canEdit,
        },
        {
            icon: <DeleteIcon className="delete-voter-icon" />,
            action: (id) => deleteAction(id),
            showAction: () => canCreate,
            label: t(`common.label.delete`),
            className: "delete-voter-icon",
        },
    ]

    const filterObject: {[key: string]: any} = {
        election_event_id: electionEventId || undefined,
        tenant_id: tenantId,
    }

    const onOpenDrawer = () => {
        setOpenCreateEvent(!openCreateEvent)
    }

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t(`eventsScreen.empty.header`)}
            </Typography>
            <Typography variant="body1" paragraph>
                {t(`eventsScreen.empty.body`)}
            </Typography>
            <ResourceListStyles.EmptyButtonList className="voter-add-button">
                <Button onClick={() => setOpenCreateEvent(true)}>
                    <ResourceListStyles.CreateIcon icon={faPlus} />
                    {t(`eventsScreen.empty.button`)}
                </Button>
            </ResourceListStyles.EmptyButtonList>
        </ResourceListStyles.EmptyBox>
    )
    return (
        <>
            <List
                resource="sequent_backend_scheduled_event"
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<Empty />}
                actions={
                    <ListActions
                        withImport={false}
                        withExport={false}
                        open={openCreateEvent}
                        setOpen={onOpenDrawer}
                        Component={
                            <CreateEvent
                                electionEventId={electionEventId}
                                setIsOpenDrawer={setOpenCreateEvent}
                                elections={elections}
                            />
                        }
                    />
                }
                filter={filterObject}
                filters={Filters}
            >
                <DataGridContainerStyle
                    // bulkActionButtons={false}
                    isOpenSideBar={isOpenSidebar}
                    omit={OMIT_FIELDS}
                >
                    <FunctionField
                        label={"Election"}
                        source="event_payload.election_id"
                        render={getElectionName}
                    />
                    <FunctionField
                        label={"Event Type"}
                        source="event_processor"
                        render={(record: {event_processor: keyof typeof EventProcessors}) =>
                            EventProcessorsToLabel[record.event_processor]
                        }
                    />
                    <FunctionField
                        label={"Schedule"}
                        source="cron_config.scheduled_date"
                        render={(record: any) =>
                            new Date(record.cron_config.scheduled_date).toLocaleString()
                        }
                    />
                    <WrapperField label="Actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DataGridContainerStyle>
            </List>
            <ResourceListStyles.Drawer anchor="right" open={openCreateEvent} onClose={handleClose}>
                <CreateEvent
                    electionEventId={electionEventId}
                    setIsOpenDrawer={setOpenCreateEvent}
                    elections={elections}
                    isEditEvent={isEditEvent}
                    selectedEventId={selectedEventId}
                />
            </ResourceListStyles.Drawer>
            <Dialog
                variant="warning"
                open={isDeleteModalOpen}
                ok={t("common.label.delete")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteAction()
                    }
                    setIsDeleteModalOpen(false)
                }}
            >
                {t(`eventsScreen.edit.delete`)}
            </Dialog>
        </>
    )
}

export default ListEvents
