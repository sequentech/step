// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {
    Datagrid,
    FormDataConsumer,
    ListContextProvider,
    SaveButton,
    SimpleForm,
    TextField,
    TextInput,
    useList,
    useNotify,
    useRecordContext,
    useRefresh,
    useUpdate,
    WrapperField,
} from "react-admin"
import {
    Button,
    Alert,
    Box,
    Card,
    Drawer,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    TablePagination,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {IVR_CONFIG_ANNOTATION, IVR_PROMPTS_ANNOTATION} from "@/utils/ivr"
import {JsonEditor} from "json-edit-react"
import {Dialog} from "@sequentech/ui-essentials"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {Action, ActionsColumn} from "@/components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import DeleteIcon from "@mui/icons-material/Delete"
import Add from "@mui/icons-material/Add"

const RESOURCE = "sequent_backend_election_event"

type Prompts = Record<string, Record<string, string>>

const collectRequiredPromptKeys = (configAnnotation: string) => {
    let config: any = {}
    try {
        config = JSON.parse(configAnnotation)
    } catch (e) {
        console.error("Failed to parse the ivr config annotation", e)
        return new Set<string>()
    }
    let flow = config?.flow
    if (Array.isArray(flow)) {
        return new Set<string>(
            flow.map((item) => item?.prompt_key).filter((key) => typeof key === "string")
        )
    } else {
        return new Set<string>()
    }
}

interface PromptsListProps {
    prompts: Prompts
    requiredPromptKeys: Set<string>
    selectedLanguage: string
    actions: Action[]
}

const PromptsList: React.FC<PromptsListProps> = ({
    prompts,
    requiredPromptKeys,
    selectedLanguage,
    actions,
}) => {
    const {t} = useTranslation()
    const data = useMemo(() => {
        return Object.entries(prompts[selectedLanguage] || {})
            .map(([key, value]) => ({
                id: key,
                value: value,
                required: requiredPromptKeys.has(key),
            }))
            .sort((a, b) => {
                return Number(b.required) - Number(a.required) || a.id.localeCompare(b.id)
            })
    }, [prompts, selectedLanguage])
    const listContext = useList({data: data, perPage: 10})

    return (
        <ListContextProvider value={listContext}>
            <Card>
                <Datagrid
                    bulkActionButtons={false}
                    onClick={(e) => e.preventDefault()}
                    sx={{
                        "& .column-id": {minWidth: "150px"},
                        "& .column-value": {width: "100%"},
                        "& .column-actions": {minWidth: "100px", whiteSpace: "nowrap"},
                    }}
                >
                    <TextField
                        source="id"
                        label={String(t("electionEventScreen.localization.labels.key"))}
                    />
                    <TextField
                        source="value"
                        label={String(t("electionEventScreen.localization.labels.value"))}
                    />
                    <WrapperField source="actions" label="Actions">
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </Datagrid>
            </Card>
            <TablePagination
                component="div"
                page={listContext.page ? listContext.page - 1 : 0}
                rowsPerPage={listContext.perPage}
                rowsPerPageOptions={[5, 10, 25, 50]}
                count={listContext.total || 0}
                onPageChange={(e, page) => listContext.setPage(page + 1)}
                onRowsPerPageChange={(e) => {
                    listContext.setPerPage(parseInt(e.target.value, 10))
                    listContext.setPage(1)
                }}
            />
        </ListContextProvider>
    )
}

export const IvrPrompts: React.FC = () => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const notify = useNotify()
    const refresh = useRefresh()
    const [update] = useUpdate()

    // Editor drawers
    const [openCreate, setOpenCreate] = useState(false)
    const [openEdit, setOpenEdit] = useState(false)
    const [openDeleteModal, setOpenDelete] = useState(false)
    const [deleteId, setDeleteId] = useState<string | null>(null)
    const [editId, setEditId] = useState<string | null>(null)
    const [saving, setSaving] = useState(false)

    // Languages
    const [selectedLanguage, setSelectedLanguage] = useState<string>(
        record?.presentation?.language_conf?.default_language_code ?? "en"
    )
    const languages = useMemo(() => {
        return (record?.presentation?.language_conf?.enabled_language_codes ?? []) as string[]
    }, [record?.presentation?.language_conf?.enabled_language_codes])

    // Annotation mapping
    const annotations = (record?.annotations ?? {}) as Record<string, string>
    const recordPrompts =
        typeof annotations[IVR_PROMPTS_ANNOTATION] === "string"
            ? (annotations[IVR_PROMPTS_ANNOTATION] as string)
            : "{}"
    const requiredPromptKeys = useMemo(
        () => collectRequiredPromptKeys(annotations[IVR_CONFIG_ANNOTATION]),
        [annotations[IVR_CONFIG_ANNOTATION]]
    )
    const parsedPrompts: Prompts = useMemo(() => {
        try {
            let obj = JSON.parse(recordPrompts)
            let langs = [...Object.keys(obj), ...languages, selectedLanguage]
            langs.forEach((lang) =>
                requiredPromptKeys.forEach((key) => {
                    if (!(lang in obj)) {
                        obj[lang] = {}
                    }
                    if (!(key in obj[lang])) {
                        obj[lang][key] = ""
                    }
                })
            )
            return obj
        } catch (e) {
            console.error("Failed to parse the prompts config value as json", e)
            return {}
        }
    }, [recordPrompts, requiredPromptKeys, languages])

    const [editorData, setEditorData] = useState(parsedPrompts)
    useEffect(() => {
        setEditorData(parsedPrompts)
    }, [parsedPrompts])
    const pendingPayload = useMemo(() => {
        return JSON.stringify(editorData)
    }, [editorData])
    const dirty: boolean = useMemo(() => {
        return pendingPayload !== recordPrompts
    }, [pendingPayload, recordPrompts])

    // Data / editor validation
    const promptsValid = (prompts: Prompts) => {
        return Object.entries(prompts).every(([_lang, entries]) => {
            return Object.entries(entries).every(([key, value]) => {
                // Required prompts must be given for all languages, no exceptions.
                if (requiredPromptKeys.has(key)) {
                    return key.trim() && value.trim()
                }
                return key.trim()
            })
        })
    }
    const editorValid: boolean = useMemo(() => promptsValid(editorData), [editorData])

    if (!record?.id) {
        return null
    }

    // List / editor interface
    const actions: Action[] = [
        {
            icon: <EditIcon />,
            action: (id) => {
                if (typeof id === "string") {
                    setEditId(id)
                    setOpenEdit(true)
                }
            },
        },
        {
            icon: <DeleteIcon />,
            action: (id) => {
                if (typeof id === "string") {
                    setDeleteId(id)
                    setOpenDelete(true)
                }
            },
            showAction: (id) => typeof id !== "string" || !requiredPromptKeys.has(id),
        },
    ]

    // Editor operations
    const createPromptKey = (rawKey: any, rawValue: any) => {
        let key = (typeof rawKey === "string" ? rawKey : "").trim()
        let value = (typeof rawValue === "string" ? rawValue : "").trim()
        if (key && value) {
            let newData = {...editorData}
            let langs = [...Object.keys(newData), ...languages, selectedLanguage]
            langs.forEach((lang) => {
                if (!(lang in newData)) {
                    newData[lang] = {}
                }
                if (!(key in newData[lang])) {
                    newData[lang][key] = value
                }
            })
            setEditorData(newData)
        }
    }
    const updatePromptKey = (rawValue: any) => {
        let value = (typeof rawValue === "string" ? rawValue : "").trim()
        if (editId) {
            let newData = {...editorData}
            newData[selectedLanguage][editId] = value
            setEditorData(newData)
        }
    }
    const deletePromptKey = () => {
        if (deleteId && !requiredPromptKeys.has(deleteId)) {
            let newData = {...editorData}
            let langs = [...Object.keys(newData), ...languages, selectedLanguage]
            langs.forEach((lang) => {
                if (lang in newData) {
                    delete newData[lang][deleteId]
                }
            })
            setEditorData(newData)
        }
    }

    // Record persistence
    const handleCancel = () => {
        setEditorData(parsedPrompts)
    }
    const handleSave = () => {
        setSaving(true)
        update(
            RESOURCE,
            {
                id: record.id,
                data: {
                    annotations: {
                        ...annotations,
                        [IVR_PROMPTS_ANNOTATION]: pendingPayload,
                    },
                },
                previousData: record,
            },
            {
                onSuccess: () => {
                    setSaving(false)
                    notify(t("electionEventScreen.ivr.common.saveSuccess"), {type: "success"})
                    refresh()
                },
                onError: () => {
                    setSaving(false)
                    notify(t("electionEventScreen.ivr.common.saveError"), {type: "error"})
                },
            }
        )
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <Alert severity="info">
                Configure the prompts used by the IVR. Announcement prompts are required, and system
                prompts can be overridden for desired languages. SSML is supported, including for
                mixing languages.
            </Alert>
            <Box>
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
                            onChange={(e) => {
                                let newLang = e?.target?.value
                                if (newLang && typeof newLang === "string")
                                    setSelectedLanguage(newLang)
                            }}
                            value={selectedLanguage}
                        >
                            {languages &&
                                languages.map((lang) => {
                                    return (
                                        <MenuItem key={lang} value={lang}>
                                            {t(`common.language.${lang}`)}
                                        </MenuItem>
                                    )
                                })}
                        </Select>
                    </FormControl>
                    <Button
                        onClick={() => {
                            setOpenCreate(true)
                        }}
                    >
                        <Add />
                        {t("common.label.add")}
                    </Button>
                </Box>
                <Box sx={{flexGrow: 1, width: "100%"}}>
                    <PromptsList
                        prompts={editorData}
                        selectedLanguage={selectedLanguage}
                        actions={actions}
                        requiredPromptKeys={requiredPromptKeys}
                    />
                </Box>
            </Box>

            <Drawer
                anchor="right"
                open={openEdit || openCreate}
                onClose={() => {
                    setOpenCreate(false)
                    setOpenEdit(false)
                    setEditId(null)
                }}
            >
                {openEdit && editId ? (
                    <SimpleForm
                        defaultValues={{
                            key: editId,
                            value: editorData[selectedLanguage][editId] ?? "",
                        }}
                        toolbar={
                            <FormDataConsumer>
                                {({formData}) => (
                                    <SaveButton
                                        disabled={
                                            !formData?.value ||
                                            formData?.value === editorData[selectedLanguage][editId]
                                        }
                                        sx={{marginInline: "1rem"}}
                                    />
                                )}
                            </FormDataConsumer>
                        }
                        onSubmit={(e: any) => {
                            updatePromptKey(e?.value)
                            setOpenEdit(false)
                            setEditId(null)
                        }}
                    >
                        <>
                            <PageHeaderStyles.Title>
                                {t("electionEventScreen.localization.common.title")}
                            </PageHeaderStyles.Title>
                            <PageHeaderStyles.SubTitle>
                                {t("electionEventScreen.localization.common.subTitle")}
                            </PageHeaderStyles.SubTitle>

                            <TextInput
                                source="key"
                                label={String(t("electionEventScreen.localization.labels.key"))}
                                readOnly
                            />
                            <TextInput
                                source="value"
                                label={String(t("electionEventScreen.localization.labels.value"))}
                                multiline
                            />
                        </>
                    </SimpleForm>
                ) : null}

                {openCreate ? (
                    <SimpleForm
                        defaultValues={{key: "new_prompt_key", value: ""}}
                        toolbar={
                            <FormDataConsumer>
                                {({formData}) => (
                                    <SaveButton
                                        disabled={
                                            !formData?.value ||
                                            !formData?.key ||
                                            Object.keys(editorData[selectedLanguage]).includes(
                                                formData?.key
                                            )
                                        }
                                        sx={{marginInline: "1rem"}}
                                    />
                                )}
                            </FormDataConsumer>
                        }
                        onSubmit={(e: any) => {
                            createPromptKey(e?.key, e?.value)
                            setOpenCreate(false)
                        }}
                    >
                        <>
                            <PageHeaderStyles.Title>
                                {t("electionEventScreen.localization.common.title")}
                            </PageHeaderStyles.Title>
                            <PageHeaderStyles.SubTitle>
                                {t("electionEventScreen.localization.common.subTitle")}
                            </PageHeaderStyles.SubTitle>

                            <TextInput
                                source="key"
                                label={String(t("electionEventScreen.localization.labels.key"))}
                            />
                            <TextInput
                                source="value"
                                label={String(t("electionEventScreen.localization.labels.value"))}
                                multiline
                            />
                        </>
                    </SimpleForm>
                ) : null}
            </Drawer>

            <Dialog
                // Delete dialog
                variant="warning"
                open={openDeleteModal}
                ok={String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        deletePromptKey()
                    }
                    setDeleteId(null)
                    setOpenDelete(false)
                }}
            >
                {t("common.message.delete")}
            </Dialog>

            <JsonEditor
                data={editorData}
                collapse={true}
                rootName="ivr:prompts"
                maxWidth={"100%"}
                setData={(nextData) => {
                    setEditorData(nextData as Prompts)
                }}
            />
            <Box sx={{mt: 3, display: "flex", gap: 2}}>
                <Button
                    variant="contained"
                    onClick={handleSave}
                    disabled={!dirty || !editorValid || saving}
                >
                    {t("common.label.save")}
                </Button>
                <Button variant="contained" onClick={handleCancel} disabled={!dirty || saving}>
                    {t("common.label.cancel")}
                </Button>
            </Box>
        </Box>
    )
}
