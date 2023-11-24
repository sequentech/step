// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    BooleanInput,
    DateTimeInput,
    SelectInput,
    TextInput,
    useRecordContext,
    useRefresh,
    SimpleForm,
    useGetOne,
    RecordContext,
    RadioButtonGroupInput,
    NumberInput,
} from "react-admin"
import {Accordion, AccordionDetails, AccordionSummary, Tabs, Tab, Grid} from "@mui/material"
import {CreateScheduledEventMutation, Sequent_Backend_Contest} from "../../gql/graphql"
import React, {useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {DropFile} from "@sequentech/ui-essentials"
import {useForm} from "react-hook-form"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {COUNTING_ALGORITHMS, VOTING_TYPES} from "./constants"
import {ContestStyles} from "../../components/styles/ContestStyles"
import FileJsonInput from "../../components/FileJsonInput"

export const ContestDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Contest>()
    const [tenantId] = useTenantStore()
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const refresh = useRefresh()
    const {t} = useTranslation()

    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("contest-data-general")
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
                    <SimpleForm validate={formValidator} record={parsedValue}>
                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "contest-data-general"}
                            onChange={() => setExpanded("contest-data-general")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="contest-data-general" />}
                            >
                                <ContestStyles.Wrapper>
                                    <ContestStyles.Title>
                                        {t("contestScreen.edit.general")}
                                    </ContestStyles.Title>
                                </ContestStyles.Wrapper>
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
                            expanded={expanded === "contest-data-system"}
                            onChange={() => setExpanded("contest-data-system")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="contest-data-system" />}
                            >
                                <ContestStyles.Wrapper>
                                    <ContestStyles.Title>
                                        {t("contestScreen.edit.system")}
                                    </ContestStyles.Title>
                                </ContestStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <SelectInput source="voting_type" choices={VOTING_TYPES} />
                                <SelectInput
                                    source="counting_algorithm"
                                    choices={COUNTING_ALGORITHMS}
                                />
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "contest-data-design"}
                            onChange={() => setExpanded("contest-data-design")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="contest-data-design" />}
                            >
                                <ContestStyles.Wrapper>
                                    <ContestStyles.Title>
                                        {t("contestScreen.edit.design")}
                                    </ContestStyles.Title>
                                </ContestStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <BooleanInput source="is_acclaimed" />
                                <NumberInput source="min_votes" />
                                <NumberInput source="max_votes" />
                                <NumberInput source="winning_candidates_num" />
                                <SelectInput source="voting_type" choices={VOTING_TYPES} />
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
                                <ContestStyles.Wrapper>
                                    <ContestStyles.Title>
                                        {t("electionScreen.edit.image")}
                                    </ContestStyles.Title>
                                </ContestStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <DropFile
                                    handleFiles={function (files: FileList): void | Promise<void> {
                                        throw new Error("Function not implemented.")
                                    }}
                                />
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-advanced"}
                            onChange={() => setExpanded("election-data-advanced")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-advanced" />}
                            >
                                <ContestStyles.Wrapper>
                                    <ContestStyles.Title>
                                        {t("electionScreen.edit.advanced")}
                                    </ContestStyles.Title>
                                </ContestStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <FileJsonInput
                                    parsedValue={parsedValue}
                                    fileSource="configuration"
                                    jsonSource="presentation"
                                />
                            </AccordionDetails>
                        </Accordion>
                    </SimpleForm>
                )
            }}
        </RecordContext.Consumer>
    ) : null
}
