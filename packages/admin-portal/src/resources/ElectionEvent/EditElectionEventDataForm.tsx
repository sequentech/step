// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    BooleanInput,
    DateTimeInput,
    RecordContext,
    SimpleForm,
    TextInput,
    Toolbar,
    SaveButton,
    DeleteButton,
} from "react-admin"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Tabs,
    Tab,
    Grid,
    Button,
    Drawer,
    Box,
} from "@mui/material"
import React, {useContext, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useAtom} from "jotai"
import importDrawerState from "@/atoms/import-drawer-state"
import {Dialog} from "@sequentech/ui-essentials"
import {ImportScreen} from "@/components/election-event/ImportScreen"
import {ListActions} from "@/components/ListActions"
import {ImportElectionEvent} from "@/components/election-event/ImportElectionEvent"
import {ListSupportMaterials} from "../SupportMaterials/ListSuportMaterial"

export const EditElectionEventDataForm: React.FC = () => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)

    const canEdit = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )

    const [value, setValue] = useState(0)
    const [valueMaterials, setValueMaterials] = useState(0)
    const [expanded, setExpanded] = useState("election-event-data-general")
    const [languageSettings] = useState<any>([{es: true}, {en: true}])
    const [votingSettings] = useState<any>({online: true, kiosk: true})
    const [openImport, setOpenImport] = useAtom(importDrawerState)
    const [openExport, setOpenExport] = React.useState(false)

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

        // set english first lang always
        const en = {en: temp.enabled_languages["en"]}
        delete temp.enabled_languages.en
        const rest = temp.enabled_languages
        temp.enabled_languages = {...en, ...rest}

        // voting channels
        const all_channels = {...incoming?.voting_channels}

        // delete incoming.voting_channels
        temp.voting_channels = {}
        for (const setting in votingSettings) {
            const enabled_item: any = {}
            enabled_item[setting] =
                setting in all_channels ? all_channels[setting] : votingSettings[setting]
            // temp.voting_channels = {...temp.voting_channels, ...enabled_item}
            temp.voting_channels = {...all_channels}
        }

        return temp
    }

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    const handleChangeMaterials = (event: React.SyntheticEvent, newValue: number) => {
        setValueMaterials(newValue)
    }

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
                    disabled={!canEdit}
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
                    disabled={!canEdit}
                    key={channel}
                    source={`voting_channels[${channel}]`}
                    label={t(`common.channel.${channel}`)}
                />
            )
        }
        return channelNodes
    }

    const renderTabs = (parsedValue: any, type: string = "general") => {
        let tabNodes = []
        for (const lang in parsedValue?.enabled_languages) {
            if (parsedValue?.enabled_languages[lang]) {
                tabNodes.push(<Tab key={lang} label={t(`common.language.${lang}`)} id={lang}></Tab>)
            }
        }

        // reset actived tab to first tab if only one
        if (tabNodes.length === 1) {
            if (type === "materials") {
                setValueMaterials(0)
            } else {
                setValue(0)
            }
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
                                disabled={!canEdit}
                                source={`presentation.i18n[${lang}].name`}
                                label={t("electionEventScreen.field.name")}
                            />
                            <TextInput
                                disabled={!canEdit}
                                source={`presentation.i18n[${lang}].alias`}
                                label={t("electionEventScreen.field.alias")}
                            />
                            <TextInput
                                disabled={!canEdit}
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

    const renderTabContentMaterials = (parsedValue: any) => {
        let tabNodes = []
        let index = 0
        for (const lang in parsedValue?.enabled_languages) {
            if (parsedValue?.enabled_languages[lang]) {
                tabNodes.push(
                    <CustomTabPanel key={lang} value={valueMaterials} index={index}>
                        <div style={{marginTop: "16px"}}>
                            <TextInput
                                disabled={!canEdit}
                                source={`presentation.i18n[${lang}].materialsTitle`}
                                label={t("electionEventScreen.field.materialTitle")}
                            />
                            <TextInput
                                disabled={!canEdit}
                                source={`presentation.i18n[${lang}].materialsSubtitle`}
                                label={t("electionEventScreen.field.materialSubTitle")}
                            />
                        </div>
                    </CustomTabPanel>
                )
                index++
            }
        }
        return tabNodes
    }

    const handleImport = () => {
        console.log("IMPORT")
        setOpenImport(true)
    }

    const handleExport = () => {
        console.log("EXPORT")
        setOpenExport(true)
    }

    const confirmExportAction = async () => {
        console.log("CONFIRM EXPORT")
    }

    return (
        <>
            <Box
                sx={{
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "flex-end",
                    alignItems: "center",
                }}
            >
                <ListActions
                    withImport
                    doImport={handleImport}
                    withExport
                    doExport={handleExport}
                    withColumns={false}
                    withFilter={false}
                />
            </Box>
            <RecordContext.Consumer>
                {(incoming) => {
                    const parsedValue = parseValues(incoming)
                    console.log("parsedValue :>> ", parsedValue)
                    return (
                        <SimpleForm
                            validate={formValidator}
                            record={parsedValue}
                            toolbar={
                                <Toolbar>{canEdit ? <SaveButton type="button" /> : null}</Toolbar>
                            }
                        >
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
                                                disabled={!canEdit}
                                                source="dates.start_date"
                                                label={t("electionScreen.field.startDateTime")}
                                                parse={(value) => new Date(value).toISOString()}
                                            />
                                        </Grid>
                                        <Grid item xs={12} md={6}>
                                            <DateTimeInput
                                                disabled={!canEdit}
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
                                    expandIcon={
                                        <ExpandMoreIcon id="election-event-data-language" />
                                    }
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

                            <Accordion
                                sx={{width: "100%"}}
                                expanded={expanded === "election-event-data-materials"}
                                onChange={() => setExpanded("election-event-data-materials")}
                            >
                                <AccordionSummary
                                    expandIcon={
                                        <ExpandMoreIcon id="election-event-data-materials" />
                                    }
                                >
                                    <ElectionHeaderStyles.Wrapper>
                                        <ElectionHeaderStyles.Title>
                                            {t("electionEventScreen.edit.materials")}
                                        </ElectionHeaderStyles.Title>
                                    </ElectionHeaderStyles.Wrapper>
                                </AccordionSummary>
                                <AccordionDetails>
                                    <BooleanInput
                                        disabled={!canEdit}
                                        source={`presentation.materials.activated`}
                                        label={t(`electionEventScreen.field.materialActivated`)}
                                    />
                                    <Tabs value={valueMaterials} onChange={handleChangeMaterials}>
                                        {renderTabs(parsedValue, "materials")}
                                    </Tabs>
                                    {renderTabContentMaterials(parsedValue)}
                                    <Box>
                                        <ListSupportMaterials electionEventId={parsedValue?.id} />
                                    </Box>
                                </AccordionDetails>
                            </Accordion>
                        </SimpleForm>
                    )
                }}
            </RecordContext.Consumer>

            <Drawer
                anchor="right"
                open={openImport}
                onClose={() => {
                    setOpenImport(false)
                }}
                PaperProps={{
                    sx: {width: "30%"},
                }}
            >
                <ImportElectionEvent doRefresh={() => {}} />
            </Drawer>

            <Dialog
                variant="info"
                open={openExport}
                ok={t("common.label.export")}
                cancel={t("common.label.cancel")}
                title={t("common.label.export")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    }
                    setOpenExport(false)
                }}
            >
                {t("common.export")}
            </Dialog>
        </>
    )
}
