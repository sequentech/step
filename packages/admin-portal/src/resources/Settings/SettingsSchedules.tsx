// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useEffect} from "react"

import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {faPlus} from "@fortawesome/free-solid-svg-icons"

import {Box, Button, Drawer, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"

import {
    List,
    TextField,
    TextInput,
    useDelete,
    Identifier,
    DatagridConfigurable,
    useRecordContext,
    useListContext,
    useGetList,
    useList,
    RaRecord,
    ListContextProvider,
    Datagrid,
    useUpdate,
    useGetOne,
    useNotify,
} from "react-admin"

import {ITenantScheduledEvent, ITenantSettings} from "@sequentech/ui-core"
import {Dialog, IconButton} from "@sequentech/ui-essentials"
import {ListActions} from "@/components/ListActions"
import {ActionsColumn} from "@/components/ActionButons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

import {SettingsSchedulesEdit} from "./SettingsSchedulesEdit"
import {SettingsSchedulesCreate} from "./SettingsSchedulesCreate"
import {ISchedule} from "./constants"
import {Sequent_Backend_Tenant} from "@/gql/graphql"

const EmptyBox = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    width: 100%;
`

const useActionPermissions = () => {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)

    const canWriteTenant = authContext.isAuthorized(true, tenantId, IPermissions.TENANT_WRITE)

    return {
        canWriteTenant,
    }
}

const OMIT_FIELDS = ["id", "ballot_eml"]
const Filters: Array<ReactElement> = [<TextInput label="Name" source="name" key={0} />]

export const SettingsSchedules: React.FC = () => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const {canWriteTenant} = useActionPermissions()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [scheduleData, setScheduleData] = React.useState<Array<ITenantScheduledEvent>>([])
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)

    const {data, isLoading, refetch} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
        id: tenantId,
    })
    const [update, {isLoading: isLoadingDelete}] =
        useUpdate<Sequent_Backend_Tenant>("sequent_backend_tenant")
    const listContext = useList({data: scheduleData})

    useEffect(() => {
        let settings = data?.settings as ITenantSettings | undefined
        const temp = settings?.schedules ?? []
        setScheduleData(temp)
    }, [data])

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setOpenDrawer(false)
        setOpen(false)
        refetch()
    }

    const handleOpenCreateDrawer = () => {
        setRecordId(undefined)
        setOpenDrawer(true)
        setOpen(false)
    }

    const handleCloseEditDrawer = () => {
        setOpen(false)
        setTimeout(() => {
            setRecordId(undefined)
            refetch()
        }, 400)
    }

    const editAction = (id: Identifier) => {
        console.log("record editAction", id)

        setRecordId(id)
    }

    const deleteAction = (id: Identifier) => {
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const confirmDeleteAction = () => {
        const filteredData = scheduleData.filter((s) => s.id !== deleteId)
        let settings = data?.settings as ITenantSettings | undefined
        const sendData = {
            ...data,
            settings: {
                ...settings,
                schedules: filteredData,
            },
        }

        update(
            "sequent_backend_tenant",
            {
                id: tenantId,
                data: sendData,
            },
            {
                onSuccess: () => {
                    notify(t("scheduleScreen.deleteScheduleSuccess"), {type: "success"})
                    setDeleteId(undefined)
                    refetch()
                },
                onError: (error) => {
                    notify(t("scheduleScreen.deleteScheduleError"), {type: "error"})
                    setDeleteId(undefined)
                    refetch()
                },
            }
        )
    }

    const actions: any[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    const CreateButton = () => (
        <Button onClick={handleOpenCreateDrawer}>
            <IconButton icon={faPlus} fontSize="24px" />
            {t("scheduleScreen.common.createNew")}
        </Button>
    )

    const Empty = () => (
        <EmptyBox m={1}>
            <Typography variant="h4" paragraph>
                {t("scheduleScreen.common.emptyHeader")}
            </Typography>
            {canWriteTenant ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("scheduleScreen.common.emptyBody")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </EmptyBox>
    )

    if (!canWriteTenant) {
        return <Empty />
    }

    return (
        <>
            <ListContextProvider value={listContext}>
                {scheduleData.length > 0 && (
                    <Box display="flex" justifyContent="flex-end" mb={2}>
                        <ListActions
                            custom
                            withFilter={false}
                            withImport={false}
                            withExport={false}
                            withColumns={false}
                            open={openDrawer}
                            setOpen={setOpenDrawer}
                            Component={<SettingsSchedulesCreate close={handleCloseCreateDrawer} />}
                        />
                    </Box>
                )}

                <Datagrid
                    empty={<Empty />}
                    bulkActionButtons={false}
                    sx={{
                        "& .column-name": {width: "70%"},
                        "& .column-undefined": {textAlign: "center"},
                    }}
                >
                    <TextField source="name" />
                    <TextField source="date" />
                    <ActionsColumn actions={actions} />
                </Datagrid>
            </ListContextProvider>

            <Drawer
                anchor="right"
                open={openDrawer}
                onClose={handleCloseCreateDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <SettingsSchedulesCreate close={handleCloseCreateDrawer} />
            </Drawer>

            <Drawer
                anchor="right"
                open={open}
                onClose={handleCloseEditDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <SettingsSchedulesEdit id={recordId} close={handleCloseEditDrawer} />
            </Drawer>

            <Dialog
                variant="warning"
                open={openDeleteModal}
                ok={t("common.label.delete")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteAction()
                    }
                    setOpenDeleteModal(false)
                }}
            >
                {t("common.message.delete")}
            </Dialog>
        </>
    )
}
