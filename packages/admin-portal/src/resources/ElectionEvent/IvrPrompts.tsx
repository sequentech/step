// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {
    Datagrid,
    Identifier,
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
    Box,
    Button,
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

const collectRequiredPrompts = (configAnnotation: string) => {
    let config: any = {}
    try {
        config = JSON.parse(configAnnotation)
    } catch (e) {
        console.error("Failed to parse the ivr config annotation", e)
        return []
    }
    let flow = config?.flow
    if (Array.isArray(flow)) {
        return flow.map((item) => item?.prompt_key).filter((key) => typeof key === "string")
    } else {
        return []
    }
}

interface PromptsListProps {
    prompts: Record<string, Record<string, string>>
    selectedLanguage: string
    actions: Action[]
}

const PromptsList: React.FC<PromptsListProps> = ({prompts, selectedLanguage, actions}) => {
    const {t} = useTranslation()
    const data = useMemo(() => {
        return Object.entries(prompts[selectedLanguage] || {}).map(([key, value]) => ({
            id: key,
            value: value as string,
        }))
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
    const [openEdit, setOpenEdit] = useState(false)
    const [openCreate, setOpenCreate] = useState(false)
    const [openDeleteModal, setOpenDeleteModal] = useState(false)
    const [deleteId, setDeleteId] = useState<string | null>(null)
    const [editId, setEditId] = useState<string | null>(null)
    const editValue: string | null = null

    // Languages
    const [selectedLanguage, setSelectedLanguage] = useState<string>(
        record?.presentation?.language_conf?.default_language_code ?? "en"
    )
    const languages = useMemo(() => {
        return (record?.presentation?.language_conf?.enabled_language_codes ?? []) as string[]
    }, [record?.presentation?.language_conf?.enabled_language_codes])

    // Annotation mapping
    const annotations = (record?.annotations ?? {}) as Record<string, string>
    const stringPrompts =
        typeof annotations[IVR_PROMPTS_ANNOTATION] === "string"
            ? (annotations[IVR_PROMPTS_ANNOTATION] as string)
            : "{}"
    const requiredPrompts = useMemo(
        () => collectRequiredPrompts(annotations[IVR_CONFIG_ANNOTATION]),
        [annotations[IVR_CONFIG_ANNOTATION]]
    )
    const parsedPrompts: Record<string, Record<string, string>> = useMemo(() => {
        try {
            let obj = JSON.parse(stringPrompts)
            let langs = [...Object.keys(obj), ...languages]
            langs.forEach((lang) =>
                requiredPrompts.forEach((key) => {
                    if (!(lang in obj)) {
                        obj[lang] = {}
                    }
                    if (!(key in obj[lang])) {
                        obj[lang][key] = null
                    }
                })
            )
            return obj
        } catch (e) {
            console.error("Failed to parse the prompts config value as json", e)
            return {}
        }
    }, [stringPrompts, requiredPrompts])

    const [editorData, setEditorData] = useState(parsedPrompts)
    const pendingPayload = useMemo(() => {
        return JSON.stringify(editorData)
    }, [editorData])

    const [saving, setSaving] = useState(false)
    const dirty = pendingPayload !== stringPrompts

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
                    setOpenDeleteModal(true)
                }
            },
        },
    ]
    const handleSetPromptKey = (param: any) => {
        let newValue = param?.value
        if (editId && typeof newValue === "string") {
            editorData[selectedLanguage][editId] = newValue
            console.log(editorData[selectedLanguage][editId])
            console.log(editorData[selectedLanguage])
            console.log(editorData)
            setEditorData(editorData)
        }
    }
    const handleDeletePromptKey = () => {
        if (deleteId) {
            delete editorData[selectedLanguage][deleteId]
            setEditorData(editorData)
        }
    }
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
        <Box sx={{display: "flex", flexDirection: "column", gap: 2, mt: 2, maxWidth: 640}}>
            <>
                <JsonEditor
                    data={editorData}
                    rootName="ivr:prompts"
                    setData={(nextData) => {
                        // @ts-ignore
                        setEditorData(nextData)
                    }}
                />
            </>
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
                </Box>
                <Box sx={{flexGrow: 1, width: "100%"}}>
                    <PromptsList
                        prompts={editorData}
                        selectedLanguage={selectedLanguage}
                        actions={actions}
                    />
                </Box>
            </SimpleForm>
            {editId ? (
                <Drawer
                    anchor="right"
                    open={openEdit}
                    onClose={() => {
                        setEditId(null)
                        setOpenEdit(false)
                    }}
                    PaperProps={{
                        sx: {width: "40%"},
                    }}
                >
                    <SimpleForm
                        defaultValues={{key: editId, value: editorData[selectedLanguage][editId]}}
                        toolbar={<SaveButton sx={{marginInline: "1rem"}} />}
                        onSubmit={(e: any) => handleSetPromptKey(e)}
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
                </Drawer>
            ) : null}
            <Dialog
                // Delete dialog
                variant="warning"
                open={openDeleteModal}
                ok={String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        handleDeletePromptKey()
                    }
                    setOpenDeleteModal(false)
                    setDeleteId(null)
                }}
            >
                {t("common.message.delete")}
            </Dialog>
            <Box sx={{mt: 3, display: "flex", gap: 2}}>
                <Button variant="contained" onClick={handleSave} disabled={!dirty || saving}>
                    {t("common.label.save")}
                </Button>
                <Button variant="contained" onClick={handleCancel} disabled={!dirty || saving}>
                    {t("common.label.cancel")}
                </Button>
            </Box>
        </Box>
    )
}
