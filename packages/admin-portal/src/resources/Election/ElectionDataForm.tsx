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
    Checkbox,
    FormControlLabel,
} from "@mui/material"
import {CreateScheduledEventMutation, Sequent_Backend_Election_Event} from "../../gql/graphql"
import React, {useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {CREATE_SCHEDULED_EVENT} from "../../queries/CreateScheduledEvent"
import {ScheduledEventType} from "../../services/ScheduledEvent"
import {getConfigCreatedStatus} from "../../services/ElectionEventStatus"
import {useMutation} from "@apollo/client"
import {useTenantStore} from "../../components/CustomMenu"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {ElectionStyles} from "../../components/styles/ElectionStyles"
import {DropFile} from "@sequentech/ui-essentials"
import {useFormState, useForm} from "react-hook-form"

export const EditElectionDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [expanded, setExpanded] = useState("election-data-general")
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

    const form = useForm()

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

    return (
        <SimpleForm validate={formValidator}>
            <Accordion
                sx={{width: "100%"}}
                expanded={expanded === "election-data-general"}
                onChange={() => setExpanded("election-data-general")}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-data-general" />}>
                    <ElectionStyles.Wrapper>
                        <ElectionStyles.Title>
                            {t("electionScreen.edit.general")}
                        </ElectionStyles.Title>
                    </ElectionStyles.Wrapper>
                </AccordionSummary>
                <AccordionDetails>
                    <Tabs value={value} onChange={handleChange}>
                        <Tab label="English" id="tab-1"></Tab>
                        <Tab label="Spanish" id="tab-2"></Tab>
                    </Tabs>
                    <CustomTabPanel value={value} index={0}>
                        <div style={{marginTop: "16px"}}>
                            <TextInput source="name" label={t("electionScreen.field.name")} />
                            <TextInput source="alias" label={t("electionScreen.field.alias")} />
                            <TextInput
                                source="description"
                                label={t("electionScreen.field.description")}
                            />
                        </div>
                    </CustomTabPanel>
                    <CustomTabPanel value={value} index={1}>
                        <div style={{marginTop: "16px"}}>
                            <TextInput source="name" label={t("electionScreen.field.name")} />
                            <TextInput source="alias" label={t("electionScreen.field.alias")} />
                            <TextInput
                                source="description"
                                label={t("electionScreen.field.description")}
                            />
                        </div>
                    </CustomTabPanel>
                </AccordionDetails>
            </Accordion>

            <Accordion
                sx={{width: "100%"}}
                expanded={expanded === "election-data-dates"}
                onChange={() => setExpanded("election-data-dates")}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-data-dates" />}>
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
                expanded={expanded === "election-data-language"}
                onChange={() => setExpanded("election-data-language")}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-data-language" />}>
                    <ElectionStyles.Wrapper>
                        <ElectionStyles.Title>
                            {t("electionScreen.edit.language")}
                        </ElectionStyles.Title>
                    </ElectionStyles.Wrapper>
                </AccordionSummary>
                <AccordionDetails>
                    <ElectionStyles.AccordionContainer>
                        <ElectionStyles.AccordionWrapper>
                            <BooleanInput
                                source="language.english"
                                label={"English"}
                                defaultValue={true}
                            />
                            <FormControlLabel
                                control={<Checkbox checked={true} />}
                                label={t("electionScreen.edit.default")}
                            />
                        </ElectionStyles.AccordionWrapper>
                        <ElectionStyles.AccordionWrapper>
                            <BooleanInput
                                source="language.spanish"
                                label={"Spanish"}
                                defaultValue={false}
                            />
                            <FormControlLabel
                                control={<Checkbox checked={true} />}
                                label={t("electionScreen.edit.default")}
                            />
                        </ElectionStyles.AccordionWrapper>
                    </ElectionStyles.AccordionContainer>
                </AccordionDetails>
            </Accordion>

            <Accordion
                sx={{width: "100%"}}
                expanded={expanded === "election-data-allowed"}
                onChange={() => setExpanded("election-data-allowed")}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-data-allowed" />}>
                    <ElectionStyles.Wrapper>
                        <ElectionStyles.Title>
                            {t("electionScreen.edit.allowed")}
                        </ElectionStyles.Title>
                    </ElectionStyles.Wrapper>
                </AccordionSummary>
                <AccordionDetails>
                    <Grid container spacing={4}>
                        <Grid item xs={12} md={6}>
                            <BooleanInput source="allowed.one" label={"One"} defaultValue={true} />
                            <BooleanInput source="allowed.two" label={"Two"} defaultValue={true} />
                        </Grid>
                    </Grid>
                </AccordionDetails>
            </Accordion>

            <Accordion
                sx={{width: "100%"}}
                expanded={expanded === "election-data-receipts"}
                onChange={() => setExpanded("election-data-receipts")}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-data-receipts" />}>
                    <ElectionStyles.Wrapper>
                        <ElectionStyles.Title>
                            {t("electionScreen.edit.receipts")}
                        </ElectionStyles.Title>
                    </ElectionStyles.Wrapper>
                </AccordionSummary>
                <AccordionDetails>
                    <ElectionStyles.AccordionContainer>
                        <ElectionStyles.AccordionWrapper alignment="center">
                            <BooleanInput source="allowed.sms" label={"SMS"} defaultValue={true} />
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
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-data-image" />}>
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
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-data-advanced" />}>
                    <ElectionStyles.Wrapper>
                        <ElectionStyles.Title>
                            {t("electionScreen.edit.advanced")}
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
        </SimpleForm>
    )
}
