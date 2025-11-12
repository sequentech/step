// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {
    SimpleForm,
    useRefresh,
    useNotify,
    Toolbar,
    SaveButton,
    useUpdate,
    Identifier,
    EditBase,
    RecordContext,
    useGetOne,
    BooleanInput,
    useRecordContext,
    useGetList,
    RaRecord,
} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {Tabs} from "@/components/Tabs"
import {DropFile} from "@sequentech/ui-essentials"
import {IElectionEventPresentation} from "@sequentech/ui-core"
import {Box, TextField} from "@mui/material"
import {useMutation} from "@apollo/client"
import {
    GetUploadUrlMutation,
    Sequent_Backend_Document,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Support_Material,
} from "@/gql/graphql"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {useTenantStore} from "@/providers/TenantContextProvider"
import VideoFileIcon from "@mui/icons-material/VideoFile"
import AudioFileIcon from "@mui/icons-material/AudioFile"
import PictureAsPdfIcon from "@mui/icons-material/PictureAsPdf"
import ImageIcon from "@mui/icons-material/Image"
import {SettingsContext} from "@/providers/SettingsContextProvider"

export type Sequent_Backend_Support_Material_Extended = RaRecord<Identifier> & {
    enabled_languages?: {[key: string]: boolean}
    defaultLanguage?: string
} & Sequent_Backend_Support_Material

interface EditSupportMaterialProps {
    id?: string
    electionEventId?: string
    close?: () => void
}

interface I18n {
    [key: string]: {
        [key: string]: string
    }
}

interface GetPublicURLProps {
    electionEventId: string
}

const GetPublicURL: React.FC<GetPublicURLProps> = ({electionEventId}) => {
    const record = useRecordContext<Sequent_Backend_Support_Material>()
    const {globalSettings} = useContext(SettingsContext)
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {data} = useGetList<Sequent_Backend_Document>("sequent_backend_document", {
        pagination: {page: 1, perPage: 1},
        filter: {
            id: record?.document_id,
            election_event_id: electionEventId,
            tenant_id: tenantId,
        },
    })

    if (!data) {
        return null
    }

    const url = `${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/document-${record?.document_id}/${data[0]?.name}`

    return (
        <>
            <TextField
                contentEditable={false}
                value={url}
                label={String(t("materials.fields.publicUrl"))}
            />
        </>
    )
}

