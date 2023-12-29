import React, {ReactElement, useContext, useEffect} from "react"

import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {faPlus} from "@fortawesome/free-solid-svg-icons"

import {Box, Button, Drawer, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"

import {List, TextField, useDelete, Identifier, DatagridConfigurable, useRefresh} from "react-admin"

import {IPermissions} from "@/types/keycloak"
import {ListActions} from "@/components/ListActions"
import {ActionsColumn} from "@/components/ActionButons"
import {AuthContext} from "@/providers/AuthContextProvider"
import {Dialog, IconButton} from "@sequentech/ui-essentials"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {CommunicationTemplateCreate} from "./CommunicationTemplateCreate"
import {CommunicationTemplateEdit} from "./CommunicationTemplateEdit"
import { CustomApolloContextProvider } from "@/providers/ApolloContextProvider"

const CommunicationTemplateEmpty = styled(Box)`
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

const OMIT_FIELDS = ["id"]
const Filters: Array<ReactElement> = []

export const CommunicationTemplateList: React.FC = () => {
    const {t} = useTranslation()
    const [deleteOne] = useDelete()
    const {canWriteTenant} = useActionPermissions()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)
    const refresh = useRefresh()

    const handleCloseDrawer = () => {
        setOpenDrawer(false)
        setOpen(false)
        refresh()

        setTimeout(() => {
            setRecordId(undefined)
        }, 400)
    }

    const handleCreateDrawer = () => {
        setRecordId(undefined)
        setOpenDrawer(true)
        setOpen(true)
    }

    const handleEditDrawer = (id: Identifier) => {
        setRecordId(id)
        setOpenDrawer(true)
        setOpen(true)
    }

    const deleteAction = (id: Identifier) => {
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const confirmDeleteAction = () => {
        deleteOne("sequent_backend_communication_template", {id: deleteId})
        setDeleteId(undefined)
    }

    const actions: any[] = [
        {icon: <EditIcon />, action: handleEditDrawer},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    const CreateButton = () => (
        <Button onClick={handleCreateDrawer}>
            <IconButton icon={faPlus} fontSize="24px" />
            {t("communicationTemplate.action.createOne")}
        </Button>
    )

    const Empty = () => (
        <CommunicationTemplateEmpty m={1}>
            <Typography variant="h4" paragraph>
                {t("communicationTemplate.empty.title")}
            </Typography>

            {canWriteTenant ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("communicationTemplate.empty.subtitle")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </CommunicationTemplateEmpty>
    )

    useEffect(() => {
        if (recordId) {
            setOpen(true)
        }
    }, [recordId])

    if (!canWriteTenant) {
        return <Empty />
    }

    return (
        <>
            <List
                resource="sequent_backend_communication_template"
                filters={Filters}
                actions={
                    <ListActions
                        custom
                        withFilter
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={<CommunicationTemplateCreate close={handleCloseDrawer} />}
                    />
                }
                empty={<Empty />}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />

                    <ActionsColumn actions={actions} />
                </DatagridConfigurable>
            </List>

            <Drawer
                anchor="right"
                open={open}
                onClose={handleCloseDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                {recordId ? (
                    <CommunicationTemplateEdit id={recordId} close={handleCloseDrawer} />
                ) : (
                    <CustomApolloContextProvider role="communication-template-write">
                        <CommunicationTemplateCreate close={handleCloseDrawer} />
                    </CustomApolloContextProvider>
                )}
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
