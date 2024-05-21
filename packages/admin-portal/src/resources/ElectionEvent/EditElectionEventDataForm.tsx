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
    RaRecord,
    Identifier,
    useEditController,
    useRecordContext,
    RadioButtonGroupInput,
    useNotify,
    Button,
} from "react-admin"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Tabs,
    Tab,
    Grid,
    Drawer,
    Box,
} from "@mui/material"
import DownloadIcon from "@mui/icons-material/Download"
import React, {useContext, useEffect, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {Dialog, IElectionEventPresentation, ITenantSettings} from "@sequentech/ui-essentials"
import {ListActions} from "@/components/ListActions"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {ListSupportMaterials} from "../SupportMaterials/ListSuportMaterial"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {TVotingSetting} from "@/types/settings"
import {
    ExportElectionEventMutation,
    ImportCandidatesMutation,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../User/DownloadDocument"
import {EXPORT_ELECTION_EVENT} from "@/queries/ExportElectionEvent"
import {useMutation} from "@apollo/client"
import {CustomApolloContextProvider} from "@/providers/ApolloContextProvider"
import {IMPORT_CANDIDTATES} from "@/queries/ImportCandidates"

export type Sequent_Backend_Election_Event_Extended = RaRecord<Identifier> & {
    enabled_languages?: {[key: string]: boolean}
    defaultLanguage?: string
} & Sequent_Backend_Election_Event

interface ExportWrapperProps {
    electionEventId: string
    openExport: boolean
    setOpenExport: (val: boolean) => void
    exportDocumentId: string | undefined
    setExportDocumentId: (val: string | undefined) => void
}

const ExportWrapper: React.FC<ExportWrapperProps> = ({
    electionEventId,
    openExport,
    setOpenExport,
    exportDocumentId,
    setExportDocumentId,
}) => {
    const [exportElectionEvent] = useMutation<ExportElectionEventMutation>(EXPORT_ELECTION_EVENT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EVENT_READ,
            },
        },
    })
    const notify = useNotify()
    const {t} = useTranslation()

    const confirmExportAction = async () => {
        console.log("CONFIRM EXPORT")

        const {data: exportElectionEventData, errors} = await exportElectionEvent({
            variables: {
                electionEventId,
            },
        })
        let documentId = exportElectionEventData?.export_election_event?.document_id
        if (errors || !documentId) {
            setOpenExport(false)
            notify(t(`electionEventScreen.exportError`), {type: "error"})
            console.log(`Error exporting users: ${errors}`)
            return
        }
        setExportDocumentId(documentId)
    }

    return (
        <Dialog
            variant="info"
            open={openExport}
            ok={t("common.label.export")}
            cancel={t("common.label.cancel")}
            title={t("common.label.export")}
            handleClose={(result: boolean) => {
                if (result) {
                    confirmExportAction()
                } else {
                    setOpenExport(false)
                }
            }}
        >
            {t("common.export")}
            {exportDocumentId ? (
                <>
                    <FormStyles.ShowProgress />
                    <DownloadDocument
                        documentId={exportDocumentId}
                        electionEventId={electionEventId ?? ""}
                        fileName={`election-event-${electionEventId}-export.csv`}
                        onDownload={() => {
                            console.log("onDownload called")
                            setExportDocumentId(undefined)
                            setOpenExport(false)
                        }}
                    />
                </>
            ) : null}
        </Dialog>
    )
}

