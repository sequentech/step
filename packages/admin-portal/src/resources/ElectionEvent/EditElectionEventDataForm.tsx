// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    BooleanInput,
    DateField,
    DateInput,
    DateTimeInput,
    Edit,
    EditBase,
    RecordContext,
    ReferenceManyField,
    SelectInput,
    SimpleForm,
    TabbedForm,
    TabbedShowLayout,
    TextField,
    TextInput,
    useRecordContext,
    useRefresh,
} from "react-admin"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Button,
    Tabs,
    Tab,
    CircularProgress,
    Menu,
    MenuItem,
    Typography,
    Grid,
} from "@mui/material"
import {CreateScheduledEventMutation, Sequent_Backend_Election_Event} from "../../gql/graphql"
import React, {useEffect, useState} from "react"
import {faPieChart, faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ChipList} from "../../components/ChipList"
import {HorizontalBox} from "../../components/HorizontalBox"
import {IconButton} from "@sequentech/ui-essentials"
import {JsonInput} from "react-admin-json-view"
import {KeysGenerationDialog} from "../../components/KeysGenerationDialog"
import {Link} from "react-router-dom"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {StartTallyDialog} from "../../components/StartTallyDialog"
import {getConfigCreatedStatus} from "../../services/ElectionEventStatus"
import {useMutation} from "@apollo/client"
import {useTenantStore} from "../../components/CustomMenu"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {ElectionHeaderStyles} from "../../components/styles/ElectionHeaderStyles"
import {parse} from "path"

