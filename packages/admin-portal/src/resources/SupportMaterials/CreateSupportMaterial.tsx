// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {
    SimpleForm,
    TextInput,
    Create,
    useRefresh,
    useNotify,
    Toolbar,
    SaveButton,
    useUpdate,
    useGetOne,
    BooleanInput,
} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {Tabs} from "@/components/Tabs"
import {DropFile} from "@sequentech/ui-essentials"
import {TextField} from "@mui/material"
import {Box, styled} from "@mui/material"
import {JsonInput} from "react-admin-json-view"
import {useMutation} from "@apollo/client"
import {GetUploadUrlMutation, Sequent_Backend_Support_Material} from "@/gql/graphql"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import VideoFileIcon from "@mui/icons-material/VideoFile"
import AudioFileIcon from "@mui/icons-material/AudioFile"
import PictureAsPdfIcon from "@mui/icons-material/PictureAsPdf"
import ImageIcon from "@mui/icons-material/Image"
import DescriptionIcon from "@mui/icons-material/Description"
import {Sequent_Backend_Support_Material_Extended} from "./EditSuportMaterial"

interface CreateSupportMaterialProps {
    record: any
    close?: () => void
}

interface I18n {
    [key: string]: {
        [key: string]: string
    }
}

const BASE_DATA = {
    title_i18n: {},
    subtitle_i18n: {},
}

const Hidden = styled(Box)`
    display: none;
`

export const CreateSupportMaterial: React.FC<CreateSupportMaterialProps> = (props) => {
    const {record, close} = props
    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [valueMaterials, setValueMaterials] = useState<I18n>(BASE_DATA)
    const [imageType, setImageType] = useState<string | undefined>()
    const [imageId, setImageId] = useState<string | undefined>()

    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const [updateImage] = useUpdate()

    const onSuccess = (data: Sequent_Backend_Support_Material) => {
        updateImage("sequent_backend_support_material", {
            id: data.id,
            data: {
                document_id: imageId,
            },
        })

        refresh()
        close?.()
        notify(t("materials.createMaterialSuccess"), {type: "success"})
    }

    const onError = async (res: any) => {
        refresh()
        close?.()
        notify("materials.createMaterialError", {type: "error"})
    }

    const renderTabs = (parsedValue: Sequent_Backend_Support_Material_Extended) => {
        let tabNodes = []
        for (const lang in parsedValue?.enabled_languages) {
            if (parsedValue?.enabled_languages[lang]) {
                tabNodes.push({
                    label: t(`common.language.${lang}`),
                    component: () => (
                        <>
                            <TextField
                                label={String(t("electionEventScreen.field.materialTitle"))}
                                size="small"
                                value={valueMaterials.title_i18n[lang] || ""}
                                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                                    setValueMaterials((prev) => ({
                                        ...prev,
                                        title_i18n: {...prev.title_i18n, [lang]: e.target.value},
                                    }))
                                }
                            />
                            <TextField
                                label={String(t("electionEventScreen.field.materialSubTitle"))}
                                size="small"
                                value={valueMaterials.subtitle_i18n[lang] || ""}
                                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                                    setValueMaterials((prev) => ({
                                        ...prev,
                                        subtitle_i18n: {
                                            ...prev.subtitle_i18n,
                                            [lang]: e.target.value,
                                        },
                                    }))
                                }
                            />
                        </>
                    ),
                })
            }
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
                    election_event_id: record?.id,
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
        data.kind = imageType ?? ""
        return data
    }

    const formValidator = (values: any): any => {
        const errors: {[key: string]: string} = {}
        if (!valueMaterials.title_i18n.en) {
            errors.data = t("materials.error.title")
        }
        if (!imageId) {
            errors.document_id = t("materials.error.document")
        }
        return errors
    }

    return (
        <Create
            transform={transform}
            resource="sequent_backend_support_material"
            mutationOptions={{onSuccess, onError}}
            redirect={false}
        >
            <PageHeaderStyles.Wrapper>
                <SimpleForm
                    validate={formValidator}
                    toolbar={
                        <Toolbar {...props}>
                            <SaveButton alwaysEnable />
                        </Toolbar>
                    }
                >
                    <PageHeaderStyles.Title>{t("materials.common.title")}</PageHeaderStyles.Title>
                    <PageHeaderStyles.SubTitle>
                        {t("materials.common.subtitle")}
                    </PageHeaderStyles.SubTitle>
                    <Tabs elements={renderTabs(record)} />
                    <BooleanInput
                        source={"is_hidden"}
                        label={String(t("materials.fields.isHidden"))}
                    />
                    <DropFile handleFiles={handleFiles} />
                    {imageType ? (
                        <Box
                            sx={{
                                width: "100%",
                                display: "flex",
                                justifyContent: "center",
                            }}
                        >
                            {imageType.includes("image") ? (
                                <ImageIcon sx={{fontSize: "80px"}} />
                            ) : imageType.includes("pdf") ? (
                                <PictureAsPdfIcon sx={{fontSize: "80px"}} />
                            ) : imageType.includes("video") ? (
                                <VideoFileIcon sx={{fontSize: "80px"}} />
                            ) : imageType.includes("audio") ? (
                                <AudioFileIcon sx={{fontSize: "80px"}} />
                            ) : (
                                <DescriptionIcon sx={{fontSize: "80px"}} />
                            )}
                        </Box>
                    ) : null}
                    <Hidden>
                        <TextInput
                            label="Election Event"
                            source="election_event_id"
                            defaultValue={record?.id || ""}
                        />
                        <TextInput
                            label="Tenant"
                            source="tenant_id"
                            defaultValue={record?.tenant_id || ""}
                        />
                        <JsonInput source="labels" jsonString={false} defaultValue={{}} />
                        <JsonInput source="annotations" jsonString={false} defaultValue={{}} />
                    </Hidden>
                </SimpleForm>
            </PageHeaderStyles.Wrapper>
        </Create>
    )
}
