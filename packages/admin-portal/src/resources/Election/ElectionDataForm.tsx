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
    FileField,
    FileInput,
} from "react-admin"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Tabs,
    Tab,
    Grid,
    Radio,
    FormControl,
    RadioGroup,
    FormControlLabel,
} from "@mui/material"
import {
    CreateScheduledEventMutation,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
} from "../../gql/graphql"
import React, {useEffect, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {getConfigCreatedStatus} from "../../services/ElectionEventStatus"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {ElectionStyles} from "../../components/styles/ElectionStyles"
import {DropFile} from "@sequentech/ui-essentials"
import {useFormState, useForm} from "react-hook-form"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {JsonField, JsonInput} from "react-admin-json-view"

export const ElectionDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const [tenantId] = useTenantStore()
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const refresh = useRefresh()
    const {t} = useTranslation()
    const form = useForm()

    const [showMenu, setShowMenu] = useState(false)
    const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)
    const [showProgress, setShowProgress] = useState(false)
    const [showCreateKeysDialog, setShowCreateKeysDialog] = useState(false)
    const [showStartTallyDialog, setShowStartTallyDialog] = useState(false)

    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("election-data-general")
    const [defaultLangValue, setDefaultLangValue] = useState<string>("")
    const [jsonConfiguration, setJsonConfiguration] = useState<any>({})

    const {data} = useGetOne("sequent_backend_election_event", {
        id: record.election_event_id,
    })

    const buildLanguageSettings = () => {
        const tempSettings = data?.presentation?.language_conf?.enabled_language_codes

        const temp = []
        if (tempSettings) {
            for (const item of tempSettings) {
                const enabled_item: any = {}
                enabled_item[item] = true
                temp.push(enabled_item)
            }
        }

        return temp
    }

    const parseValues = (incoming: any) => {
        console.log("incoming :>> ", incoming)

        const temp = {...incoming}

        const languageSettings = buildLanguageSettings()
        const votingSettings = data?.voting_channels

        // languages
        // temp.configuration = {...jsonConfiguration}
        temp.enabled_languages = {}

        if (languageSettings) {
            if (
                incoming?.presentation?.language_conf?.enabled_language_codes &&
                incoming?.presentation?.language_conf?.enabled_language_codes.length > 0
            ) {
                // if presentation has lang then set from event
                setDefaultLangValue(incoming?.presentation?.language_conf?.default_language_code)
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

    const handleActionsButtonClick: React.MouseEventHandler<HTMLButtonElement> = (event) => {
        setAnchorEl(event.currentTarget)
        setShowMenu(true)
    }

    const createBulletinBoardAction = async () => {
        setShowMenu(false)
        setShowProgress(true)

        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: tenantId,
                electionEventId: record.id,
                eventProcessor: ScheduledEventType.CREATE_BOARD,
                cronConfig: undefined,
                eventPayload: {},
                createdBy: "admin",
            },
        })
        if (errors) {
            console.log(errors)
        }
        if (data) {
            console.log(data)
        }
        setShowProgress(false)
        refresh()
    }

    const setPublicKeysAction = async () => {
        setShowMenu(false)
        setShowProgress(true)

        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: tenantId,
                electionEventId: record.id,
                eventProcessor: ScheduledEventType.SET_PUBLIC_KEY,
                cronConfig: undefined,
                eventPayload: {},
                createdBy: "admin",
            },
        })
        if (errors) {
            console.log(errors)
        }
        if (data) {
            console.log(data)
        }
        setShowProgress(false)
        refresh()
    }

    const openKeysDialog = () => {
        console.log("opening...")
        setShowCreateKeysDialog(true)
    }

    const openStartTallyDialog = () => {
        console.log("opening...")
        setShowStartTallyDialog(true)
    }

    const createBallotStylesAction = async () => {
        setShowMenu(false)
        setShowProgress(true)

        const {data, errors} = await createScheduledEvent({
            variables: {
                tenantId: tenantId,
                electionEventId: record.id,
                eventProcessor: ScheduledEventType.CREATE_ELECTION_EVENT_BALLOT_STYLES,
                cronConfig: undefined,
                eventPayload: {},
                createdBy: "admin",
            },
        })
        if (errors) {
            console.log(errors)
        }
        if (data) {
            console.log(data)
        }
        setShowProgress(false)
        refresh()
    }

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    let configCreatedStatus = getConfigCreatedStatus(record.status)

    const formValidator = (values: any): any => {
        const errors: any = {dates: {}}
        if (values?.dates?.start_date && values?.dates?.end_date <= values?.dates?.start_date) {
            errors.dates.end_date = t("electionEventScreen.error.endDate")
        }
        return errors
    }

    const renderLangs = (parsedValue: any) => {
        let langNodes = []
        for (const lang in parsedValue?.enabled_languages) {
            langNodes.push(
                <BooleanInput
                    key={lang}
                    source={`enabled_languages.${lang}`}
                    label={t(`common.language.${lang}`)}
                    helperText={false}
                />
            )
        }
        return <div style={{marginTop: "46px"}}>{langNodes}</div>
    }

    const renderDefaultLangs = (parsedValue: any) => {
        let langNodes = []
        for (const lang in parsedValue?.enabled_languages) {
            langNodes.push({id: lang, name: t(`electionScreen.edit.default`)})
        }
        return <RadioButtonGroupInput source="defaultLanguage" choices={langNodes} row={true} />
    }

    const renderVotingChannels = (parsedValue: any) => {
        let channelNodes = []
        for (const channel in parsedValue?.voting_channels) {
            channelNodes.push(
                <BooleanInput
                    key={channel}
                    source={`voting_channels[${channel}]`}
                    label={t(`common.channel.${channel}`)}
                />
            )
        }
        return channelNodes
    }

    // const getJsonText = (json: any) => {

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

    const getJsonText = (value: any, parsedValue: any) => {
        var url = URL.createObjectURL(value) //Create Object URL
        var xhr = new XMLHttpRequest()
        xhr.open("GET", url, false) //Synchronous XMLHttpRequest on Object URL
        xhr.overrideMimeType("text/plain; charset=x-user-defined") //Override MIME Type to prevent UTF-8 related errors
        xhr.send()
        URL.revokeObjectURL(url)
        var returnText = ""
        for (var i = 0; i < xhr.responseText.length; i++) {
            returnText += String.fromCharCode(xhr.responseText.charCodeAt(i) & 0xff)
        } //remove higher byte
        return {
            ...parsedValue.configuration,
            ...(JSON.parse(returnText as string) || {}),
        }
    }

    // TODO: renderReceipts

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
                            expanded={expanded === "election-data-general"}
                            onChange={() => setExpanded("election-data-general")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-general" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.general")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
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
                            expanded={expanded === "election-data-dates"}
                            onChange={() => setExpanded("election-data-dates")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-dates" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.dates")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Grid container spacing={4}>
                                    <Grid item xs={12} md={6}>
                                        <DateTimeInput
                                            source={`dates.start_date`}
                                            label={t("electionScreen.field.startDateTime")}
                                            parse={(value) => new Date(value).toISOString()}
                                        />
                                    </Grid>
                                    <Grid item xs={12} md={6}>
                                        <DateTimeInput
                                            source="dates.end_date"
                                            label={t("electionScreen.field.endDateTime")}
                                            parse={(value) => new Date(value).toISOString()}
                                        />
                                    </Grid>
                                </Grid>
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-language"}
                            onChange={() => setExpanded("election-data-language")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-language" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.language")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <ElectionStyles.AccordionContainer>
                                    <ElectionStyles.AccordionWrapper>
                                        {renderLangs(parsedValue)}
                                        {renderDefaultLangs(parsedValue)}
                                    </ElectionStyles.AccordionWrapper>
                                </ElectionStyles.AccordionContainer>
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-allowed"}
                            onChange={() => setExpanded("election-data-allowed")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-allowed" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.allowed")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Grid container spacing={4}>
                                    <Grid item xs={12} md={6}>
                                        {renderVotingChannels(parsedValue)}
                                    </Grid>
                                </Grid>
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-receipts"}
                            onChange={() => setExpanded("election-data-receipts")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-receipts" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.receipts")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <ElectionStyles.AccordionContainer>
                                    <ElectionStyles.AccordionWrapper alignment="center">
                                        <BooleanInput
                                            source="allowed.sms"
                                            label={"SMS"}
                                            defaultValue={true}
                                        />
                                        <SelectInput
                                            source="template.sms"
                                            choices={[
                                                {id: "tech", name: "Tech"},
                                                {id: "lifestyle", name: "Lifestyle"},
                                                {id: "people", name: "People"},
                                            ]}
                                        />
                                    </ElectionStyles.AccordionWrapper>
                                    <ElectionStyles.AccordionWrapper alignment="center">
                                        <BooleanInput
                                            source="allowed.email"
                                            label={"EMAIL"}
                                            defaultValue={true}
                                        />
                                        <SelectInput
                                            source="template.email"
                                            choices={[
                                                {id: "tech", name: "Tech"},
                                                {id: "lifestyle", name: "Lifestyle"},
                                                {id: "people", name: "People"},
                                            ]}
                                        />
                                    </ElectionStyles.AccordionWrapper>
                                    <ElectionStyles.AccordionWrapper alignment="center">
                                        <BooleanInput
                                            source="allowed.print"
                                            label={"PRINT"}
                                            defaultValue={true}
                                        />
                                        <SelectInput
                                            source="template.print"
                                            choices={[
                                                {id: "tech", name: "Tech"},
                                                {id: "lifestyle", name: "Lifestyle"},
                                                {id: "people", name: "People"},
                                            ]}
                                        />
                                    </ElectionStyles.AccordionWrapper>
                                </ElectionStyles.AccordionContainer>
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
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.image")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
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
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.advanced")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <FileInput
                                    source="configuration"
                                    accept={"application/json"}
                                    parse={(value) => {
                                        return getJsonText(value, parsedValue)
                                    }}
                                />

                                <JsonField
                                    source="presentation"
                                    reactJsonOptions={{
                                        name: null,
                                        collapsed: true,
                                        enableClipboard: true,
                                        displayDataTypes: false,
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
