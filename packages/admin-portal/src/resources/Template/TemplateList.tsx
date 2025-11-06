// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useContext, useState} from "react"

import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {faPlus} from "@fortawesome/free-solid-svg-icons"

import {Box, Button, Drawer, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"

import {
    List,
    TextField,
    useDelete,
    Identifier,
    DatagridConfigurable,
    useRefresh,
    WrapperField,
    Button as ReactAdminButton,
    useNotify,
} from "react-admin"

import {IPermissions} from "@/types/keycloak"
import {ListActions} from "@/components/ListActions"
import UploadIcon from "@mui/icons-material/Upload"
import {ActionsColumn} from "@/components/ActionButons"
import {AuthContext} from "@/providers/AuthContextProvider"
import {Dialog, IconButton} from "@sequentech/ui-essentials"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {TemplateCreate} from "./TemplateCreate"
import {CustomApolloContextProvider} from "@/providers/ApolloContextProvider"
import ElectionHeader from "@/components/ElectionHeader"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {TemplateEdit} from "./TemplateEdit"
import {useMutation} from "@apollo/client"
import {EXPORT_TEMPLATE} from "@/queries/ExportTemplate"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../User/DownloadDocument"
import {ExportTemplateMutation, ImportTemplatesMutation} from "@/gql/graphql"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {IMPORT_TEMPLATES} from "@/queries/ImportTemplate"
import {EIntegrityCheckError} from "@/types/templates"

const TemplateEmpty = styled(Box)`
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

export const TemplateList: React.FC = () => {
    const {t} = useTranslation()
    const [deleteOne] = useDelete()
    const {canWriteTenant} = useActionPermissions()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const templateRead = authContext.isAuthorized(true, tenantId, IPermissions.template_READ)
    const templateWrite = authContext.isAuthorized(true, tenantId, IPermissions.template_WRITE)
    const [openExport, setOpenExport] = useState(false)
    const [exporting, setExporting] = useState(false)
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const [openImportDrawer, setOpenImportDrawer] = useState<boolean>(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [deleteId, setDeleteId] = React.useState<Identifier | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [recordId, setRecordId] = React.useState<Identifier | undefined>(undefined)
    const [ExportTemplate] = useMutation<ExportTemplateMutation>(EXPORT_TEMPLATE)
    const [ImportTemplate] = useMutation<ImportTemplatesMutation>(IMPORT_TEMPLATES)
    const refresh = useRefresh()
    const notify = useNotify()

    const handleExport = async () => {
        setExporting(false)
        setExportDocumentId(undefined)
        setOpenExport(true)
    }

    const confirmExportAction = async () => {
        try {
            setExporting(true)
            const {data, errors} = await ExportTemplate({variables: {tenantId}})
            notify("Templates exported successfully", {type: "success"})
            if (errors) {
                setExporting(false)
                notify("Error exporting templates", {type: "error"})
                return
            }
            const documentId = data?.export_template?.document_id
            setExportDocumentId(documentId)
        } catch (error) {
            console.log(error)
            notify("Error exporting templates", {type: "error"})
        }
    }

    const handleCloseDrawer = () => {
        setOpenDrawer(false)
        refresh()

        setTimeout(() => {
            setRecordId(undefined)
        }, 400)
    }

    const handleImport = () => {
        setOpenImportDrawer(true)
    }

    const handleCreateDrawer = () => {
        setRecordId(undefined)
        setOpenDrawer(true)
    }

    const handleEditDrawer = (id: Identifier) => {
        setRecordId(id)
        setOpenDrawer(true)
    }

    const deleteAction = (id: Identifier) => {
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const confirmDeleteAction = () => {
        deleteOne("sequent_backend_template", {id: deleteId})
        setDeleteId(undefined)
    }

    const actions: any[] = [
        {icon: <EditIcon />, action: handleEditDrawer},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    const CreateButton = () => (
        <Button onClick={handleCreateDrawer}>
            <IconButton icon={faPlus} fontSize="24px" />
            {t("template.action.createOne")}
        </Button>
    )

    const handleImportTemplates = async (documentId: string, sha256: string) => {
        setOpenImportDrawer(false)
        try {
            const {data, errors} = await ImportTemplate({
                variables: {
                    tenantId,
                    documentId,
                    sha256,
                },
            })
            let errMsg = data?.import_templates?.error_msg
            if (errMsg) {
                let errType = errMsg as EIntegrityCheckError
                if (errType == EIntegrityCheckError.HASH_MISSMATCH) {
                    notify(t("importResource.ImportHashMismatch"), {type: "error"})
                } else {
                    notify("Error importing templates", {type: "error"})
                }
                return
            }
            notify("Templates imported successfully", {type: "success"})
            refresh()
        } catch (err) {
            console.log(err)
            notify("Error importing templates", {type: "error"})
        }
    }

    const Empty = () => (
        <TemplateEmpty m={1}>
            <Typography variant="h4" paragraph>
                {t("template.empty.title")}
            </Typography>

            {templateWrite ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("template.empty.subtitle")}
                    </Typography>

                    <ResourceListStyles.EmptyButtonList>
                        <CreateButton />
                        <ReactAdminButton onClick={handleImport} label={t("common.label.import")}>
                            <UploadIcon />
                        </ReactAdminButton>
                    </ResourceListStyles.EmptyButtonList>
                </>
            ) : null}
        </TemplateEmpty>
    )

    const showTemplatesMenu = authContext.isAuthorized(true, tenantId, IPermissions.TEMPLATES_MENU)

    if (!templateRead || !showTemplatesMenu) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {t("template.noPermissions")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    if (!canWriteTenant) {
        return <Empty />
    }

    return (
        <>
            <ElectionHeader title={t("template.title")} subtitle={t("template.subtitle")} />

            <List
                resource="sequent_backend_template"
                filters={Filters}
                actions={
                    <ListActions
                        custom
                        withFilter
                        doImport={handleImport}
                        withExport={true}
                        doExport={handleExport}
                        withImport={true}
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={<TemplateCreate close={handleCloseDrawer} />}
                        withComponent={templateWrite}
                    />
                }
                empty={<Empty />}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="template.alias" label="Alias" />
                    <TextField source="template.name" label="Name" />
                    <TextField source="type" />
                    <WrapperField source="actions" label="Actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DatagridConfigurable>
            </List>

            <Drawer
                anchor="right"
                open={openDrawer}
                onClose={handleCloseDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                {recordId ? (
                    <TemplateEdit id={recordId} close={handleCloseDrawer} />
                ) : (
                    <CustomApolloContextProvider role={IPermissions.template_WRITE}>
                        <TemplateCreate close={handleCloseDrawer} />
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
            <Dialog
                variant="info"
                open={openExport}
                ok={t("common.label.export")}
                okEnabled={() => !exporting}
                cancel={t("common.label.cancel")}
                title={t("common.label.export")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    } else {
                        setExportDocumentId(undefined)
                        setExporting(false)
                        setOpenExport(false)
                    }
                }}
            >
                {t("common.export")}
                <FormStyles.ReservedProgressSpace>
                    {exporting ? <FormStyles.ShowProgress /> : null}
                    {exporting && exportDocumentId ? (
                        <DownloadDocument
                            documentId={exportDocumentId}
                            fileName={`templates-export.csv`}
                            onDownload={() => {
                                console.log("onDownload called")
                                setExportDocumentId(undefined)
                                setExporting(false)
                                setOpenExport(false)
                            }}
                        />
                    ) : null}
                </FormStyles.ReservedProgressSpace>
            </Dialog>

            <ImportDataDrawer
                open={openImportDrawer}
                closeDrawer={() => setOpenImportDrawer(false)}
                title="template.import.title"
                subtitle="template.import.subtitle"
                paragraph="template.import.paragraph"
                doImport={handleImportTemplates}
                errors={null}
            />
        </>
    )
}
