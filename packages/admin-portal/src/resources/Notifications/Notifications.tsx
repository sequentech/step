// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {FC, ReactElement, useContext} from "react"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Button, Typography} from "@mui/material"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {FunctionField, List, TextInput, useRecordContext, WrapperField} from "react-admin"
import {useTranslation} from "react-i18next"
import {ListActions} from "@/components/ListActions"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {DataGridContainerStyle} from "../ScheduledEvents/ListScheduledEvent"
import {useTenantStore} from "@/providers/TenantContextProvider"

interface notificationsProps {
    electionEventId: string
}
const Notifications: FC<notificationsProps> = ({electionEventId}) => {
    const {globalSettings} = useContext(SettingsContext)
    const {t} = useTranslation()
    const record = useRecordContext()
    const [tenantId] = useTenantStore()

    const Filters: Array<ReactElement> = [
        <TextInput key="Election" source="election" />,
        <TextInput key="Event Type" source="event_type" />,
        <TextInput key="Schedule" source="schedule" />,
    ]

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t(`eventsScreen.empty.header`)}
            </Typography>
            <Typography variant="body1" paragraph>
                {t(`eventsScreen.empty.body`)}
            </Typography>
            <ResourceListStyles.EmptyButtonList className="voter-add-button">
                <Button onClick={() => console.log("test")}>
                    <ResourceListStyles.CreateIcon icon={faPlus} />
                    {t(`eventsScreen.empty.button`)}
                </Button>
            </ResourceListStyles.EmptyButtonList>
        </ResourceListStyles.EmptyBox>
    )

    const actions: Action[] = [
        // {
        //     icon: <EditIcon className="edit-voter-icon" />,
        //     action: (id) => editAction(id),
        //     showAction: () => canEdit,
        // },
        // {
        //     icon: <DeleteIcon className="delete-voter-icon" />,
        //     action: (id) => deleteAction(id),
        //     showAction: () => canCreate,
        //     label: t(`common.label.delete`),
        //     className: "delete-voter-icon",
        // },
    ]

    const filterObject: {[key: string]: any} = {
        election_event_id: record?.id || undefined,
        tenant_id: tenantId,
    }
    return (
        <>
            <List
                resource="sequent_backend_notification"
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
                        // open={openCreateEvent}
                        // setOpen={onOpenDrawer}
                        // Component={

                        //     <CreateEvent
                        //         electionEventId={electionEventId}
                        //         setIsOpenDrawer={setOpenCreateEvent}
                        //         elections={elections}
                        //     />
                        // }
                    />
                }
                filter={filterObject}
                filters={Filters}
            >
                <DataGridContainerStyle
                // bulkActionButtons={<BulkActions />}
                // isOpenSideBar={isOpenSidebar}
                // omit={OMIT_FIELDS}
                >
                    {/* <FunctionField label={"Election"} source="election" render={getElectionName} /> */}
                    {/* <FunctionField
                        label={"Event Type"}
                        source="event_type"
                        render={(record: {event_type: keyof typeof EventProcessors}) =>
                            EventProcessorsToLabel[record.event_type]
                        }
                    /> */}
                    <FunctionField
                        label={"Schedule"}
                        source="schedule"
                        render={(record: any) => new Date(record.schedule).toLocaleString()}
                    />
                    <WrapperField label="Actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DataGridContainerStyle>
            </List>
            <ResourceListStyles.Drawer
                anchor="right"
                // open={openCreateEvent} onClose={handleClose}
            >
                {/* <CreateEvent
                    electionEventId={electionEventId}
                    setIsOpenDrawer={setOpenCreateEvent}
                    elections={elections}
                    isEditEvent={isEditEvent}
                    selectedEventId={selectedEventId}
                /> */}
            </ResourceListStyles.Drawer>
            {/* <Dialog
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
            </Dialog> */}
        </>
    )
}

export default Notifications
