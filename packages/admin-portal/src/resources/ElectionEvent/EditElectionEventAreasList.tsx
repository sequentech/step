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
                            <TextInput source="name" label={t("electionEventScreen.field.name")} />
                            <TextInput
                                source="alias"
                                label={t("electionEventScreen.field.alias")}
                            />
                            <TextInput
                                source="description"
                                label={t("electionEventScreen.field.description")}
                            />
                        </div>
                    </CustomTabPanel>
                    <CustomTabPanel value={value} index={1}>
                        <div style={{marginTop: "16px"}}>
                            <TextInput source="name" label={t("electionEventScreen.field.name")} />
                            <TextInput
                                source="alias"
                                label={t("electionEventScreen.field.alias")}
                            />
                            <TextInput
                                source="description"
                                label={t("electionEventScreen.field.description")}
                            />
                        </div>
                    </CustomTabPanel>

                    {/* <TabbedShowLayout>
                        <TabbedShowLayout.Tab label="English" >
                            <TextInput source="name" label={t("electionEventScreen.field.name")} />
                            <TextInput
                                source="alias"
                                label={t("electionEventScreen.field.alias")}
                            />
                            <TextInput
                                source="description"
                                label={t("electionEventScreen.field.description")}
                            />
                        </TabbedShowLayout.Tab>
                        <TabbedShowLayout.Tab label="Spanish" >
                            <TextInput source="name" label={t("electionEventScreen.field.name")} />
                            <TextInput
                                source="alias"
                                label={t("electionEventScreen.field.alias")}
                            />
                            <TextInput
                                source="description"
                                label={t("electionEventScreen.field.description")}
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
                        <Grid item xs={12} md={6}>
                            <DateTimeInput
                                source="start_date"
                                label={t("electionEventScreen.field.startDateTime")}
                            />
                        </Grid>
                        <Grid item xs={12} md={6}>
                            <DateTimeInput
                                source="end_date"
                                label={t("electionEventScreen.field.endDateTime")}
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
                        <Grid item xs={12} md={6}>
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
                        <Grid item xs={12} md={6}>
                            <BooleanInput source="allowed.one" label={"One"} defaultValue={true} />
                            <BooleanInput source="allowed.two" label={"Two"} defaultValue={true} />
                        </Grid>
                    </Grid>
                </AccordionDetails>
            </Accordion>

            {/* <Typography variant="h4">Election Event</Typography>
            <Typography variant="body2">Election event configuration</Typography>
            <Button onClick={handleActionsButtonClick}>
                Actions {showProgress ? <CircularProgress /> : null}
            </Button>
            <Menu
                id="election-event-actions-menu"
                anchorEl={anchorEl}
                open={showMenu}
                onClose={() => setShowMenu(false)}
            >
                <MenuItem
                    onClick={createBulletinBoardAction}
                    disabled={!!record.bulletin_board_reference}
                >
                    Create Bulletin Board
                </MenuItem>
                <MenuItem
                    onClick={openKeysDialog}
                    disabled={!record.bulletin_board_reference || configCreatedStatus}
                >
                    Create Keys
                </MenuItem>
                <MenuItem
                    onClick={setPublicKeysAction}
                    disabled={!!record.public_key || !configCreatedStatus}
                >
                    Set Public Keys
                </MenuItem>
                <MenuItem onClick={createBallotStylesAction}>Create Ballot Styles</MenuItem>
                <MenuItem onClick={openStartTallyDialog}>Start Tally</MenuItem>
            </Menu>
            <KeysGenerationDialog
                show={showCreateKeysDialog}
                handleClose={() => setShowCreateKeysDialog(false)}
                electionEvent={record}
            />
            <StartTallyDialog
                show={showStartTallyDialog}
                handleClose={() => setShowStartTallyDialog(false)}
                electionEvent={record}
            />
            <Typography variant="h5">ID</Typography>
            <TextField source="id" />
            <TextInput source="name" />
            <TextInput source="description" />
            <SelectInput source="encryption_protocol" choices={[{id: "RSA256", name: "RSA256"}]} />
            <BooleanInput source="is_archived" />
            <BooleanInput source="is_audit" />
            <TextInput source="public_key" />
            <Typography variant="h5">Elections</Typography>
            <ReferenceManyField
                label="Elections"
                reference="sequent_backend_election"
                target="election_event_id"
            >
                <HorizontalBox>
                    <ChipList
                        source="sequent_backend_election"
                        filterFields={["election_event_id"]}
                    />
                </HorizontalBox>
            </ReferenceManyField>
            <Link
                to={{
                    pathname: "/sequent_backend_election/create",
                }}
                state={{
                    record: {
                        election_event_id: record.id,
                        tenant_id: record.tenant_id,
                    },
                }}
            >
                <Button>
                    <IconButton icon={faPlusCircle} fontSize="24px" />
                    Add election
                </Button>
            </Link>
            <Typography variant="h5">Areas</Typography>
            <ReferenceManyField
                label="Areas"
                reference="sequent_backend_area"
                target="election_event_id"
            >
                <ChipList source="sequent_backend_area" filterFields={["election_event_id"]} />
            </ReferenceManyField>
            <Link
                to={{
                    pathname: "/sequent_backend_area/create",
                }}
                state={{
                    record: {
                        election_event_id: record.id,
                        tenant_id: record.tenant_id,
                    },
                }}
            >
                <Button>
                    <IconButton icon={faPlusCircle} fontSize="24px" />
                    Add area
                </Button>
            </Link>
            <JsonInput
                source="bulletin_board_reference"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="labels"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="presentation"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="voting_channels"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="voting_channels"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="dates"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <TextInput source="user_boards" />
            <TextInput source="audit_election_event_id" />
            <Typography variant="h5">Documents</Typography>
            <ReferenceManyField
                label="Documents"
                reference="sequent_backend_document"
                target="election_event_id"
            >
                <HorizontalBox>
                    <ChipList
                        source="sequent_backend_document"
                        filterFields={["election_event_id"]}
                    />
                </HorizontalBox>
            </ReferenceManyField> */}
        </SimpleForm>
    )
}
