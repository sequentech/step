// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BooleanInput, DateTimeInput, SimpleForm, TextInput, useRefresh} from "react-admin"

import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Tabs,
    Tab,
    Typography,
    Grid,
} from "@mui/material"
import React, {useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {ElectionHeaderStyles} from "../../components/styles/ElectionHeaderStyles"

export const EditElectionEventAreasList: React.FC = () => {
    const [expanded, setExpanded] = useState("election-event-data-general")
    const [value, setValue] = useState(0)
    const refresh = useRefresh()
    const {t} = useTranslation()

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    return (
        <SimpleForm>
            <Accordion
                sx={{width: "100%"}}
                expanded={expanded === "election-event-data-general"}
                onChange={() => setExpanded("election-event-data-general")}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-event-data-general" />}>
                    <ElectionHeaderStyles.Wrapper>
                        <ElectionHeaderStyles.Title>
                            {t("electionEventScreen.edit.general")}
                        </ElectionHeaderStyles.Title>
                    </ElectionHeaderStyles.Wrapper>
                </AccordionSummary>
                <AccordionDetails>
                    <Tabs value={value} onChange={handleChange}>
                        <Tab label="English" id="tab-1"></Tab>
                        <Tab label="Spanish" id="tab-2"></Tab>
                    </Tabs>
                    <CustomTabPanel value={value} index={0}>
                        <div style={{marginTop: "16px"}}>
                            <TextInput
                                source="name"
                                label={String(t("electionEventScreen.field.name"))}
                            />
                            <TextInput
                                source="alias"
                                label={String(t("electionEventScreen.field.alias"))}
                            />
                            <TextInput
                                source="description"
                                label={String(t("electionEventScreen.field.description"))}
                            />
                        </div>
                    </CustomTabPanel>
                    <CustomTabPanel value={value} index={1}>
                        <div style={{marginTop: "16px"}}>
                            <TextInput
                                source="name"
                                label={String(t("electionEventScreen.field.name"))}
                            />
                            <TextInput
                                source="alias"
                                label={String(t("electionEventScreen.field.alias"))}
                            />
                            <TextInput
                                source="description"
                                label={String(t("electionEventScreen.field.description"))}
                            />
                        </div>
                    </CustomTabPanel>

                    {/* <TabbedShowLayout>
                        <TabbedShowLayout.Tab label="English" >
                            <TextInput source="name" label={String(t("electionEventScreen.field.name"))} />
                            <TextInput
                                source="alias"
                                label={String(t("electionEventScreen.field.alias"))}
                            />
                            <TextInput
                                source="description"
                                label={String(t("electionEventScreen.field.description"))}
                            />
                        </TabbedShowLayout.Tab>
                        <TabbedShowLayout.Tab label="Spanish" >
                            <TextInput source="name" label={String(t("electionEventScreen.field.name"))} />
                            <TextInput
                                source="alias"
                                label={String(t("electionEventScreen.field.alias"))}
                            />
                            <TextInput
                                source="description"
                                label={String(t("electionEventScreen.field.description"))}
                            />
                        </TabbedShowLayout.Tab>
                    </TabbedShowLayout> */}
                </AccordionDetails>
            </Accordion>

            <Accordion
                sx={{width: "100%"}}
                expanded={expanded === "election-event-data-dates"}
                onChange={() => setExpanded("election-event-data-dates")}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-event-data-dates" />}>
                    <Typography variant="h5">{t("electionEventScreen.edit.dates")}</Typography>
                </AccordionSummary>
                <AccordionDetails>
                    <Grid container spacing={4}>
                        <Grid size={{xs: 12, md: 6}}>
                            <DateTimeInput
                                source="start_date"
                                label={String(t("electionEventScreen.field.startDateTime"))}
                            />
                        </Grid>
                        <Grid size={{xs: 12, md: 6}}>
                            <DateTimeInput
                                source="end_date"
                                label={String(t("electionEventScreen.field.endDateTime"))}
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
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-event-data-language" />}>
                    <Typography variant="h5">{t("electionEventScreen.edit.language")}</Typography>
                </AccordionSummary>
                <AccordionDetails>
                    <Grid container spacing={4}>
                        <Grid size={{xs: 12, md: 6}}>
                            <BooleanInput
                                source="language.english"
                                label={"English"}
                                defaultValue={true}
                            />
                            <BooleanInput
                                source="language.spanish"
                                label={"Spanish"}
                                defaultValue={false}
                            />
                        </Grid>
                    </Grid>
                </AccordionDetails>
            </Accordion>

            <Accordion
                sx={{width: "100%"}}
                expanded={expanded === "election-event-data-allowed"}
                onChange={() => setExpanded("election-event-data-allowed")}
            >
                <AccordionSummary expandIcon={<ExpandMoreIcon id="election-event-data-allowed" />}>
                    <Typography variant="h5">{t("electionEventScreen.edit.allowed")}</Typography>
                </AccordionSummary>
                <AccordionDetails>
                    {" "}
                    <Grid container spacing={4}>
                        <Grid size={{xs: 12, md: 6}}>
                            <BooleanInput source="allowed.one" label={"One"} defaultValue={true} />
                            <BooleanInput source="allowed.two" label={"Two"} defaultValue={true} />
                        </Grid>
                    </Grid>
                </AccordionDetails>
            </Accordion>
        </SimpleForm>
    )
}
