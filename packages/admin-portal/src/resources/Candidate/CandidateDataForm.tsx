// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    SelectInput,
    TextInput,
    useRecordContext,
    useRefresh,
    SimpleForm,
    useGetOne,
    Toolbar,
    SaveButton,
    useUpdate,
    useNotify,
    RaRecord,
    Identifier,
    RecordContext,
} from "react-admin"
import {Accordion, AccordionDetails, AccordionSummary, Tabs, Tab, Grid} from "@mui/material"
import {
    GetUploadUrlMutation,
    Sequent_Backend_Candidate,
    Sequent_Backend_Document,
    Sequent_Backend_Election_Event,
} from "../../gql/graphql"
import React, {useCallback, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {DropFile} from "@sequentech/ui-essentials"
import {CandidateStyles} from "../../components/styles/CandidateStyles"
import {CANDIDATE_TYPES} from "./constants"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"

export type Sequent_Backend_Candidate_Extended = RaRecord<Identifier> & {
    enabled_languages?: {[key: string]: boolean}
    defaultLanguage?: string
}

export const CandidateDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Candidate>()

    const {t} = useTranslation()
    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const notify = useNotify()
    const refresh = useRefresh()

    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("candidate-data-general")
    // const [defaultLangValue, setDefaultLangValue] = useState<string>("")

    const {data} = useGetOne<Sequent_Backend_Election_Event>("sequent_backend_election_event", {
        id: record.election_event_id,
    })

    const {data: imageData, refetch: refetchImage} = useGetOne<Sequent_Backend_Document>(
        "sequent_backend_document",
        {
            id: record.image_document_id || record.tenant_id,
            meta: {tenant_id: record.tenant_id},
        }
    )

    const [updateImage] = useUpdate()

    const buildLanguageSettings = () => {
        const tempSettings = data?.presentation?.language_conf?.enabled_language_codes || []
        const temp = []
        for (const item of tempSettings) {
            const enabled_item: any = {}
            enabled_item[item] = true
            temp.push(enabled_item)
        }
        return temp
    }

    const parseValues = useCallback(
        (incoming: Sequent_Backend_Candidate_Extended): Sequent_Backend_Candidate_Extended => {
            if (!data) {
                return incoming as Sequent_Backend_Candidate_Extended
            }
            const temp = {...(incoming as Sequent_Backend_Candidate_Extended)}

            let languageSettings
            // const languageSettings = buildLanguageSettings()
            const votingSettings = data?.voting_channels

            // languages
            temp.enabled_languages = {}

            if (
                incoming?.presentation?.language_conf?.enabled_language_codes &&
                incoming?.presentation?.language_conf?.enabled_language_codes.length > 0
            ) {
                languageSettings = incoming?.presentation?.language_conf?.enabled_language_codes

                // if presentation has lang then set from event
                // setDefaultLangValue(incoming?.presentation?.language_conf?.default_language_code)
                temp.defaultLanguage = incoming?.presentation?.language_conf?.default_language_code
                for (const setting of languageSettings) {
                    const enabled_item: any = {}

                    const isInEnabled =
                        incoming?.presentation?.language_conf?.enabled_language_codes.length > 0
                            ? incoming?.presentation?.language_conf?.enabled_language_codes.find(
                                  (item: any) => setting === item
                              )
                            : false

                    if (isInEnabled) {
                        enabled_item[setting] = true
                    } else {
                        enabled_item[setting] = false // setting[Object.keys(setting)[0]]
                    }

                    temp.enabled_languages = {
                        ...temp.enabled_languages,
                        ...enabled_item,
                    }
                }
            } else {
                // if presentation has no lang then use always de default settings
                languageSettings = buildLanguageSettings()

                temp.defaultLanguage = ""
                let enabled_items: any = {}
                for (const item of languageSettings) {
                    enabled_items = {...enabled_items, ...item}
                }
                temp.enabled_languages = {...temp.enabled_languages, ...enabled_items}
            }

            // set english first lang always
            if (temp.enabled_languages) {
                const en = {en: temp.enabled_languages["en"]}
                delete temp.enabled_languages.en
                const rest = temp.enabled_languages
                temp.enabled_languages = {...en, ...rest}
            }

            // voting channels
            const all_channels = {...incoming?.voting_channels}
            delete incoming.voting_channels
            temp.voting_channels = {}
            for (const setting in votingSettings) {
                const enabled_item: any = {}
                enabled_item[setting] =
                    setting in all_channels ? all_channels[setting] : votingSettings[setting]
                temp.voting_channels = {...temp.voting_channels, ...enabled_item}
            }

            // name, alias and description fields
            if (!temp.presentation || !temp.presentation?.i18n) {
                temp.presentation = {i18n: {en: {}}}
            }
            temp.presentation.i18n.en.name = temp.name
            temp.presentation.i18n.en.alias = temp.alias
            temp.presentation.i18n.en.description = temp.description

            return temp
        },
        [data]
    )

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

    const renderTabs = (parsedValue: any) => {
        let tabNodes = []
        for (const lang in parsedValue?.enabled_languages) {
            if (parsedValue?.enabled_languages[lang]) {
                tabNodes.push(<Tab key={lang} label={t(`common.language.${lang}`)} id={lang}></Tab>)
            }
        }

        // reset actived tab to first tab if only one
        if (tabNodes.length === 1) {
            setValue(0)
        }

        return tabNodes
    }

    const handleFiles = async (files: FileList | null) => {
        // https://fullstackdojo.medium.com/s3-upload-with-presigned-url-react-and-nodejs-b77f348d54cc

        const theFile = files?.[0]

        if (theFile) {
            let {data, errors} = await getUploadUrl({
                variables: {
                    name: theFile.name,
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

                    updateImage("sequent_backend_candidate", {
                        id: record.id,
                        data: {
                            image_document_id: data.get_upload_url.document_id,
                        },
                    })

                    refetchImage()
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

    const renderTabContent = (parsedValue: any) => {
        let tabNodes = []
        let index = 0
        for (const lang in parsedValue?.enabled_languages) {
            if (parsedValue?.enabled_languages[lang]) {
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
                        </div>
                    </CustomTabPanel>
                )
                index++
            }
        }
        return tabNodes
    }

    return data ? (
        <RecordContext.Consumer>
            {(incoming) => {
                const parsedValue = parseValues(incoming as Sequent_Backend_Candidate_Extended)
                console.log("parsedValue :>> ", parsedValue)
                return (
                    <SimpleForm
                        validate={formValidator}
                        record={parsedValue}
                        toolbar={
                            <Toolbar>
                                <SaveButton />
                            </Toolbar>
                        }
                    >
                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "candidate-data-general"}
                            onChange={() => setExpanded("candidate-data-general")}
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
                                    {renderTabs(parsedValue)}
                                </Tabs>
                                {renderTabContent(parsedValue)}
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "candidate-data-type"}
                            onChange={() => setExpanded("candidate-data-type")}
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
                                <SelectInput source="type" choices={CANDIDATE_TYPES(t)} />
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-image"}
                            onChange={() => setExpanded("election-data-image")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-image" />}
                            >
                                <CandidateStyles.Wrapper>
                                    <CandidateStyles.Title>
                                        {t("electionScreen.edit.image")}
                                    </CandidateStyles.Title>
                                </CandidateStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Grid container spacing={1}>
                                    <Grid item xs={2}>
                                        {parsedValue?.image_document_id &&
                                        parsedValue?.image_document_id !== "" ? (
                                            <img
                                                width={200}
                                                height={200}
                                                src={`http://localhost:9000/public/tenant-${parsedValue?.tenant_id}/document-${parsedValue?.image_document_id}/${imageData?.name}`}
                                                alt={`tenant-${parsedValue?.tenant_id}/document-${parsedValue?.image_document_id}/${imageData?.name}`}
                                            />
                                        ) : null}
                                    </Grid>
                                    <Grid item xs={10}>
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
