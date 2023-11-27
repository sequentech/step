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
    RecordContext,
    Toolbar,
    SaveButton
} from "react-admin"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Tabs,
    Tab,
} from "@mui/material"
import {
    CreateScheduledEventMutation,
    Sequent_Backend_Candidate,
} from "../../gql/graphql"
import React, {useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {DropFile} from "@sequentech/ui-essentials"
import {useForm} from "react-hook-form"
import { CandidateStyles } from '../../components/styles/CandidateStyles'
import { useTenantStore } from '../../providers/TenantContextProvider'
import { CANDIDATE_TYPES } from './constants'

export const CandidateDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Candidate>()
    const [tenantId] = useTenantStore()
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const refresh = useRefresh()
    const {t} = useTranslation()


    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("candidate-data-general")
    // const [defaultLangValue, setDefaultLangValue] = useState<string>("")

    const {data} = useGetOne("sequent_backend_election_event", {
        id: record.election_event_id,
    })

    const buildLanguageSettings = () => {
        const tempSettings = data?.presentation?.language_conf?.enabled_language_codes
        const temp = []
        for (const item of tempSettings) {
            const enabled_item: any = {}
            enabled_item[item] = true
            temp.push(enabled_item)
        }
        return temp
    }

    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        const languageSettings = buildLanguageSettings()
        const votingSettings = data?.voting_channels

        // languages
        temp.enabled_languages = {}

        if (languageSettings) {
            if (
                incoming?.presentation?.language_conf?.enabled_language_codes &&
                incoming?.presentation?.language_conf?.enabled_language_codes.length > 0
            ) {
                // if presentation has lang then set from event
                // setDefaultLangValue(incoming?.presentation?.language_conf?.default_language_code)
                temp.defaultLanguage = incoming?.presentation?.language_conf?.default_language_code
                for (const setting of languageSettings) {
                    const enabled_item: any = {}

                    const isInEnabled =
                        incoming?.presentation?.language_conf?.enabled_language_codes.length > 0
                            ? incoming?.presentation?.language_conf?.enabled_language_codes.find(
                                  (item: any) => Object.keys(setting)[0] === item
                              )
                            : false

                    if (isInEnabled) {
                        enabled_item[Object.keys(setting)[0]] = true
                    } else {
                        enabled_item[Object.keys(setting)[0]] = false // setting[Object.keys(setting)[0]]
                    }
                    temp.enabled_languages = {...temp.enabled_languages, ...enabled_item}
                }
            } else {
                // if presentation has no lang then use always de default settings
                temp.defaultLanguage = ""
                for (const item of languageSettings) {
                    temp.enabled_languages = {...temp.enabled_languages, ...item}
                }
            }
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

        return temp
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
                const parsedValue = parseValues(incoming)
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
                            expanded={expanded === "candidate-data-image"}
                            onChange={() => setExpanded("candidate-data-image")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="candidate-data-image" />}
                            >
                                <CandidateStyles.Wrapper>
                                    <CandidateStyles.Title>
                                        {t("candidateScreen.edit.image")}
                                    </CandidateStyles.Title>
                                </CandidateStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <DropFile
                                    handleFiles={function (files: FileList): void | Promise<void> {
                                        throw new Error("Function not implemented.")
                                    }}
                                />
                            </AccordionDetails>
                        </Accordion>
                    </SimpleForm>
                )
            }}
        </RecordContext.Consumer>
    ) : null
}