export const EditSupportMaterial: React.FC<EditSupportMaterialProps> = (props) => {
    const {id, electionEventId, close} = props
    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [valueMaterials, setValueMaterials] = useState<I18n | null>(null)
    const [imageType, setImageType] = useState<string | undefined>()
    const [imageId, setImageId] = useState<string | undefined>()
    const [renderUI, setRenderUI] = useState(false)

    const [tenantId] = useTenantStore()
    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const [updateImage] = useUpdate()

    const {data: record} = useGetOne<Sequent_Backend_Election_Event>(
        "sequent_backend_election_event",
        {
            id: electionEventId,
            meta: {tenant_id: tenantId},
        }
    )

    useEffect(() => {
        if (record) {
            setRenderUI(true)
        }
    }, [record])

    const onSuccess = () => {
        if (imageId) {
            updateImage("sequent_backend_support_material", {
                id,
                data: {
                    document_id: imageId,
                },
            })
        }

        refresh()
        notify(t("materials.updateMaterialSuccess"), {type: "success"})
        if (close) {
            close()
        }
    }

    const onError = async (res: any) => {
        refresh()

        // react-admin bug: https://stackoverflow.com/questions/54729867/cannot-read-property-hasownproperty-of-undefined-during-writing-my-own-datap
        // It seems to be caused by validateResponseFormat in fetch. When an options request is triggered prior to a put it provides an empty response and cannot be parsed

        if (res?.message?.includes("hasOwnProperty")) {
            notify(t("materials.updateMaterialSuccess"), {type: "success"})
        } else {
            notify("materials.updateMaterialError", {type: "error"})
        }

        if (close) {
            close()
        }
    }

    const renderTabs = (parsedValue: Sequent_Backend_Support_Material_Extended) => {
        let tabNodes = []

        if (!record) {
            return []
        }

        if (!valueMaterials) setValueMaterials({...parsedValue.data})

        let presentation: IElectionEventPresentation = record.presentation

        for (const lang of presentation?.language_conf?.enabled_language_codes ?? []) {
            // if (parsedValue?.enabled_languages[lang]) {
            tabNodes.push({
                label: t(`common.language.${lang}`),
                component: () => (
                    <>
                        <TextField
                            label={String(t("electionEventScreen.field.materialTitle"))}
                            size="small"
                            value={valueMaterials?.title_i18n[lang] || ""}
                            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                                setValueMaterials((prev) => ({
                                    ...prev,
                                    title_i18n: {...prev?.title_i18n, [lang]: e.target.value},
                                }))
                            }
                        />
                        <TextField
                            label={String(t("electionEventScreen.field.materialSubTitle"))}
                            size="small"
                            value={valueMaterials?.subtitle_i18n[lang] || ""}
                            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                                setValueMaterials((prev) => ({
                                    ...prev,
                                    subtitle_i18n: {
                                        ...prev?.subtitle_i18n,
                                        [lang]: e.target.value,
                                    },
                                }))
                            }
                        />
                    </>
                ),
            })
            // }
        }

        return tabNodes
    }

    const handleFiles = async (files: FileList | null) => {
        // https://fullstackdojo.medium.com/s3-upload-with-presigned-url-react-and-nodejs-b77f348d54cc

        const theFile = files?.[0]

        setImageType(theFile?.type)

        if (theFile) {
            let {data, errors} = await getUploadUrl({
                variables: {
                    name: theFile.name,
                    media_type: theFile.type,
                    size: theFile.size,
                },
            })
            if (data?.get_upload_url?.document_id) {
                try {
                    await fetch(data.get_upload_url.url, {
                        method: "PUT",
                        headers: {
                            "Content-Type": theFile.type,
                        },
                        body: theFile,
                    })
                    notify(t("electionScreen.common.fileLoaded"), {type: "success"})

                    setImageId(data.get_upload_url.document_id)
                } catch (e) {
                    console.log("error :>> ", e)
                    notify(t("electionScreen.error.fileError"), {type: "error"})
                }
            } else {
                console.log("error :>> ", errors)
                notify(t("electionScreen.error.fileError"), {type: "error"})
            }
        }
    }

    const transform = (data: Sequent_Backend_Support_Material_Extended) => {
        data.data = {...valueMaterials}
        if (imageType) {
            data.kind = imageType
        }
        return data
    }

    const formValidator = (values: any): any => {
        const errors: {[key: string]: string} = {}
        if (!valueMaterials?.title_i18n?.en) {
            errors.data = t("materials.error.title")
        }
        return errors
    }

    const parseValues = (
        incoming: Sequent_Backend_Support_Material_Extended
    ): Sequent_Backend_Support_Material_Extended => {
        const temp = {...incoming}
        return temp
    }

    if (renderUI) {
        return (
            <EditBase
                id={id}
                transform={transform}
                resource="sequent_backend_support_material"
                mutationMode="pessimistic"
                mutationOptions={{onSuccess, onError}}
                redirect={false}
            >
                <PageHeaderStyles.Wrapper>
                    <RecordContext.Consumer>
                        {(incoming) => {
                            const parsedValue = parseValues(
                                incoming as Sequent_Backend_Support_Material_Extended
                            )
                            return (
                                <SimpleForm
                                    validate={formValidator}
                                    toolbar={
                                        <Toolbar>
                                            <SaveButton alwaysEnable />
                                        </Toolbar>
                                    }
                                >
                                    <PageHeaderStyles.Title>
                                        {t("materials.common.title")}
                                    </PageHeaderStyles.Title>
                                    <PageHeaderStyles.SubTitle>
                                        {t("materials.common.subtitle")}
                                    </PageHeaderStyles.SubTitle>
                                    <Tabs elements={renderTabs(parsedValue)} />
                                    <BooleanInput
                                        source="is_hidden"
                                        label={String(t("materials.fields.isHidden"))}
                                    />
                                    {electionEventId ? (
                                        <GetPublicURL electionEventId={electionEventId} />
                                    ) : null}
                                    <DropFile handleFiles={handleFiles} />
                                    {parsedValue.document_id ? (
                                        <Box
                                            sx={{
                                                width: "100%",
                                                display: "flex",
                                                justifyContent: "center",
                                            }}
                                        >
                                            {parsedValue.kind.includes("image") ? (
                                                <ImageIcon sx={{fontSize: "80px"}} />
                                            ) : parsedValue.kind.includes("pdf") ? (
                                                <PictureAsPdfIcon sx={{fontSize: "80px"}} />
                                            ) : parsedValue.kind.includes("video") ? (
                                                <VideoFileIcon sx={{fontSize: "80px"}} />
                                            ) : parsedValue.kind.includes("audio") ? (
                                                <AudioFileIcon sx={{fontSize: "80px"}} />
                                            ) : null}
                                        </Box>
                                    ) : null}
                                </SimpleForm>
                            )
                        }}
                    </RecordContext.Consumer>
                </PageHeaderStyles.Wrapper>
            </EditBase>
        )
    } else {
        return null
    }
}
