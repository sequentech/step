// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {Dialog, isString} from "@sequentech/ui-essentials"
import React, {useContext, useState} from "react"
import {
    Datagrid,
    DatagridConfigurable,
    EditButton,
    Identifier,
    List,
    RecordContextProvider,
    SaveButton,
    SelectInput,
    SimpleForm,
    TextField,
    TextInput,
    Toolbar,
    WrapperField,
    required,
    useNotify,
    useRecordContext,
    useUpdate,
} from "react-admin"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import {Sequent_Backend_Election_Event_Extended} from "./EditElectionEventDataForm"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {Drawer, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {ListActions} from "@/components/ListActions"

const EditElectionEventTextDataTable = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event_Extended>()
    const [update, {isLoading}] = useUpdate()

    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const notify = useNotify()
    const canEdit = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )

    const [selectedLanguage, setSelectedLanguage] = useState<string>(
        record?.presentation?.language_conf?.default_language_code || "en"
    )
    const [openEdit, setOpenEdit] = useState(false)
    const [openCreate, setOpenCreate] = useState(false)
    const [openDeleteModal, setOpenDeleteModal] = useState(false)
    const [deleteId, setDeleteId] = useState<Identifier | null>(null)
    const [recordId, setRecordId] = useState<Identifier | null>(null)

    const languageOptions = record?.presentation?.language_conf?.enabled_language_codes?.map(
        (lang: string) => ({
            id: lang,
            name: lang,
        })
    )
    const handleLanguageChange = (event: any) => {
        const value = event.target ? event.target.value : event
        if (!isString(value) || !value) return
        setSelectedLanguage(value)
    }
    const translationData = Object.entries(
        record?.presentation?.i18n?.[selectedLanguage] || {}
    ).map(([key, value]) => ({
        id: key,
        value: value,
    }))

    
    const editAction = (id: Identifier) => {
        setOpenEdit(true)
        setRecordId(id)
    }
    const deleteAction = (id: Identifier) => {
        setOpenDeleteModal(true)
        setDeleteId(id)
    }

    const handleCloseEditDrawer = () => {
        setRecordId(null)
        setOpenEdit(false)
    }
    const handleCreateText = (e: any) => {
        if (!e || !e?.presentation || !e?.presentation?.i18n) return
        const newKey: string = e?.presentation?.i18n?.[selectedLanguage]?.newKey ?? ""
        const newValue: string = e?.presentation?.i18n?.[selectedLanguage]?.newVal ?? ""
        if (!newValue || !newValue) return
        update(
            "sequent_backend_election_event",
            {
                id: record.id,
                data: {
                    ...record,
                    presentation: {
                        ...record.presentation,
                        i18n: {
                            ...record.presentation.i18n,
                            [selectedLanguage]: {
                                ...record.presentation.i18n?.[selectedLanguage],
                                [newKey]: newValue,
                            },
                        },
                    },
                },
                previousData: record,
            },
            {
                onError() {
                    notify("Translations updates failed", {type: "warning"})
                    setOpenCreate(false)
                },
                onSuccess() {
                    notify("Translations updated successfully", {type: "success"})
                    setOpenCreate(false)
                },
            }
        )
    }
    const handleEditText = (e:any) =>{
        if (!e || !recordId) return
        const editVal: string = e?.editableVal ?? ""
        if (!editVal) return
        update(
            "sequent_backend_election_event",
            {
                id: record.id,
                data: {
                    ...record,
                    presentation: {
                        ...record.presentation,
                        i18n: {
                            ...record.presentation.i18n,
                            [selectedLanguage]: {
                                ...record.presentation.i18n?.[selectedLanguage],
                                [recordId as string]: editVal,
                            },
                        },
                    },
                },
                previousData: record,
            },
            {
                onError() {
                    notify("Translations updates failed", {type: "warning"})
                    handleCloseEditDrawer()
                },
                onSuccess() {
                    notify("Translations updated successfully", {type: "success"})
                    handleCloseEditDrawer()
                },
            }
        )
    }
    const confirmDeleteAction = () => {
        if(!deleteId || !selectedLanguage) return
        const updatedI18nForLanguage = { ...record.presentation.i18n[selectedLanguage] }
        delete updatedI18nForLanguage[deleteId as string]

        update(
            "sequent_backend_election_event",
            {
                id: record.id,
                data: {
                    ...record,
                    presentation: {
                        ...record.presentation,
                        i18n: {
                            ...record.presentation.i18n,
                            [selectedLanguage]: updatedI18nForLanguage,
                        },
                    },
                },
                previousData: record,
            },
            {
                onError() {
                    notify("Translations updates failed", {type: "warning"})
                    handleCloseEditDrawer()
                },
                onSuccess() {
                    notify("Translations updated successfully", {type: "success"})
                    handleCloseEditDrawer()
                },
            }
        )
    }
   

    const actions: Action[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    if (!languageOptions) {
        return (
            <>
                <Typography variant="h4" paragraph>
                    {t("areas.empty.header")}
                </Typography>
            </>
        )
    }

    return (
        <>
            <SimpleForm toolbar={false}>
                <SelectInput
                    source="selectedLanguage"
                    choices={languageOptions}
                    translateChoice={false}
                    defaultValue={selectedLanguage}
                    onChange={handleLanguageChange}
                    optionText="name"
                    optionValue="id"
                    validate={required()}
                    label="Select Language" //TODO: Place in translations file
                />
                <List
                    actions={
                        <ListActions
                            open={openCreate}
                            setOpen={setOpenCreate}
                            withFilter={false}
                            withColumns={false}
                            withExport={false}
                            withImport={false}
                            Component={
                                <SimpleForm
                                    onSubmit={handleCreateText}
                                    toolbar={<SaveButton sx={{marginInline: "1rem"}} />}
                                >
                                    <>
                                        {/* TODO: Update texts */}
                                        <PageHeaderStyles.Title>
                                            {t("areas.common.title")}
                                        </PageHeaderStyles.Title>
                                        <PageHeaderStyles.SubTitle>
                                            {t("areas.common.subTitle")}
                                        </PageHeaderStyles.SubTitle>

                                        <TextInput
                                            source={`presentation.i18n.${selectedLanguage}.newKey`}
                                            label="Key"
                                        />
                                        <TextInput
                                            source={`presentation.i18n.${selectedLanguage}.newVal`}
                                            label="Value"
                                        />
                                    </>
                                </SimpleForm>
                            }
                        />
                    }
                    sx={{flexGrow: 1, width: "100%"}}
                >
                    <Datagrid
                        data={translationData}
                        total={translationData.length}
                        bulkActionButtons={false}
                    >
                        <TextField source="id" label={"Key"}/>
                        <TextField source="value" />
                        <WrapperField label="Actions">
                            <ActionsColumn actions={actions} />
                        </WrapperField>
                    </Datagrid>
                </List>
            </SimpleForm>

            <Drawer
                anchor="right"
                open={openEdit}
                onClose={handleCloseEditDrawer}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <SimpleForm
                    record={record.presentation.i18n[selectedLanguage]}
                    toolbar={<SaveButton sx={{marginInline: "1rem"}} />}
                    onSubmit={handleEditText}
                >
                    <>
                        {/* TODO: Replace texts */}
                        <PageHeaderStyles.Title>{t("areas.common.title")}</PageHeaderStyles.Title>
                        <PageHeaderStyles.SubTitle>
                            {t("areas.common.subTitle")}
                        </PageHeaderStyles.SubTitle>

                        <TextInput source="editableKey" label={"Key"} defaultValue={recordId??undefined} disabled/>
                        <TextInput source="editableVal" label={"Value"} defaultValue={recordId ? record.presentation.i18n[selectedLanguage][recordId] : undefined}/>
                    </>
                </SimpleForm>
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
                    setDeleteId(null)
                }}
            >
                {t("common.message.delete")}
            </Dialog>
        </>
    )
}

export default EditElectionEventTextDataTable
