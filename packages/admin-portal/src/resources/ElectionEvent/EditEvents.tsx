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
    AuthContext,
    DatagridConfigurable,
    FunctionField,
    Identifier,
    List,
    TextInput,
    useDelete,
    useGetList,
    useGetOne,
    useRecordContext,
    useSidebarState,
    WrapperField,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {idCountryTranslations} from "intl-tel-input/i18n"
import {AudienceSelection} from "../User/SendCommunication"
import {Sequent_Backend_Election, Sequent_Backend_Tally_Session} from "@/gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import CreateEvent, {EventProcessors} from "../Events/CreateEvent"
import {Dialog} from "@sequentech/ui-essentials"
import {faPlus} from "@fortawesome/free-solid-svg-icons"

const DataGridContainerStyle = styled(DatagridConfigurable)<{isOpenSideBar?: boolean}>`
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
const EditEvents: React.FC<EditEventsProps> = ({electionEventId}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const record = useRecordContext()
    const [isOpenSidebar] = useSidebarState()
    const [tenantId] = useTenantStore()
    const {tallyId} = useElectionEventTallyStore()
    const [isDeleteModalOpen, setIsDeleteModalOpen] = useState(false)
    const [isDeleteId, setIsDeleteId] = useState<string | undefined>()
    const [deleteOne] = useDelete()
    const [openCreateEvent, setOpenCreateEvent] = useState(false)

    const [recordIds, setRecordIds] = useState<Array<Identifier>>([])

    const {data: tally} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: elections} = useGetList<Sequent_Backend_Election>("sequent_backend_election")

    const getElectionName = (election: any) => {
        const electionName = elections?.find((item) => election?.election === item.id)?.name
        return election.election ? electionName : "-"
    }

    const OMIT_FIELDS: Array<string> = ["election", "email_verified"]

    const Filters: Array<ReactElement> = [
        <TextInput key="Election" source="election" />,
        <TextInput key="Event Type" source="event_type" />,
        <TextInput key="Schedule" source="schedule" />,
    ]

    const sendCommunicationForIdAction = (id: Identifier) => {
        console.log(idCountryTranslations)
    }

    const editAction = (id: Identifier) => {
        setOpenCreateEvent(false)
        setRecordIds([id as string])
    }

    const handleClose = () => {
        setOpenCreateEvent(false)
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

    // const BulkActions = (props: any) => {
    //     return (
    //         <>
    //             {/* {canSendCommunications && ( */}
    //             <Button
    //                 variant="actionbar"
    //                 key="send-notification"
    //                 onClick={() => {
    //                     sendCommunicationAction(props.selectedIds ?? [], AudienceSelection.SELECTED)
    //                 }}
    //             >
    //                 <ResourceListStyles.MailIcon />
    //                 {t(`sendCommunication.send`)}
    //             </Button>
    //             {/* )} */}

    //             {/* {canEditUsers && (
    //                 // <Button
    //                 //     variant="actionbar"
    //                 //     onClick={() => {
    //                 //         setSelectedIds(props.selectedIds)
    //                 //         setOpenDeleteBulkModal(true)
    //                 //     }}
    //                 // >
    //                 //     <ResourceListStyles.DeleteIcon />
    //                 //     {t("common.label.delete")}
    //                 // </Button>
    //             )} */}
    //         </>
    //     )
    // }

    const actions: Action[] = [
        {
            icon: <EditIcon className="edit-voter-icon" />,
            action: (id) => editAction(id),
            showAction: () => {
                return true
            },
        },
        {
            icon: <DeleteIcon className="delete-voter-icon" />,
            action: (id) => deleteAction(id),
            showAction: () => true,
            label: t(`common.label.delete`),
            className: "delete-voter-icon",
        },
    ]

    const filterObject: {[key: string]: any} = {
        election_event_id: record?.id || undefined,
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
                resource="event_list"
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<Empty />}
                actions={
                    <ListActions
                        withImport
                        // doImport={handleImport}
                        withExport
                        // doExport={handleExport}
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
                filter={{
                    tenant_id: tenantId,
                    election_event_id: record.id,
                }}
                // aside={aside}
                filters={Filters}
            >
                <DataGridContainerStyle
                    // bulkActionButtons={<BulkActions />}
                    isOpenSideBar={isOpenSidebar}
                    omit={OMIT_FIELDS}
                >
                    <FunctionField label={"Election"} source="election" render={getElectionName} />
                    <FunctionField
                        label={"event_type"}
                        source="event_type"
                        render={(record: {event_type: keyof typeof EventProcessors}) =>
                            EventProcessorsToLabel[record.event_type]
                        }
                    />
                    <FunctionField
                        label={"Schedule"}
                        source="schedule"
                        render={(record: any) => new Date(record.schedule).toLocaleString()}
                    />
                    <WrapperField label="Actions" source="actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DataGridContainerStyle>
            </List>
            <ResourceListStyles.Drawer anchor="right" open={openCreateEvent} onClose={handleClose}>
                <CreateEvent
                    electionEventId={electionEventId}
                    setIsOpenDrawer={setOpenCreateEvent}
                    elections={elections}
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
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.delete.body`)}
            </Dialog>
        </>
    )
}

export default EditEvents
