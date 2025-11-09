// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    BooleanInput,
    SelectInput,
    TextInput,
    useRecordContext,
    useRefresh,
    SimpleForm,
    useGetOne,
    Toolbar,
    SaveButton,
    required,
    useUpdate,
    useNotify,
    RaRecord,
    Identifier,
    RecordContext,
} from "react-admin"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Tabs,
    Tab,
    Grid,
    IconButton,
    CircularProgress,
} from "@mui/material"
import {
    GetUploadUrlMutation,
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Document,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
} from "../../gql/graphql"
import React, {ReactNode, useCallback, useContext, useEffect, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {
    ICandidatePresentation,
    IElectionEventPresentation,
    ILanguageConf,
    ICandidateUrl,
    IElectionPresentation,
    IInvalidVotePosition,
} from "@sequentech/ui-core"
import {CandidateStyles} from "../../components/styles/CandidateStyles"
import {CANDIDATE_TYPES} from "./constants"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {cloneDeep} from "lodash"
import {faTrash} from "@fortawesome/free-solid-svg-icons"
import styled from "@emotion/styled"
import {DropFile, Icon, adminTheme} from "@sequentech/ui-essentials"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

const StyledIconButton = styled(IconButton)`
    color: ${adminTheme.palette.brandColor};
    font-size: 18px;
    margin-left: auto;
    margin-right: 8px;
`

export type Sequent_Backend_Candidate_Extended = Sequent_Backend_Candidate &
    RaRecord<Identifier> & {
        enabled_languages?: {[key: string]: boolean}
        defaultLanguage?: string
    }

export const CandidateDataForm: React.FC<{
    record: Sequent_Backend_Candidate
}> = ({record}) => {
    const {t} = useTranslation()
    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const [languageConf, setLanguageConf] = useState<Array<string>>(["en"])
    const notify = useNotify()
    const refresh = useRefresh()
    const {globalSettings} = useContext(SettingsContext)
    const [enabledDeleteImage, setEnabledDeleteImage] = useState<boolean>(true)

    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("candidate-data-general")
    // const [defaultLangValue, setDefaultLangValue] = useState<string>("")
    const authContext = useContext(AuthContext)

    const canEdit = authContext.isAuthorized(true, authContext.tenantId, IPermissions.CONTEST_WRITE)

    const {data: electionEvent} = useGetOne<Sequent_Backend_Election_Event>(
        "sequent_backend_election_event",
        {
            id: record.election_event_id,
            meta: {tenant_id: record.tenant_id},
        }
    )

    const {data: contest} = useGetOne<Sequent_Backend_Contest>("sequent_backend_contest", {
        id: record.contest_id,
        meta: {tenant_id: record.tenant_id},
    })

    const {data: election} = useGetOne<Sequent_Backend_Election>("sequent_backend_election", {
        id: contest?.election_id ?? record.tenant_id,
        meta: {tenant_id: record.tenant_id},
    })

    const {data: imageData} = useGetOne<Sequent_Backend_Document>("sequent_backend_document", {
        id: record.image_document_id,
        meta: {tenant_id: record.tenant_id},
    })

    useEffect(() => {
        if (election) {
            let langConf = (election.presentation as IElectionPresentation | undefined)
                ?.language_conf
            if (langConf?.enabled_language_codes) {
                setLanguageConf(langConf?.enabled_language_codes)
                return
            }
        }
        if (electionEvent) {
            let langConf = (electionEvent.presentation as IElectionEventPresentation | undefined)
                ?.language_conf
            if (langConf?.enabled_language_codes) {
                setLanguageConf(langConf?.enabled_language_codes)
                return
            }
        }
    }, [electionEvent?.presentation?.language_conf, election?.presentation?.language_conf])

    const getImageUrl = (
        tenantId?: string,
        imageDocumentId?: string | null,
        name?: string | null
    ) => `tenant-${tenantId}/document-${imageDocumentId}/${name}`

    const [updateImage] = useUpdate<Sequent_Backend_Candidate>()

    const parseValues = useCallback(
        (incoming: Sequent_Backend_Candidate_Extended): Sequent_Backend_Candidate_Extended => {
            if (!electionEvent) {
                return incoming
            }
            const newCandidate: Sequent_Backend_Candidate_Extended = cloneDeep(incoming)
            newCandidate.type = newCandidate.type || undefined
            const newPresentation = (newCandidate.presentation ?? {}) as ICandidatePresentation

            newCandidate.presentation = newPresentation
            // name, alias and description fields
            if (!newCandidate.presentation) {
                newCandidate.presentation = {i18n: {en: {}}}
            }
            if (!newCandidate.presentation.i18n) {
                newCandidate.presentation.i18n = {en: {}}
            }
            if (!newCandidate.presentation.i18n.en) {
                newCandidate.presentation.i18n.en = {}
            }
            if (!newCandidate.presentation.i18n.en.name && newCandidate.name) {
                newCandidate.presentation.i18n.en.name = newCandidate.name
            }
            if (!newCandidate.presentation.i18n.en.name && newCandidate.name) {
                newCandidate.presentation.i18n.en.name = newCandidate.name
            }
            if (!newCandidate.presentation.i18n.en.alias && newCandidate.alias) {
                newCandidate.presentation.i18n.en.alias = newCandidate.alias
            }
            if (!newCandidate.presentation.i18n.en.description && newCandidate.description) {
                newCandidate.presentation.i18n.en.description = newCandidate.description
            }
            newCandidate.name = newCandidate.presentation.i18n.en.name
            newCandidate.alias = newCandidate.presentation.i18n.en.alias
            newCandidate.description = newCandidate.presentation.i18n.en.description

            return newCandidate
        },
        [electionEvent]
    )

    const invalidVotePositionChoices = () => {
        const choices: {id: IInvalidVotePosition | "null"; name: string}[] = Object.values(
            IInvalidVotePosition
        ).map((value) => ({
            id: value,
            name: t(`candidateScreen.invalidVotePosition.${value}`),
        }))
        choices.unshift({id: "null", name: t("candidateScreen.invalidVotePosition.null")})
        return choices
    }

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    const formValidator = (values: any): any => {
        const errors: any = {dates: {}}
        if (values?.dates?.end_date <= values?.dates?.start_date) {
            errors.dates.end_date = t("electionEventScreen.error.endDate")
        }
        return errors
    }

    const renderTabs = () => {
        let tabNodes: Array<ReactNode> = []

        languageConf.forEach((lang) => {
            tabNodes.push(<Tab key={lang} label={t(`common.language.${lang}`)} id={lang}></Tab>)
        })

        // reset actived tab to first tab if only one
        if (tabNodes.length === 1) {
            setValue(0)
        }

        return tabNodes
    }

    const filterString = (input: string): string => input.replace(/[^a-zA-Z0-9-.]/g, "")

    const addUrlToPresentation = (
        newCandidate: Sequent_Backend_Candidate,
        imageDocumentId?: string,
        name?: string
    ) => {
        let imgUrlBase = getImageUrl(newCandidate?.tenant_id, imageDocumentId, name)
        let imgUrl: ICandidateUrl = {
            url: imgUrlBase,
            is_image: true,
        }
        let presentation = cloneDeep(
            (newCandidate.presentation as ICandidatePresentation | undefined) ?? {}
        )
        let urls = presentation.urls ?? []
        urls = urls.filter((url) => !url.is_image)
        urls.push(imgUrl)
        presentation.urls = urls

        return presentation
    }

    const removeUrlFromPresentation = (newCandidate: Sequent_Backend_Candidate) => {
        let presentation = cloneDeep(
            (newCandidate.presentation as ICandidatePresentation | undefined) ?? {}
        )
        let urls = presentation.urls ?? []
        urls = urls.filter((url) => !url.is_image)
        presentation.urls = urls

        return presentation
    }

    const handleFiles = async (files: FileList | null) => {
        // https://fullstackdojo.medium.com/s3-upload-with-presigned-url-react-and-nodejs-b77f348d54cc

        const theFile = files?.[0]

        if (theFile) {
            let name = filterString(theFile.name)
            let {data, errors} = await getUploadUrl({
                variables: {
                    name: name,
                    media_type: theFile.type,
                    size: theFile.size,
                },
            })
            if (data?.get_upload_url?.document_id) {
                console.log("upload :>> ", data)

                try {
                    await fetch(data.get_upload_url.url, {
                        method: "PUT",
                        headers: {
                            "Content-Type": "image/*",
                        },
                        body: theFile,
                    })
                    notify(t("electionScreen.common.fileLoaded"), {type: "success"})

                    let presentation = addUrlToPresentation(
                        record,
                        data.get_upload_url.document_id,
                        name
                    )

                    updateImage("sequent_backend_candidate", {
                        id: record.id,
                        data: {
                            image_document_id: data.get_upload_url.document_id,
                            presentation: presentation,
                        },
                    })

                    refresh()
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

    const removeImage = () => {
        try {
            setEnabledDeleteImage(false)
            let presentation = removeUrlFromPresentation(record)
            updateImage("sequent_backend_candidate", {
                id: record.id,
                data: {
                    image_document_id: null,
                    presentation: presentation,
                },
            })

            setEnabledDeleteImage(true)
            refresh()
        } catch (e) {
            console.log("error :>> ", e)
            notify(t("electionScreen.error.fileError"), {type: "error"})
            setEnabledDeleteImage(true)
        }
    }

    const renderTabContent = () => {
        let tabNodes: Array<ReactNode> = []
        let index = 0
        languageConf.forEach((lang) => {
            tabNodes.push(
                <CustomTabPanel key={lang} value={value} index={index}>
                    <div style={{marginTop: "16px"}}>
                        <TextInput
                            source={`presentation.i18n[${lang}].name`}
                            label={t("electionEventScreen.field.name")}
                        />
                        <TextInput
                            source={`presentation.i18n[${lang}].alias`}
                            label={t("electionEventScreen.field.alias")}
                        />
                        <TextInput
                            source={`presentation.i18n[${lang}].description`}
                            label={t("electionEventScreen.field.description")}
                        />
                        <BooleanInput
                            source={`presentation.is_disabled`}
                            label={t("candidateScreen.edit.isDisabled")}
                        />
                    </div>
                </CustomTabPanel>
            )
            index++
        })
        return tabNodes
    }

    const DeleteImage: React.FC = () => (
        <StyledIconButton onClick={removeImage} disabled={!enabledDeleteImage}>
            {!enabledDeleteImage ? (
                <CircularProgress size="18px" style={{marginRight: "6px"}} />
            ) : null}
            <Icon variant="info" icon={faTrash} fontSize="18px" />
        </StyledIconButton>
    )

    return electionEvent ? (
        <RecordContext.Consumer>
            {(incoming) => {
                const parsedValue = parseValues(incoming as Sequent_Backend_Candidate_Extended)
                const imageUrl = getImageUrl(
                    parsedValue?.tenant_id,
                    parsedValue?.image_document_id,
                    imageData?.name
                )
                return (
                    <SimpleForm
                        validate={formValidator}
                        record={parsedValue}
                        toolbar={<Toolbar>{canEdit && <SaveButton />}</Toolbar>}
                    >
                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "candidate-data-general"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "candidate-data-general"
                                        ? ""
                                        : "candidate-data-general"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="candidate-data-general" />}
                            >
                                <CandidateStyles.Wrapper>
                                    <CandidateStyles.Title>
                                        {t("candidateScreen.edit.general")}
                                    </CandidateStyles.Title>
                                </CandidateStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Tabs value={value} onChange={handleChange}>
                                    {renderTabs()}
                                </Tabs>
                                {renderTabContent()}
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "candidate-data-type"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "candidate-data-type" ? "" : "candidate-data-type"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="candidate-data-type" />}
                            >
                                <CandidateStyles.Wrapper>
                                    <CandidateStyles.Title>
                                        {t("candidateScreen.edit.type")}
                                    </CandidateStyles.Title>
                                </CandidateStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <TextInput source="type" label={t("candidateScreen.edit.type")} />
                                <TextInput source="presentation.subtype" label="Subtype" />

                                <BooleanInput
                                    source={`presentation.is_explicit_invalid`}
                                    label={t("candidateScreen.edit.isExplicitInvalid")}
                                />
                                <BooleanInput
                                    source={`presentation.is_explicit_blank`}
                                    label={t("candidateScreen.edit.isExplicitBlank")}
                                />
                                <BooleanInput
                                    source={`presentation.is_category_list`}
                                    label={t("candidateScreen.edit.isCategoryList")}
                                />
                                <BooleanInput
                                    source={`presentation.is_write_in`}
                                    label={t("candidateScreen.edit.isWriteIn")}
                                />

                                <SelectInput
                                    source={`presentation.invalid_vote_position`}
                                    choices={invalidVotePositionChoices()}
                                    format={(value) => (typeof value == "string" ? value : "null")}
                                    parse={(value) => (value == "null" ? null : value)}
                                    label={t(`candidateScreen.invalidVotePosition.label`)}
                                    emptyValue={t(`candidateScreen.invalidVotePosition.null`)}
                                    defaultValue={null}
                                    validate={required()}
                                />
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-image"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "election-data-image" ? "" : "election-data-image"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-image" />}
                            >
                                <CandidateStyles.Wrapper
                                    style={{width: "100%", flexDirection: "row"}}
                                >
                                    <>
                                        <CandidateStyles.Title>
                                            {t("electionScreen.edit.image")}
                                        </CandidateStyles.Title>
                                        {parsedValue?.image_document_id ? <DeleteImage /> : null}
                                    </>
                                </CandidateStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Grid container spacing={1}>
                                    <Grid size={2}>
                                        {parsedValue?.image_document_id &&
                                        parsedValue?.image_document_id !== "" ? (
                                            <img
                                                width={200}
                                                height={200}
                                                src={`${globalSettings.PUBLIC_BUCKET_URL}${imageUrl}`}
                                                alt={imageUrl}
                                            />
                                        ) : null}
                                    </Grid>
                                    <Grid size={10}>
                                        <DropFile
                                            handleFiles={async (files) => handleFiles(files)}
                                        />
                                    </Grid>
                                </Grid>
                            </AccordionDetails>
                        </Accordion>
                    </SimpleForm>
                )
            }}
        </RecordContext.Consumer>
    ) : null
}
