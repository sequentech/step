// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Dialog} from "@sequentech/ui-essentials"
import {
    ETranslationScope,
    ILanguageConf,
    isString,
    isTranslationScope,
    ITenantSettings,
    parseTranslationOverrideKey,
    updateTranslationOverride,
} from "@sequentech/ui-core"
import React, {useEffect, useMemo, useState} from "react"
import {
    Button,
    Datagrid,
    Identifier,
    ListContextProvider,
    Pagination,
    SaveButton,
    SimpleForm,
    TextField,
    TextInput,
    WrapperField,
    useEditController,
    useList,
    useNotify,
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
import {ThreeStateDatagridHeader} from "@/components/ThreeStateDatagridHeader"
import {TranslationScopeInput, translationScopeLabel} from "@/components/TranslationScopeInput"

const TENANT_TRANSLATION_SCOPES = [
    ETranslationScope.GLOBAL,
    ETranslationScope.ADMIN_PORTAL,
] as const

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
    const translationData = useMemo(
        () =>
            Object.entries(
                (record?.settings as ITenantSettings | undefined)?.i18n?.[selectedLanguage] || {}
            ).map(([storedKey, value]) => {
                const {key, scope} = parseTranslationOverrideKey(storedKey)
                return {
                    id: storedKey,
                    key,
                    scope: translationScopeLabel(t, scope, ETranslationScope.ADMIN_PORTAL),
                    value,
                }
            }),
        [record?.settings, selectedLanguage, t]
    )
    const translationListContext = useList({data: translationData, perPage: 25})

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
        if (!e) return
        const newKey: string = e?.newKey ?? ""
        const newValue: string = e?.newVal ?? ""
        const newScope = e?.newScope
        if (!newValue || !newKey || !isTranslationScope(newScope)) return
        const currentTranslations =
            (record?.settings as ITenantSettings | undefined)?.i18n?.[selectedLanguage] ?? {}
        const updatedTranslations = updateTranslationOverride(
            currentTranslations,
            newKey,
            newScope,
            newValue
        )
        if (!updatedTranslations) {
            notify(
                t("electionEventScreen.localization.notify.duplicateKey", {
                    defaultValue: "An override with this key and portal scope already exists.",
                }),
                {type: "error"}
            )
            return
        }
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
                            [selectedLanguage]: updatedTranslations,
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
        if (!e || !recordId) return
        const editVal: string = e?.editableVal ?? ""
        const editKey = parseTranslationOverrideKey(String(recordId)).key
        const editScope = e?.editableScope
        if (!editVal || !editKey || !isTranslationScope(editScope)) return
        const currentTranslations =
            (record?.settings as ITenantSettings | undefined)?.i18n?.[selectedLanguage] ?? {}
        const updatedI18nForLanguage = updateTranslationOverride(
            currentTranslations,
            editKey,
            editScope,
            editVal,
            String(recordId)
        )
        if (!updatedI18nForLanguage) {
            notify(
                t("electionEventScreen.localization.notify.duplicateKey", {
                    defaultValue: "An override with this key and portal scope already exists.",
                }),
                {type: "error"}
            )
            return
        }
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

    const parsedRecordId = parseTranslationOverrideKey(String(recordId ?? ""))
    const editRecord = {
        editableKey: parsedRecordId.key,
        editableScope: parsedRecordId.scope ?? ETranslationScope.ADMIN_PORTAL,
        editableVal: recordId
            ? (record?.settings as ITenantSettings | undefined)?.i18n?.[selectedLanguage]?.[
                  recordId as string
              ]
            : undefined,
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
                                record={{}}
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

                                    <Box sx={{display: "flex", gap: 2, width: "100%"}}>
                                        <TranslationScopeInput
                                            source="newScope"
                                            defaultValue={ETranslationScope.ADMIN_PORTAL}
                                            allowedScopes={TENANT_TRANSLATION_SCOPES}
                                        />
                                        <TextInput
                                            source="newKey"
                                            label={String(
                                                t("electionEventScreen.localization.labels.key")
                                            )}
                                            fullWidth
                                        />
                                    </Box>
                                    <TextInput
                                        source="newVal"
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
                <Box sx={{flexGrow: 1, width: "100%"}}>
                    <ListContextProvider value={translationListContext}>
                        <Datagrid header={ThreeStateDatagridHeader} bulkActionButtons={false}>
                            <TextField
                                source="key"
                                label={String(t("electionEventScreen.localization.labels.key"))}
                            />
                            <TextField
                                source="scope"
                                label={String(
                                    t("electionEventScreen.localization.labels.scope", {
                                        defaultValue: "Portal scope",
                                    })
                                )}
                            />
                            <TextField
                                source="value"
                                label={String(t("electionEventScreen.localization.labels.value"))}
                            />
                            <WrapperField label="Actions">
                                <ActionsColumn actions={actions} />
                            </WrapperField>
                        </Datagrid>
                        <Pagination />
                    </ListContextProvider>
                </Box>
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
                    key={`${selectedLanguage}:${String(recordId)}`}
                    record={editRecord}
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

                        <Box sx={{display: "flex", gap: 2, width: "100%"}}>
                            <TranslationScopeInput
                                source="editableScope"
                                defaultValue={ETranslationScope.ADMIN_PORTAL}
                                allowedScopes={TENANT_TRANSLATION_SCOPES}
                            />
                            <TextInput
                                source="editableKey"
                                label={String(t("electionEventScreen.localization.labels.key"))}
                                readOnly
                                fullWidth
                            />
                        </Box>
                        <TextInput
                            source="editableVal"
                            label={String(t("electionEventScreen.localization.labels.value"))}
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