export const EditElectionEventDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [expanded, setExpanded] = useState("election-event-data-general")
    const [showMenu, setShowMenu] = useState(false)
    const [value, setValue] = useState(0)
    const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)
    const [showProgress, setShowProgress] = useState(false)
    const [showCreateKeysDialog, setShowCreateKeysDialog] = useState(false)
    const [showStartTallyDialog, setShowStartTallyDialog] = useState(false)
    const [tenantId] = useTenantStore()
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const refresh = useRefresh()
    const {t} = useTranslation()

    const [languageSettings, setLanguageSettings] = useState<any>([{es: true}, {en: true}])

    const parseValues = (data: any) => {
        const temp = {...data}
        if (data.presentation) {
            if (data.presentation.language_conf) {
                temp.enabled_languages = {}

                for (const settingLang of languageSettings) {
                    const enabled_lang: any = {}
                    const isInEnabled = data.presentation.language_conf.enabled_language_codes.find(
                        (item: any) => Object.keys(settingLang)[0] === item
                    )
                    if (isInEnabled) {
                        enabled_lang[Object.keys(settingLang)[0]] = true
                    } else {
                        enabled_lang[Object.keys(settingLang)[0]] = false
                    }

                    temp.enabled_languages = {...temp.enabled_languages, ...enabled_lang}
                }
            }
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
        if (values?.dates?.end_date <= values?.dates?.start_date) {
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
                    source={`enabled_languages[${lang}]`}
                    label={t(`common.language.${lang}`)}
                />
            )
        }
        return langNodes
    }

    const renderTabs = (parsedValue: any) => {
        let tabNodes = []
        for (const lang in parsedValue?.enabled_languages) {
            if (parsedValue?.enabled_languages[lang]) {
                tabNodes.push(<Tab key={lang} label={t(`common.language.${lang}`)} id={lang}></Tab>)
            }
        }
        return tabNodes
    }

    const renderTabContent = (parsedValue: any) => {
        let tabNodes = []
        let index = 0
        for (const lang in parsedValue?.enabled_languages) {
            tabNodes.push(
                <CustomTabPanel key={lang} value={value} index={index}>
                    <div style={{marginTop: "16px"}}>
                        <TextInput source="name" label={t("electionEventScreen.field.name")} />
                        <TextInput source="alias" label={t("electionEventScreen.field.alias")} />
                        <TextInput
                            source="description"
                            label={t("electionEventScreen.field.description")}
                        />
                    </div>
                </CustomTabPanel>
            )
            index++
        }
        return tabNodes
    }

    return (
        <RecordContext.Consumer>
            {(data) => {
                const parsedValue = parseValues(data)
                console.log("parsedValue :>> ", parsedValue)
                return (
                    <SimpleForm validate={formValidator} record={parsedValue}>
                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-event-data-general"}
                            onChange={() => setExpanded("election-event-data-general")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-event-data-general" />}
                            >
                                <ElectionHeaderStyles.Wrapper>
                                    <ElectionHeaderStyles.Title>
                                        {t("electionEventScreen.edit.general")}
                                    </ElectionHeaderStyles.Title>
                                </ElectionHeaderStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Tabs value={value} onChange={handleChange}>
                                    {renderTabs(parsedValue)}
                                    {/* <Tab label="Spanish" id="tab-2"></Tab> */}
                                </Tabs>
                                {renderTabContent(parsedValue)}
                                {/* {parsedValue?.enabled_languages.map((lang: any, index: number) => {
                                    return (
                                        <CustomTabPanel
                                            key={lang}
                                            value={value}
                                            index={index}
                                        >
                                            <div style={{marginTop: "16px"}}>
                                                <TextInput
                                                    source="name"
                                                    label={t("electionEventScreen.field.name")}
                                                />
                                                <TextInput
                                                    source="alias"
                                                    label={t("electionEventScreen.field.alias")}
                                                />
                                                <TextInput
                                                    source="description"
                                                    label={t(
                                                        "electionEventScreen.field.description"
                                                    )}
                                                />
                                            </div>
                                        </CustomTabPanel>
                                    )
                                })} */}
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-event-data-dates"}
                            onChange={() => setExpanded("election-event-data-dates")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-event-data-dates" />}
                            >
                                <ElectionHeaderStyles.Wrapper>
                                    <ElectionHeaderStyles.Title>
                                        {t("electionEventScreen.edit.dates")}
                                    </ElectionHeaderStyles.Title>
                                </ElectionHeaderStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Grid container spacing={4}>
                                    <Grid item xs={12} md={6}>
                                        <DateTimeInput
                                            source="dates.start_date"
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
                            expanded={expanded === "election-event-data-language"}
                            onChange={() => setExpanded("election-event-data-language")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-event-data-language" />}
                            >
                                <ElectionHeaderStyles.Wrapper>
                                    <ElectionHeaderStyles.Title>
                                        {t("electionEventScreen.edit.language")}
                                    </ElectionHeaderStyles.Title>
                                </ElectionHeaderStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Grid container spacing={4}>
                                    <Grid item xs={12} md={6}>
                                        {renderLangs(parsedValue)}
                                        {/* {parsedValue?.enabled_languages.map((lang: any) => {
                                            console.log(
                                                "lang",
                                                `enabled_languages[${Object.keys(lang)[0]}]`,
                                                lang,
                                                lang[Object.keys(lang)[0]]
                                            )
                                            return (
                                                <BooleanInput
                                                    key={Object.keys(lang)[0]}
                                                    source={`lang[${
                                                        Object.keys(lang)[0]
                                                    }]`}
                                                    label={t(
                                                        `common.language.${Object.keys(lang)[0]}`
                                                    )}
                                                    // defaultValue={lang[Object.keys(lang)[0]]}
                                                />
                                            )
                                        })} */}
                                    </Grid>
                                </Grid>
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-event-data-allowed"}
                            onChange={() => setExpanded("election-event-data-allowed")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-event-data-allowed" />}
                            >
                                <ElectionHeaderStyles.Wrapper>
                                    <ElectionHeaderStyles.Title>
                                        {t("electionEventScreen.edit.allowed")}
                                    </ElectionHeaderStyles.Title>
                                </ElectionHeaderStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Grid container spacing={4}>
                                    <Grid item xs={12} md={6}>
                                        <BooleanInput
                                            source="allowed.one"
                                            label={"One"}
                                            defaultValue={true}
                                        />
                                        <BooleanInput
                                            source="allowed.two"
                                            label={"Two"}
                                            defaultValue={true}
                                        />
                                    </Grid>
                                </Grid>
                            </AccordionDetails>
                        </Accordion>
                    </SimpleForm>
                )
            }}
        </RecordContext.Consumer>
    )
}
