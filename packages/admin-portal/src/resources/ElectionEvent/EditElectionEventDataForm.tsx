// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    BooleanInput,
    DateTimeInput,
    RecordContext,
    SimpleForm,
    TextInput,
    useRecordContext,
    useRefresh,
} from "react-admin"
import {Accordion, AccordionDetails, AccordionSummary, Tabs, Tab, Grid, Button} from "@mui/material"
import {CreateScheduledEventMutation, GetUploadUrlMutation, Sequent_Backend_Election_Event} from "../../gql/graphql"
import React, {useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {getConfigCreatedStatus} from "../../services/ElectionEventStatus"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {ElectionHeaderStyles} from "../../components/styles/ElectionHeaderStyles"
import {useTenantStore} from "../../providers/TenantContextProvider"
import { GET_UPLOAD_URL } from '@/queries/GetUploadUrl'

export const EditElectionEventDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [tenantId] = useTenantStore()
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    //const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const refresh = useRefresh()

    const [showMenu, setShowMenu] = useState(false)
    const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)
    const [showProgress, setShowProgress] = useState(false)
    const [showCreateKeysDialog, setShowCreateKeysDialog] = useState(false)
    const [showStartTallyDialog, setShowStartTallyDialog] = useState(false)
    const {t} = useTranslation()

    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("election-event-data-general")
    const [languageSettings] = useState<any>([{es: true}, {en: true}])
    const [votingSettings] = useState<any>({online: true, kiosk: true})

    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        // languages
        temp.enabled_languages = {}

        if (
            incoming?.presentation?.language_conf?.enabled_language_codes &&
            incoming?.presentation?.language_conf?.enabled_language_codes.length > 0
        ) {
            // if presentation has lang then set from event
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
            for (const item of languageSettings) {
                temp.enabled_languages = {...temp.enabled_languages, ...item}
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
                />
            )
        }
        return <div>{langNodes}</div>
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

    return (
        <RecordContext.Consumer>
            {(incoming) => {
                const parsedValue = parseValues(incoming)
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
                                </Tabs>
                                {renderTabContent(parsedValue)}
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
                                        {renderVotingChannels(parsedValue)}
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
