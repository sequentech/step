import React, {ReactElement, useContext, useEffect} from "react"

import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {faPlus} from "@fortawesome/free-solid-svg-icons"

import {Box, Button, Drawer, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"

import {List, TextField, TextInput, useDelete, Identifier, DatagridConfigurable} from "react-admin"

import {IPermissions} from "@/types/keycloak"
import {Dialog} from "@sequentech/ui-essentials"
import {ListActions} from "@/components/ListActions"
import {IconButton} from "@sequentech/ui-essentials"
import {ActionsColumn} from "@/components/ActionButons"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {CommunicationTemplateCreate} from "./CommunicationTemplateCreate"
import {CommunicationTemplateEdit} from "./CommunicationTemplateEdit"

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

export const CommunicationTemplateList: React.FC = () => {
    const {t} = useTranslation()
    const [deleteOne] = useDelete()
    const {canWriteTenant} = useActionPermissions()

    const [open, setOpen] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)

    const handleCloseCreateDrawer = () => {
        setRecordId(undefined)
        setOpenDrawer(false)
        setOpen(false)
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
        }, 400)
    }

    const editAction = (id: Identifier) => {
        setRecordId(id)
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
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    const CreateButton = () => (
        <Button onClick={handleOpenCreateDrawer}>
            <IconButton icon={faPlus} fontSize="24px" />
            {t("electionTypeScreen.common.createNew")}
        </Button>
    )

    const Empty = () => (
        <EmptyBox m={1}>
            <Typography variant="h4" paragraph>
                {t("electionTypeScreen.common.emptyHeader")}
            </Typography>
            {canWriteTenant ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("electionTypeScreen.common.emptyBody")}
                    </Typography>
                    <CreateButton />
                </>
            ) : null}
        </EmptyBox>
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
                        Component={<CommunicationTemplateCreate close={handleCloseCreateDrawer} />}
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
                onClose={handleCloseEditDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <CommunicationTemplateEdit id={recordId} close={handleCloseEditDrawer} />
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
