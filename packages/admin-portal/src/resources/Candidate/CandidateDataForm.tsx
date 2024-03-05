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
import React, {ReactNode, useCallback, useContext, useEffect, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {
    DropFile,
    ICandidatePresentation,
    IElectionEventPresentation,
    ILanguageConf,
} from "@sequentech/ui-essentials"
import {CandidateStyles} from "../../components/styles/CandidateStyles"
import {CANDIDATE_TYPES} from "./constants"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {cloneDeep} from "lodash"

export type Sequent_Backend_Candidate_Extended = Sequent_Backend_Candidate &
    RaRecord<Identifier> & {
        enabled_languages?: {[key: string]: boolean}
        defaultLanguage?: string
    }

export const CandidateDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Candidate>()

    const {t} = useTranslation()
    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const [languageConf, setLanguageConf] = useState<ILanguageConf>({
        enabled_language_codes: ["en"],
        default_language_code: "en",
    })
    const notify = useNotify()
    const refresh = useRefresh()
    const {globalSettings} = useContext(SettingsContext)

    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("candidate-data-general")
    // const [defaultLangValue, setDefaultLangValue] = useState<string>("")

    const {data: electionEvent} = useGetOne<Sequent_Backend_Election_Event>(
        "sequent_backend_election_event",
        {
            id: record.election_event_id,
        }
    )

    const {data: imageData, refetch: refetchImage} = useGetOne<Sequent_Backend_Document>(
        "sequent_backend_document",
        {
            id: record.image_document_id || record.tenant_id,
            meta: {tenant_id: record.tenant_id},
        }
    )

    useEffect(() => {
        if (!electionEvent) {
            return
        }
        let presentation = electionEvent.presentation as IElectionEventPresentation | undefined
        if (!presentation?.language_conf) {
            return
        }
        setLanguageConf(presentation.language_conf)
    }, [electionEvent?.presentation?.language_conf])

    const [updateImage] = useUpdate()

    const parseValues = useCallback(
        (incoming: Sequent_Backend_Candidate_Extended): Sequent_Backend_Candidate_Extended => {
            if (!electionEvent) {
                return incoming
            }
            const newCandidate: Sequent_Backend_Candidate_Extended = cloneDeep(incoming)
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

        languageConf.enabled_language_codes?.forEach((lang) => {
            tabNodes.push(<Tab key={lang} label={t(`common.language.${lang}`)} id={lang}></Tab>)
        })

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

    const renderTabContent = () => {
        let tabNodes: Array<ReactNode> = []
        let index = 0
        languageConf.enabled_language_codes?.forEach((lang) => {
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
    const renderTabContent0 = (parsedValue: any) => {
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
                            <BooleanInput
                                source={`presentation.is_disabled`}
                                label={t("candidateScreen.edit.isDisabled")}
                            />
                        </div>
                    </CustomTabPanel>
                )
                index++
            }
        }
        return tabNodes
    }

    return electionEvent ? (
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
                                    {renderTabs()}
                                </Tabs>
                                {renderTabContent()}
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
                                                src={`${globalSettings.PUBLIC_BUCKET_URL}tenant-${parsedValue?.tenant_id}/document-${parsedValue?.image_document_id}/${imageData?.name}`}
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
