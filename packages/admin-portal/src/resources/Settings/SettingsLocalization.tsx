// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Dialog} from "@sequentech/ui-essentials"
import {ILanguageConf, isString, ITenantSettings} from "@sequentech/ui-core"
import React, {useEffect, useMemo, useState} from "react"
import {
    Button,
    Datagrid,
    Identifier,
    List,
    SaveButton,
    SimpleForm,
    TextField,
    TextInput,
    WrapperField,
    useEditController,
    useNotify,
    useRecordContext,
    useUpdate,
} from "react-admin"
import EditIcon from "@mui/icons-material/Edit"
import Add from "@mui/icons-material/Add"
import DeleteIcon from "@mui/icons-material/Delete"
// import { Sequent_Backend_Election_Event_Extended } from "./EditElectionEventDataForm"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {
    Box,
    Drawer,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    SelectChangeEvent,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {Sequent_Backend_Tenant} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"

const SettingsLocalization = () => {
    // const record = useRecordContext<Sequent_Backend_Tenant>()
    const [tenantId] = useTenantStore()

    const {
        record,
        save,
        isLoading: recordLoading,
    } = useEditController<Sequent_Backend_Tenant, undefined>({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const defaultLanguageConf: ILanguageConf = {
        enabled_language_codes: ["en"],
        default_language_code: "en",
    }

    const [languageConf, setLanguageConf] = useState<ILanguageConf>(
        (record?.settings as ITenantSettings | undefined)?.language_conf ?? defaultLanguageConf
    )

    const [update, {isLoading}] = useUpdate()
    console.log({record})

    const {t} = useTranslation()
    const notify = useNotify()

    const [selectedLanguage, setSelectedLanguage] = useState<string>(
        languageConf?.default_language_code ?? "en"
    )
    const [openEdit, setOpenEdit] = useState(false)
    const [openCreate, setOpenCreate] = useState(false)
    const [openDeleteModal, setOpenDeleteModal] = useState(false)
    const [deleteId, setDeleteId] = useState<Identifier | null>(null)
    const [recordId, setRecordId] = useState<Identifier | null>(null)

    const languageOptions = useMemo(() => {
        return (languageConf?.enabled_language_codes ?? []) as string[]
    }, [languageConf?.enabled_language_codes])

    const handleLanguageChange = (event: SelectChangeEvent<string>) => {
        const value = event?.target?.value ?? ""
        if (!isString(value) || !value) return
        setSelectedLanguage(value)
    }
    const translationData = Object.entries(
        (record?.settings as ITenantSettings | undefined)?.i18n?.[selectedLanguage] || {}
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
            "sequent_backend_tenant",
            {
                id: record?.id,
                data: {
                    ...record,
                    settings: {
                        ...(record?.settings ?? {}),
                        i18n: {
                            ...((record?.settings as ITenantSettings | undefined)?.i18n ?? {}),
                            [selectedLanguage]: {
                                ...((record?.settings as ITenantSettings | undefined)?.i18n?.[
                                    selectedLanguage
                                ] ?? {}),
                                [newKey]: newValue,
                            },
                        },
                    },
                },
                previousData: record,
            },
            {
                onError() {
                    notify(t("electionEventScreen.localization.notify.error"), {type: "error"})
                    setOpenCreate(false)
                },
                onSuccess() {
                    notify(t("electionEventScreen.localization.notify.success"), {type: "success"})
                    setOpenCreate(false)
                },
            }
        )
    }
    const handleEditText = (e: any) => {
        console.log({e})
        if (!e || !recordId) return
        const editVal: string = e?.editableVal ?? ""
        if (!editVal) return
        update(
            "sequent_backend_tenant",
            {
                id: record?.id,
                data: {
                    ...record,
                    settings: {
                        ...record?.settings,
                        i18n: {
                            ...(record?.settings as ITenantSettings | undefined)?.i18n,
                            [selectedLanguage]: {
                                ...(record?.settings as ITenantSettings | undefined)?.i18n?.[
                                    selectedLanguage
                                ],
                                [recordId as string]: editVal,
                            },
                        },
                    },
                },
                previousData: record,
            },
            {
                onError() {
                    notify(t("electionEventScreen.localization.notify.error"), {type: "error"})
                    handleCloseEditDrawer()
                },
                onSuccess() {
                    notify(t("electionEventScreen.localization.notify.success"), {type: "success"})
                    handleCloseEditDrawer()
                },
            }
        )
    }
    const confirmDeleteAction = () => {
        if (!deleteId || !selectedLanguage) return
        const updatedI18nForLanguage = {
            ...(record?.settings as ITenantSettings | undefined)?.i18n?.[selectedLanguage],
        }
        delete updatedI18nForLanguage[deleteId as string]

        update(
            "sequent_backend_tenant",
            {
                id: record?.id,
                data: {
                    ...record,
                    settings: {
                        ...(record?.settings as ITenantSettings | undefined),
                        i18n: {
                            ...(record?.settings as ITenantSettings | undefined)?.i18n,
                            [selectedLanguage]: updatedI18nForLanguage,
                        },
                    },
                },
                previousData: record,
            },
            {
                onError() {
                    notify(t("electionEventScreen.localization.notify.error"), {type: "error"})
                    handleCloseEditDrawer()
                },
                onSuccess() {
                    notify(t("electionEventScreen.localization.notify.success"), {type: "success"})
                    handleCloseEditDrawer()
                },
            }
        )
    }

    const actions: Action[] = [
        {icon: <EditIcon />, action: editAction},
        {icon: <DeleteIcon />, action: deleteAction},
    ]

    if (!languageOptions || !selectedLanguage) {
        return (
            <>
                <Typography variant="h4" paragraph>
                    {t("electionEventScreen.localization.emptyHeader")}
                </Typography>
            </>
        )
    }

    return (
        <>
            <SimpleForm toolbar={false}>
                <Box
                    sx={{
                        flexGrow: 1,
                        display: "flex",
                        alignItems: "center",
                        width: "100%",
                        justifyContent: "space-between",
                    }}
                >
                    <FormControl key="select-language" sx={{width: "50%"}}>
                        <InputLabel id="select-language">
                            {t("electionEventScreen.localization.selectLanguage")}
                        </InputLabel>
                        <Select
                            labelId="select-language"
                            fullWidth
                            label={String(t("electionEventScreen.localization.selectLanguage"))}
                            onChange={handleLanguageChange}
                            value={selectedLanguage}
                        >
                            {languageOptions &&
                                languageOptions.map((lang) => {
                                    return (
                                        <MenuItem key={lang} value={lang}>
                                            {t(`common.language.${lang}`)}
                                        </MenuItem>
                                    )
                                })}
                        </Select>
                    </FormControl>
                    <div className="list-actions">
                        <Button
                            onClick={() => setOpenCreate(true)}
                            label={String(t("common.label.add"))}
                        >
                            <Add />
                        </Button>

                        <Drawer
                            anchor="right"
                            open={openCreate}
                            onClose={() => {
                                setOpenCreate(false)
                            }}
                            PaperProps={{
                                sx: {width: "30%"},
                            }}
                        >
                            <SimpleForm
                                onSubmit={handleCreateText}
                                toolbar={<SaveButton sx={{marginInline: "1rem"}} />}
                            >
                                <>
                                    <PageHeaderStyles.Title>
                                        {t("electionEventScreen.localization.common.title")}
                                    </PageHeaderStyles.Title>
                                    <PageHeaderStyles.SubTitle>
                                        {t("electionEventScreen.localization.common.subTitle")}
                                    </PageHeaderStyles.SubTitle>

                                    <TextInput
                                        source={`presentation.i18n.${selectedLanguage}.newKey`}
                                        label={String(
                                            t("electionEventScreen.localization.labels.key")
                                        )}
                                    />
                                    <TextInput
                                        source={`presentation.i18n.${selectedLanguage}.newVal`}
                                        label={String(
                                            t("electionEventScreen.localization.labels.value")
                                        )}
                                        multiline
                                    />
                                </>
                            </SimpleForm>
                        </Drawer>
                    </div>
                </Box>
                <List actions={false} sx={{flexGrow: 1, width: "100%"}}>
                    <Datagrid
                        data={translationData}
                        total={translationData.length}
                        bulkActionButtons={false}
                    >
                        <TextField
                            source="id"
                            label={String(t("electionEventScreen.localization.labels.key"))}
                        />
                        <TextField
                            source="value"
                            label={String(t("electionEventScreen.localization.labels.value"))}
                        />
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
                    record={
                        (record?.settings as ITenantSettings | undefined)?.i18n?.[
                            selectedLanguage
                        ] || {}
                    }
                    toolbar={<SaveButton sx={{marginInline: "1rem"}} />}
                    onSubmit={handleEditText}
                >
                    <>
                        <PageHeaderStyles.Title>
                            {t("electionEventScreen.localization.common.title")}
                        </PageHeaderStyles.Title>
                        <PageHeaderStyles.SubTitle>
                            {t("electionEventScreen.localization.common.subTitle")}
                        </PageHeaderStyles.SubTitle>

                        <TextInput
                            source="editableKey"
                            label={String(t("electionEventScreen.localization.labels.key"))}
                            defaultValue={recordId ?? undefined}
                            disabled
                        />
                        <TextInput
                            source="editableVal"
                            label={String(t("electionEventScreen.localization.labels.value"))}
                            defaultValue={
                                recordId
                                    ? (record?.settings as ITenantSettings | undefined)?.i18n?.[
                                          selectedLanguage
                                      ][recordId]
                                    : undefined
                            }
                            multiline
                        />
                    </>
                </SimpleForm>
            </Drawer>

            <Dialog
                variant="warning"
                open={openDeleteModal}
                ok={String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteAction()
                        // console.log('handle close')
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

export default SettingsLocalization