export const EditElectionEventDataForm: React.FC = () => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const canEdit = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )

    const [value, setValue] = useState(0)
    const [valueMaterials, setValueMaterials] = useState(0)
    const [expanded, setExpanded] = useState("election-event-data-general")
    const [languageSettings, setLanguageSettings] = useState<Array<string>>(["en"])
    const [openExport, setOpenExport] = React.useState(false)
    const [exportDocumentId, setExportDocumentId] = React.useState<string | undefined>()
    const [openDrawer, setOpenDrawer] = useState<boolean>(false)
    const [openImportCandidates, setOpenImportCandidates] = React.useState(false)
    const [importCandidates] = useMutation<ImportCandidatesMutation>(IMPORT_CANDIDTATES)
    const notify = useNotify()
    const {record: tenant} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const [votingSettings] = useState<TVotingSetting>({
        online: tenant?.voting_channels?.online || true,
        kiosk: tenant?.voting_channels?.kiosk || false,
    })

    useEffect(() => {
        let tenantAvailableLangs = (tenant?.settings as ITenantSettings | undefined)?.language_conf
            ?.enabled_language_codes ?? ["en"]
        let eventAvailableLangs =
            (record?.presentation as IElectionEventPresentation | undefined)?.language_conf
                ?.enabled_language_codes ?? []
        let newEventLangs = eventAvailableLangs.filter(
            (eventLang) => !tenantAvailableLangs.includes(eventLang)
        )
        let completeList = tenantAvailableLangs.concat(newEventLangs)

        setLanguageSettings(completeList)
    }, [
        tenant?.settings?.language_conf?.enabled_language_codes,
        record?.presentation?.language_conf?.enabled_language_codes,
    ])

    const parseValues = (
        incoming: Sequent_Backend_Election_Event_Extended,
        languageSettings: Array<string>
    ): Sequent_Backend_Election_Event_Extended => {
        const temp = {...incoming}

        // languages
        temp.enabled_languages = {}

        const incomingLangConf = (incoming?.presentation as IElectionEventPresentation | undefined)
            ?.language_conf

        if (
            incomingLangConf?.enabled_language_codes &&
            incomingLangConf?.enabled_language_codes.length > 0
        ) {
            // if presentation has lang then set from event
            for (const setting of languageSettings) {
                const enabled_item: {[key: string]: boolean} = {}

                const isInEnabled =
                    incomingLangConf?.enabled_language_codes?.find(
                        (item: string) => setting === item
                    ) ?? false

                enabled_item[setting] = !!isInEnabled

                temp.enabled_languages = {...temp.enabled_languages, ...enabled_item}
            }
        } else {
            // if presentation has no lang then use always the default settings
            temp.enabled_languages = {...temp.enabled_languages}
            for (const item of languageSettings) {
                temp.enabled_languages[item] = false
            }
        }

        // set english first lang always
        if (temp.enabled_languages) {
            const en = {en: temp.enabled_languages["en"]}
            delete temp.enabled_languages.en
            const rest = temp.enabled_languages
            temp.enabled_languages = {...en, ...rest}
        }
        // voting channels
        const all_channels = {...incoming?.voting_channels}

        // delete incoming.voting_channels
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

    const renderDefaultLangs = (_parsedValue: Sequent_Backend_Election_Event_Extended) => {
        let langNodes = languageSettings.map((lang) => ({
            id: lang,
            name: t(`electionScreen.edit.default`),
        }))

        return (
            <RadioButtonGroupInput
                label={false}
                source="presentation.language_conf.default_language_code"
                choices={langNodes}
                row={true}
            />
        )
    }

    const renderLangs = (parsedValue: Sequent_Backend_Election_Event_Extended) => {
        return (
            <Box>
                {languageSettings.map((lang) => (
                    <BooleanInput
                        key={lang}
                        disabled={!canEdit}
                        source={`enabled_languages.${lang}`}
                        label={t(`common.language.${lang}`)}
                    />
                ))}
            </Box>
        )
    }

    const renderVotingChannels = (parsedValue: Sequent_Backend_Election_Event_Extended) => {
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

    const renderTabs = (
        parsedValue: Sequent_Backend_Election_Event_Extended,
        type: string = "general"
    ) => {
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

    const renderTabContent = (parsedValue: Sequent_Backend_Election_Event_Extended) => {
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

    const renderTabContentMaterials = (parsedValue: Sequent_Backend_Election_Event_Extended) => {
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
        setOpenDrawer(true)
    }

    const handleExport = () => {
        console.log("EXPORT")
        setOpenExport(true)
    }
    const handleImportCandidates = async (documentId: string, sha256: string) => {
        let {data, errors} = await importCandidates({
            variables: {
                documentId,
                electionEventId: record.id,
            },
        })

        if (errors) {
            console.log(errors)
            notify("Error importing candidates", {type: "error"})
            return
        }

        notify("Candidates successfully imported", {type: "success"})
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
                    extraActions={[
                        <Button
                            onClick={() => setOpenImportCandidates(true)}
                            label="Import Candidates"
                            key="1"
                        >
                            <DownloadIcon />
                        </Button>,
                    ]}
                />
            </Box>
            <RecordContext.Consumer>
                {(incoming) => {
                    const parsedValue = parseValues(
                        incoming as Sequent_Backend_Election_Event_Extended,
                        languageSettings
                    )
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
                                expanded={expanded === "election-event-data-ballot-style"}
                                onChange={() => setExpanded("election-event-data-ballot-style")}
                            >
                                <AccordionSummary
                                    expandIcon={
                                        <ExpandMoreIcon id="election-event-data-ballot-style" />
                                    }
                                >
                                    <ElectionHeaderStyles.Wrapper>
                                        <ElectionHeaderStyles.Title>
                                            {t("electionEventScreen.edit.ballotDesign")}
                                        </ElectionHeaderStyles.Title>
                                    </ElectionHeaderStyles.Wrapper>
                                </AccordionSummary>
                                <AccordionDetails>
                                    <BooleanInput
                                        disabled={!canEdit}
                                        source={"presentation.hide_audit"}
                                        label={t(`electionEventScreen.field.hideAudit`)}
                                    />
                                    <BooleanInput
                                        disabled={!canEdit}
                                        source={"presentation.skip_election_list"}
                                        label={t(`electionEventScreen.field.skipElectionList`)}
                                    />
                                    <BooleanInput
                                        disabled={!canEdit}
                                        source={"presentation.show_user_profile"}
                                        label={t(`electionEventScreen.field.showUserProfile`)}
                                    />
                                    <TextInput
                                        resettable={true}
                                        source={"presentation.logo_url"}
                                        label={t("electionEventScreen.field.logoUrl")}
                                    />
                                    <TextInput
                                        resettable={true}
                                        source={"presentation.redirect_finish_url"}
                                        label={t("electionEventScreen.field.redirectFinishUrl")}
                                    />
                                    <TextInput
                                        resettable={true}
                                        multiline={true}
                                        source={"presentation.css"}
                                        label={t("electionEventScreen.field.css")}
                                    />
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

            <ImportDataDrawer
                open={openDrawer}
                closeDrawer={() => setOpenDrawer(false)}
                title="electionEventScreen.import.eetitle"
                subtitle="electionEventScreen.import.eesubtitle"
                paragraph="electionEventScreen.import.electionEventParagraph"
                doImport={async () => {}}
                errors={null}
            />

            <ImportDataDrawer
                open={openImportCandidates}
                closeDrawer={() => setOpenImportCandidates(false)}
                title="electionEventScreen.import.importCandidatesTitle"
                subtitle="electionEventScreen.import.importCandidatesSubtitle"
                paragraph="electionEventScreen.import.importCandidatesParagraph"
                doImport={handleImportCandidates}
                errors={null}
            />

            <ExportWrapper
                electionEventId={record.id}
                openExport={openExport}
                setOpenExport={setOpenExport}
                exportDocumentId={exportDocumentId}
                setExportDocumentId={setExportDocumentId}
            />
        </>
    )
}
