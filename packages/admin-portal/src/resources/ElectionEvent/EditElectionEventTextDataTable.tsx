// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Dialog} from "@sequentech/ui-essentials"
import {isString} from "@sequentech/ui-core"
import React, {useMemo, useState} from "react"
import {
    Button,
    Datagrid,
    Identifier,
    List,
    SaveButton,
    SimpleForm,
    SortPayload,
    TextField,
    TextInput,
    WrapperField,
    useListContext,
    useNotify,
    useRecordContext,
    useUpdate,
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
import _ from "lodash"
import {useLocalizationPermissions} from "./useLocalizationPermissions"

interface LocalizationListProps {
    selectedLanguage: string
    election_event_id: string
    actions: Action[]
}

const LocalizationList: React.FC<LocalizationListProps> = ({
    selectedLanguage,
    election_event_id,
    actions,
}) => {
    const {data, isLoading} = useListContext()
    const {t} = useTranslation()
    const [page, setPage] = useState(0)
    const [pageSize, setPageSize] = useState(10)
    const [sort, setSort] = useState<SortPayload>({
        field: "id",
        order: "ASC",
    })

    const targetElectionEvent = useMemo(() => {
        return data?.find((e) => e.id === election_event_id)
    }, [data, isLoading, election_event_id])

    const translationData = Object.entries(
        targetElectionEvent?.presentation?.i18n?.[selectedLanguage] || {}
    ).map(([key, value]) => ({
        id: key,
        value: value,
    }))

    const sortedTranslationData = useMemo(() => {
        //@ts-ignore
        return _.orderBy(translationData, [sort.field], [sort.order.toLowerCase()])
    }, [translationData, sort])

    const paginatedData = useMemo(() => {
        return _.chunk(sortedTranslationData, pageSize)
    }, [sortedTranslationData, pageSize])

    if (isLoading) {
        return <p>{t("loading")}</p>
    }

    const handlePageChange = (e: any, page: number) => {
        setPage(page)
    }

    const handleRowsChange = (v: number) => {
        setPageSize(v)
    }

    return (
        <>
            <Datagrid
                data={paginatedData[page]}
                total={translationData.length}
                bulkActionButtons={false}
                sort={sort}
                setSort={setSort}
            >
                <TextField source="id" label={t("electionEventScreen.localization.labels.key")} />
                <TextField
                    source="value"
                    label={t("electionEventScreen.localization.labels.value")}
                />
                <WrapperField label="Actions">
                    <ActionsColumn actions={actions} />
                </WrapperField>
            </Datagrid>
            <TablePagination
                page={page}
                rowsPerPage={pageSize}
                count={translationData.length || 0}
                onPageChange={handlePageChange}
                onRowsPerPageChange={(e) => handleRowsChange(parseInt(e.target.value))}
            />
        </>
    )
}

const EditElectionEventTextDataTable = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event_Extended>()
    const [update, {isLoading}] = useUpdate()

    const {t} = useTranslation()
    const notify = useNotify()

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
        const updatedI18nForLanguage = {...record.presentation.i18n[selectedLanguage]}
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
                            ...record.presentation?.i18n,
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
                                label={t("electionEventScreen.localization.selectLanguage")}
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
                                label={t("common.label.add")}
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
                                        label={t("electionEventScreen.localization.labels.key")}
                                    />
                                    <TextInput
                                        source={`presentation.i18n.${selectedLanguage}.newVal`}
                                        label={t("electionEventScreen.localization.labels.value")}
                                        multiline
                                    />
                                </>
                            </SimpleForm>
                        </Drawer>
                    </div>
                </Box>
                <List actions={false} sx={{flexGrow: 1, width: "100%"}} pagination={false}>
                    <LocalizationList
                        selectedLanguage={selectedLanguage}
                        election_event_id={record.id}
                        actions={actions}
                    />
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
                            label={t("electionEventScreen.localization.labels.key")}
                            defaultValue={recordId ?? undefined}
                            disabled
                        />
                        <TextInput
                            source="editableVal"
                            label={t("electionEventScreen.localization.labels.value")}
                            defaultValue={
                                recordId
                                    ? record?.presentation?.i18n[selectedLanguage][recordId]
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
