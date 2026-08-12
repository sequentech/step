// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Dialog} from "@sequentech/ui-essentials"
import {
    isString,
    isValidVotingPortalDateTimePattern,
    VOTING_PORTAL_DATETIME_FORMAT_KEY,
} from "@sequentech/ui-core"
import React, {useMemo, useState} from "react"
import {
    Button,
    Datagrid,
    Identifier,
    SaveButton,
    SimpleForm,
    TextField,
    TextInput,
    WrapperField,
    useNotify,
    useRecordContext,
    useUpdate,
    useList,
    ListContextProvider,
} from "react-admin"
import EditIcon from "@mui/icons-material/Edit"
import Add from "@mui/icons-material/Add"
import DeleteIcon from "@mui/icons-material/Delete"
import {Sequent_Backend_Election_Event_Extended} from "./EditElectionEventDataForm"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {
    Box,
    Drawer,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    SelectChangeEvent,
    TablePagination,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {useLocalizationPermissions} from "./useLocalizationPermissions"

interface LocalizationListProps {
    selectedLanguage: string
    actions: Action[]
}

const LocalizationList: React.FC<LocalizationListProps> = ({selectedLanguage, actions}) => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event_Extended>()

    const translationData = useMemo(() => {
        return Object.entries(record?.presentation?.i18n?.[selectedLanguage] || {}).map(
            ([key, value]) => ({
                id: key,
                value: value as string,
            })
        )
    }, [record?.presentation?.i18n, selectedLanguage])

    const listContext = useList({
        data: translationData,
    })

    return (
        <ListContextProvider value={listContext}>
            <Datagrid bulkActionButtons={false}>
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
            <TablePagination
                component="div"
                page={listContext.page - 1}
                rowsPerPage={listContext.perPage}
                count={listContext.total || 0}
                onPageChange={(e, page) => listContext.setPage(page + 1)}
                onRowsPerPageChange={(e) => listContext.setPerPage(parseInt(e.target.value, 10))}
            />
        </ListContextProvider>
    )
}

const EditElectionEventTextDataTable = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event_Extended>()
    const [update] = useUpdate()

    const {t} = useTranslation()
    const notify = useNotify()

    // The Voting Portal date/time override is a free string typed by an operator. It is
    // validated by the same parser the voter-facing helper uses, so an invalid pattern is
    // rejected at save time instead of silently falling back to the preset at render time.
    const isInvalidDateTimeOverride = (key: string, value: string): boolean =>
        key === VOTING_PORTAL_DATETIME_FORMAT_KEY && !isValidVotingPortalDateTimePattern(value)

    const [selectedLanguage, setSelectedLanguage] = useState<string>(
        record?.presentation?.language_conf?.default_language_code ?? "en"
    )
    const [openEdit, setOpenEdit] = useState(false)
    const [openCreate, setOpenCreate] = useState(false)
    const [openDeleteModal, setOpenDeleteModal] = useState(false)
    const [deleteId, setDeleteId] = useState<Identifier | null>(null)
    const [recordId, setRecordId] = useState<Identifier | null>(null)

    const {
        canCreateLocalization,
        canEditLocalization,
        canDeleteLocalization,
        showLocalizationSelector,
    } = useLocalizationPermissions()

    const languageOptions = useMemo(() => {
        return (record?.presentation?.language_conf?.enabled_language_codes ?? []) as string[]
    }, [record?.presentation?.language_conf?.enabled_language_codes])

    const handleLanguageChange = (event: SelectChangeEvent<string>) => {
        const value = event?.target?.value ?? ""
        if (!isString(value) || !value) return
        setSelectedLanguage(value)
    }

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
        if (!newValue || !newKey) return
        if (isInvalidDateTimeOverride(newKey, newValue)) {
            notify(t("electionEventScreen.localization.notify.invalidDateTimeFormat"), {
                type: "error",
            })
            return
        }
        update(
            "sequent_backend_election_event",
            {
                id: record?.id,
                data: {
                    ...record,
                    presentation: {
                        ...record?.presentation,
                        i18n: {
                            ...record?.presentation.i18n,
                            [selectedLanguage]: {
                                ...record?.presentation.i18n?.[selectedLanguage],
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
        if (!e || !recordId) return
        const editVal: string = e?.editableVal ?? ""
        if (!editVal) return
        if (isInvalidDateTimeOverride(recordId as string, editVal)) {
            notify(t("electionEventScreen.localization.notify.invalidDateTimeFormat"), {
                type: "error",
            })
            return
        }
        update(
            "sequent_backend_election_event",
            {
                id: record?.id,
                data: {
                    ...record,
                    presentation: {
                        ...record?.presentation,
                        i18n: {
                            ...record?.presentation.i18n,
                            [selectedLanguage]: {
                                ...record?.presentation.i18n?.[selectedLanguage],
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
        const updatedI18nForLanguage = {...record?.presentation.i18n[selectedLanguage]}
        delete updatedI18nForLanguage[deleteId as string]

        update(
            "sequent_backend_election_event",
            {
                id: record?.id,
                data: {
                    ...record,
                    presentation: {
                        ...record?.presentation,
                        i18n: {
                            ...record?.presentation?.i18n,
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
        {icon: <EditIcon />, action: editAction, showAction: () => canEditLocalization},
        {icon: <DeleteIcon />, action: deleteAction, showAction: () => canDeleteLocalization},
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
                    {showLocalizationSelector ? (
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
                    ) : null}
                    <div className="list-actions">
                        {canCreateLocalization ? (
                            <Button
                                onClick={() => setOpenCreate(true)}
                                label={String(t("common.label.add"))}
                            >
                                <Add />
                            </Button>
                        ) : null}

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
                <Box sx={{flexGrow: 1, width: "100%"}}>
                    <LocalizationList selectedLanguage={selectedLanguage} actions={actions} />
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
                    record={record?.presentation?.i18n[selectedLanguage]}
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
                                    ? record?.presentation?.i18n[selectedLanguage][
                                          recordId as string
                                      ]
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
