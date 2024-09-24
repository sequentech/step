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
    SelectInput,
    NumberInput,
    required,
    FormDataConsumer,
    useGetList,
    useInput,
} from "react-admin"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Tabs,
    Tab,
    Grid,
    Box,
    Typography,
} from "@mui/material"
import styled from "@emotion/styled"
import DownloadIcon from "@mui/icons-material/Download"
import React, {useCallback, useContext, useEffect, useMemo, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ICommunicationType, ISendCommunicationBody} from "@/types/communications"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {
    ElectionsOrder,
    IElectionDates,
    IElectionEventPresentation,
    IElectionPresentation,
    ITenantSettings,
    EVotingPortalCountdownPolicy,
} from "@sequentech/ui-core"
import {Dialog} from "@sequentech/ui-essentials"
import {ListActions} from "@/components/ListActions"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {ListSupportMaterials} from "../SupportMaterials/ListSuportMaterial"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {TVotingSetting} from "@/types/settings"
import {
    ExportElectionEventMutation,
    ImportCandidatesMutation,
    Sequent_Backend_Election,
    ManageElectionDatesMutation,
    Sequent_Backend_Election_Event,
    SetCustomUrlsMutation,
    Sequent_Backend_Communication_Template,
} from "@/gql/graphql"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../User/DownloadDocument"
import {EXPORT_ELECTION_EVENT} from "@/queries/ExportElectionEvent"
import {FetchResult, useMutation} from "@apollo/client"
import {IMPORT_CANDIDTATES} from "@/queries/ImportCandidates"
import CustomOrderInput from "@/components/custom-order/CustomOrderInput"
import {useWatch} from "react-hook-form"
import {convertToNumber} from "@/lib/helpers"
import {MANAGE_ELECTION_DATES} from "@/queries/ManageElectionDates"
import {ETasksExecution} from "@/types/tasksExecution"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
// import {SET_CUSTOM_URL} from "@/queries/SetCustomUrl"
import {SET_CUSTOM_URLS} from "@/queries/SetCustomUrls"
import {getAuthUrl} from "@/services/UrlGeneration"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {CustomUrlsStyle} from "@/components/styles/CustomUrlsStyle"
import {StatusChip} from "@/components/StatusChip"

export type Sequent_Backend_Election_Event_Extended = RaRecord<Identifier> & {
    enabled_languages?: {[key: string]: boolean}
    defaultLanguage?: string
    electionsOrder?: Array<Sequent_Backend_Election>
} & Sequent_Backend_Election_Event

const ElectionRows = styled.div`
    display: flex;
    flex-direction: column;
    width: 100%;
    cursor: pointer;
    margin-bottom: 0.1rem;
    padding: 1rem;
`

interface ManagedNumberInputProps {
    source: string
    label: string
    defaultValue: number
    sourceToWatch: string
}

const ManagedNumberInput = ({
    source,
    label,
    defaultValue,
    sourceToWatch,
}: ManagedNumberInputProps) => {
    const secondsToShowCountdownSource = `presentation.voting_portal_countdown_policy.countdown_anticipation_secs`
    const secondsToShowAlretSource = `presentation.voting_portal_countdown_policy.countdown_alert_anticipation_secs`
    const selectedPolicy = useWatch({name: sourceToWatch})
    const isDisabled =
        (source === secondsToShowCountdownSource &&
            selectedPolicy === EVotingPortalCountdownPolicy.NO_COUNTDOWN) ||
        (source === secondsToShowAlretSource &&
            selectedPolicy !== EVotingPortalCountdownPolicy.COUNTDOWN_WITH_ALERT)

    return (
        <NumberInput
            source={source}
            disabled={isDisabled}
            label={label}
            defaultValue={defaultValue}
            style={{flex: 1}}
        />
    )
}

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
    const {t} = useTranslation()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const [exportElectionEvent] = useMutation<ExportElectionEventMutation>(EXPORT_ELECTION_EVENT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EVENT_READ,
            },
        },
    })

    const confirmExportAction = async () => {
        console.log("CONFIRM EXPORT")
        setOpenExport(false)
        const currWidget = addWidget(ETasksExecution.EXPORT_ELECTION_EVENT)
        const {data: exportElectionEventData, errors} = await exportElectionEvent({
            variables: {electionEventId},
        })

        const documentId = exportElectionEventData?.export_election_event?.document_id
        if (errors || !documentId) {
            updateWidgetFail(currWidget.identifier)
            console.log(`Error exporting users: ${errors}`)
            return
        }

        const task_id = exportElectionEventData?.export_election_event?.task_execution.id
        setWidgetTaskId(currWidget.identifier, task_id)
        setExportDocumentId(documentId)
    }

    const onDownloadDocument = () => {
        console.log("onDownload called")
        setExportDocumentId(undefined)
    }

    return (
        <>
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
            </Dialog>
            {exportDocumentId ? (
                <>
                    <FormStyles.ShowProgress />
                    <DownloadDocument
                        documentId={exportDocumentId}
                        electionEventId={electionEventId ?? ""}
                        fileName={`election-event-${electionEventId}-export.json`}
                        onDownload={onDownloadDocument}
                    />
                </>
            ) : null}
        </>
    )
}

export const EditElectionEventDataForm: React.FC = () => {
    const {t} = useTranslation()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const notify = useNotify()

    const canEdit = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )

    const [value, setValue] = useState(0)
    const [valueMaterials, setValueMaterials] = useState(0)
    const [expanded, setExpanded] = useState("election-event-data-general")
    const [languageSettings, setLanguageSettings] = useState<Array<string>>(["en"])
    const [openExport, setOpenExport] = useState(false)
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const [openDrawer, setOpenDrawer] = useState<boolean>(false)
    const [openImportCandidates, setOpenImportCandidates] = useState(false)
    const [importCandidates] = useMutation<ImportCandidatesMutation>(IMPORT_CANDIDTATES)
    const defaultSecondsForCountdown = convertToNumber(process.env.SECONDS_TO_SHOW_COUNTDOWN) ?? 60
    const defaultSecondsForAlret = convertToNumber(process.env.SECONDS_TO_SHOW_AlERT) ?? 180
    const [manageElectionDates] = useMutation<ManageElectionDatesMutation>(MANAGE_ELECTION_DATES)
    const [customUrlsValues, setCustomUrlsValues] = useState({login: "", enrollment: "", saml: ""})
    const [customLoginRes, setCustomLoginRes] = useState<FetchResult<SetCustomUrlsMutation>>()
    const [customEnrollmentRes, setCustomEnrollmentRes] =
        useState<FetchResult<SetCustomUrlsMutation>>()
    const [customSamlRes, setCustomSamlRes] = useState<FetchResult<SetCustomUrlsMutation>>()
    const [isCustomUrlLoading, setIsCustomUrlLoading] = useState(false)
    const [isCustomizeUrl, setIsCustomizeUrl] = useState(false)

    const [manageCustomUrls, response] = useMutation<SetCustomUrlsMutation>(SET_CUSTOM_URLS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EVENT_WRITE,
            },
        },
    })

    const [startDate, setStartDate] = useState<string | undefined>(undefined)
    const [endDate, setEndDate] = useState<string | undefined>(undefined)
    const {record: tenant} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })
    const {data: elections} = useGetList<Sequent_Backend_Election>("sequent_backend_election", {
        filter: {
            tenant_id: record.tenant_id,
            election_event_id: record.id,
        },
    })

    const {data: verifyVoterTemplates} = useGetList<Sequent_Backend_Communication_Template>(
        "sequent_backend_communication_template",
        {
            filter: {
                tenant_id: tenantId,
                communication_type: ICommunicationType.MANUALLY_VERIFY_VOTER,
            },
        }
    )

    const manuallyVerifyVoterTemplates = (): Array<EnumChoice<string>> => {
        if (!verifyVoterTemplates) {
            return []
        }
        const template_names = (
            verifyVoterTemplates as Sequent_Backend_Communication_Template[]
        ).map((entry) => {
            console.log("id: ", entry.id)
            console.log("name: ", entry.template?.name)
            return {
                id: entry.id,
                name: entry.template?.name,
            }
        })
        console.log("template_names: ", template_names)
        return template_names
    }

    const [votingSettings] = useState<TVotingSetting>({
        online: tenant?.voting_channels?.online || true,
        kiosk: tenant?.voting_channels?.kiosk || false,
    })

    useEffect(() => {
        let dates = record.dates as IElectionDates | undefined
        if (dates?.start_date && !startDate) {
            setStartDate(dates.start_date)
        }
        if (dates?.end_date && !endDate) {
            setEndDate(dates.end_date)
        }
    }, [
        record.dates,
        record.dates?.start_date,
        record.dates?.end_date,
        startDate,
        setStartDate,
        endDate,
        setEndDate,
    ])

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
        tenant?.settings,
        record?.presentation,
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

        if (!incoming.presentation) {
            temp.presentation = {}
        }
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
        if (!temp.presentation) {
            temp.presentation = {}
        }

        temp.presentation.elections_order =
            temp?.presentation.elections_order || ElectionsOrder.ALPHABETICAL

        if (
            !(temp.presentation as IElectionEventPresentation | undefined)
                ?.voting_portal_countdown_policy
        ) {
            temp.presentation.voting_portal_countdown_policy = {
                policy: EVotingPortalCountdownPolicy.NO_COUNTDOWN,
            }
        }
        if (!temp.presentation.custom_urls) {
            temp.presentation.custom_urls = {}
        }

        if (!temp.presentation.custom_tpl_usr_verfication) {
            temp.presentation.custom_tpl_usr_verfication = ""
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
            errors.dates.end_date = t("electionScreen.error.endDate")
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

    const handleExport = () => {
        console.log("EXPORT")
        setOpenExport(true)
    }

    interface EnumChoice<T> {
        id: T
        name: string
    }

    const orderAnswerChoices = (): Array<EnumChoice<ElectionsOrder>> => {
        return Object.values(ElectionsOrder).map((value) => ({
            id: value,
            name: t(`contestScreen.options.${value.toLowerCase()}`),
        }))
    }

    const handleImportCandidates = async (documentId: string, sha256: string) => {
        setOpenImportCandidates(false)
        const currWidget = addWidget(ETasksExecution.IMPORT_CANDIDATES)
        try {
            let {data, errors} = await importCandidates({
                variables: {
                    documentId,
                    electionEventId: record.id,
                },
            })

            if (errors) {
                console.log(errors)
                notify("Error importing candidates", {type: "error"})
                updateWidgetFail(currWidget.identifier)
                return
            }
            setWidgetTaskId(currWidget.identifier, data?.import_candidates?.task_execution.id)
        } catch (err) {
            notify("Error importing candidates", {type: "error"})
            updateWidgetFail(currWidget.identifier)
        }
    }

    const handleUpdateCustomUrls = async (
        presentation: IElectionEventPresentation,
        recordId: string
    ) => {
        try {
            const urlEntries = [
                {
                    key: "login",
                    origin: `https://${customUrlsValues.login}.${globalSettings.CUSTOM_URLS_DOMAIN_NAME}`,
                    redirect_to: getAuthUrl(
                        globalSettings.VOTING_PORTAL_URL,
                        tenantId ?? "",
                        recordId,
                        "login"
                    ),
                    dns_prefix: customUrlsValues.login,
                },
                {
                    key: "enrollment",
                    origin: `https://${customUrlsValues.enrollment}.${globalSettings.CUSTOM_URLS_DOMAIN_NAME}`,
                    redirect_to: getAuthUrl(
                        globalSettings.VOTING_PORTAL_URL,
                        tenantId ?? "",
                        recordId,
                        "enroll"
                    ),
                    dns_prefix: customUrlsValues.enrollment,
                },
                {
                    key: "saml",
                    origin: `https://${customUrlsValues.saml}.${globalSettings.CUSTOM_URLS_DOMAIN_NAME}`,
                    redirect_to: `${globalSettings.KEYCLOAK_URL}realms/tenant-${tenantId}-event-${recordId}/broker/simplesamlphp/endpoint`,
                    dns_prefix: customUrlsValues.saml,
                },
            ]
            setIsCustomUrlLoading(true)
            setIsCustomizeUrl(true)
            const [loginResponse, enrollmentResponse, samlResponse] = await Promise.all(
                urlEntries.map((item) =>
                    manageCustomUrls({
                        variables: {
                            origin: item.origin,
                            redirect_to: item.redirect_to ?? "",
                            dns_prefix: item.dns_prefix,
                            election_id: recordId,
                            key: item.key,
                        },
                    })
                )
            )
            setCustomLoginRes(loginResponse)
            setCustomEnrollmentRes(enrollmentResponse)
            setCustomSamlRes(samlResponse)
        } catch (err: any) {
            console.error(err)
        } finally {
            setIsCustomUrlLoading(false)
        }
    }

    const sortedElections = (elections ?? []).sort((a, b) => {
        let presentationA = a.presentation as IElectionPresentation | undefined
        let presentationB = b.presentation as IElectionPresentation | undefined
        let sortOrderA = presentationA?.sort_order ?? -1
        let sortOrderB = presentationB?.sort_order ?? -1
        return sortOrderA - sortOrderB
    })
    const votingPortalCountDownPolicies = () => {
        return Object.values(EVotingPortalCountdownPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.countDownPolicyOptions.${value}`),
        }))
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
                    withImport={false}
                    withExport
                    doExport={handleExport}
                    isExportDisabled={openExport}
                    withColumns={false}
                    withFilter={false}
                    extraActions={[
                        <Button
                            className="import-candidates"
                            onClick={() => setOpenImportCandidates(true)}
                            label={t("electionEventScreen.edit.importCandidates")}
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
                    const onSave = async () => {
                        await manageElectionDates({
                            variables: {
                                electionEventId: record.id,
                                start_date: startDate,
                                end_date: endDate,
                            },
                            onError() {
                                notify("Error updating custom url", {type: "error"})
                            },
                        })
                        await handleUpdateCustomUrls(
                            parsedValue.presentation as IElectionEventPresentation,
                            record.id
                        )
                    }
                    return (
                        <SimpleForm
                            defaultValues={{electionsOrder: sortedElections}}
                            validate={formValidator}
                            record={parsedValue}
                            toolbar={
                                <Toolbar>
                                    {canEdit ? (
                                        <SaveButton
                                            onClick={() => {
                                                onSave()
                                            }}
                                            type="button"
                                        />
                                    ) : null}
                                </Toolbar>
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
                                                parse={(value) =>
                                                    value && new Date(value).toISOString()
                                                }
                                                onChange={(value) => {
                                                    setStartDate(
                                                        value && value.target.value !== ""
                                                            ? new Date(
                                                                  value.target.value
                                                              ).toISOString()
                                                            : undefined
                                                    )
                                                }}
                                            />
                                        </Grid>
                                        <Grid item xs={12} md={6}>
                                            <DateTimeInput
                                                disabled={!canEdit}
                                                source="dates.end_date"
                                                label={t("electionScreen.field.endDateTime")}
                                                parse={(value) =>
                                                    value && new Date(value).toISOString()
                                                }
                                                onChange={(value) => {
                                                    setEndDate(
                                                        value.target.value !== ""
                                                            ? new Date(
                                                                  value.target.value
                                                              ).toISOString()
                                                            : undefined
                                                    )
                                                }}
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
                                        source={"presentation.skip_election_list"}
                                        label={t(`electionEventScreen.field.skipElectionList`)}
                                    />
                                    <BooleanInput
                                        disabled={!canEdit}
                                        source={"presentation.show_user_profile"}
                                        label={t(`electionEventScreen.field.showUserProfile`)}
                                    />
                                    <SelectInput
                                        source="presentation.elections_order"
                                        choices={orderAnswerChoices()}
                                        validate={required()}
                                    />
                                    <FormDataConsumer>
                                        {({formData, ...rest}) => {
                                            return (
                                                formData?.presentation as
                                                    | IElectionEventPresentation
                                                    | undefined
                                            )?.elections_order === ElectionsOrder.CUSTOM ? (
                                                <ElectionRows>
                                                    <Typography
                                                        variant="body1"
                                                        component="span"
                                                        sx={{
                                                            padding: "0.5rem 1rem",
                                                            fontWeight: "bold",
                                                            margin: 0,
                                                            display: {xs: "none", sm: "block"},
                                                        }}
                                                    >
                                                        {t("electionEventScreen.edit.reorder")}
                                                    </Typography>
                                                    <CustomOrderInput source="electionsOrder" />
                                                    <Box
                                                        sx={{width: "100%", height: "180px"}}
                                                    ></Box>
                                                </ElectionRows>
                                            ) : null
                                        }}
                                    </FormDataConsumer>
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
                                expanded={expanded === "election-event-data-user-templates"}
                                onChange={() => setExpanded("election-event-data-user-templates")}
                            >
                                <AccordionSummary
                                    expandIcon={
                                        <ExpandMoreIcon id="election-event-data-user-templates" />
                                    }
                                >
                                    <ElectionHeaderStyles.Wrapper>
                                        <ElectionHeaderStyles.Title>
                                            {t("electionEventScreen.edit.templates")}
                                        </ElectionHeaderStyles.Title>
                                    </ElectionHeaderStyles.Wrapper>
                                </AccordionSummary>
                                <AccordionDetails>
                                    <Typography
                                        variant="body1"
                                        component="span"
                                        sx={{
                                            fontWeight: "bold",
                                            margin: 0,
                                            display: {xs: "none", sm: "block"},
                                        }}
                                    >
                                        {t("electionEventScreen.field.userVerification")}
                                    </Typography>
                                    <SelectInput
                                        source={`presentation.custom_tpl_usr_verfication`}
                                        choices={manuallyVerifyVoterTemplates()}
                                        label={t("communicationTemplate.form.name")}
                                        translateChoice={false}
                                        emptyText={t("communicationTemplate.default")}
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
                                expanded={expanded === "election-event-data-custom-urls"}
                                onChange={() => setExpanded("election-event-data-custom-urls")}
                            >
                                <AccordionSummary
                                    expandIcon={
                                        <ExpandMoreIcon id="election-event-data-custom-urls" />
                                    }
                                >
                                    <ElectionHeaderStyles.Wrapper>
                                        <ElectionHeaderStyles.Title>
                                            {t("electionEventScreen.edit.customUrls")}
                                        </ElectionHeaderStyles.Title>
                                    </ElectionHeaderStyles.Wrapper>
                                </AccordionSummary>
                                <AccordionDetails>
                                    <CustomUrlsStyle.InputWrapper>
                                        <CustomUrlsStyle.InputLabel>
                                            Login:
                                        </CustomUrlsStyle.InputLabel>
                                        <CustomUrlsStyle.InputLabelWrapper>
                                            <p>https://</p>
                                            <TextInput
                                                variant="standard"
                                                helperText={false}
                                                sx={{width: "300px"}}
                                                source={`presentation.custom_urls.login`}
                                                label={""}
                                                onChange={(e) =>
                                                    setCustomUrlsValues({
                                                        ...customUrlsValues,
                                                        login: e.target.value,
                                                    })
                                                }
                                            />
                                            <p>{`.${globalSettings.CUSTOM_URLS_DOMAIN_NAME}`}</p>
                                            {isCustomUrlLoading ? (
                                                <WizardStyles.DownloadProgress size={18} />
                                            ) : (
                                                isCustomizeUrl &&
                                                (customLoginRes?.data?.set_custom_urls?.success ? (
                                                    <StatusChip status="SUCCESS" />
                                                ) : (
                                                    <StatusChip status="ERROR" />
                                                ))
                                            )}
                                        </CustomUrlsStyle.InputLabelWrapper>
                                        {customLoginRes &&
                                            !customLoginRes?.data?.set_custom_urls?.success && (
                                                <CustomUrlsStyle.ErrorText>
                                                    {customLoginRes?.data?.set_custom_urls?.message}
                                                </CustomUrlsStyle.ErrorText>
                                            )}
                                    </CustomUrlsStyle.InputWrapper>
                                    <CustomUrlsStyle.InputWrapper>
                                        <CustomUrlsStyle.InputLabel>
                                            Enrollment:
                                        </CustomUrlsStyle.InputLabel>
                                        <CustomUrlsStyle.InputLabelWrapper>
                                            <p>https://</p>
                                            <TextInput
                                                variant="standard"
                                                helperText={false}
                                                sx={{width: "300px"}}
                                                source={`presentation.custom_urls.enrollment`}
                                                label={""}
                                                onChange={(e) =>
                                                    setCustomUrlsValues({
                                                        ...customUrlsValues,
                                                        enrollment: e.target.value,
                                                    })
                                                }
                                            />
                                            <p>{`.${globalSettings.CUSTOM_URLS_DOMAIN_NAME}`}</p>
                                            {isCustomUrlLoading ? (
                                                <WizardStyles.DownloadProgress size={18} />
                                            ) : (
                                                isCustomizeUrl &&
                                                (customEnrollmentRes?.data?.set_custom_urls
                                                    ?.success ? (
                                                    <StatusChip status="SUCCESS" />
                                                ) : (
                                                    <StatusChip status="ERROR" />
                                                ))
                                            )}
                                        </CustomUrlsStyle.InputLabelWrapper>
                                        {customEnrollmentRes &&
                                            !customEnrollmentRes?.data?.set_custom_urls
                                                ?.success && (
                                                <CustomUrlsStyle.ErrorText>
                                                    {
                                                        customEnrollmentRes?.data?.set_custom_urls
                                                            ?.message
                                                    }
                                                </CustomUrlsStyle.ErrorText>
                                            )}
                                    </CustomUrlsStyle.InputWrapper>
                                    <CustomUrlsStyle.InputWrapper>
                                        <CustomUrlsStyle.InputLabel>
                                            SAML:
                                        </CustomUrlsStyle.InputLabel>
                                        <CustomUrlsStyle.InputLabelWrapper>
                                            <p>https://</p>
                                            <TextInput
                                                variant="standard"
                                                helperText={false}
                                                sx={{width: "300px"}}
                                                source={`presentation.custom_urls.saml`}
                                                label={""}
                                                onChange={(e) =>
                                                    setCustomUrlsValues({
                                                        ...customUrlsValues,
                                                        saml: e.target.value,
                                                    })
                                                }
                                            />
                                            <p>{`.${globalSettings.CUSTOM_URLS_DOMAIN_NAME}`}</p>
                                            {isCustomUrlLoading ? (
                                                <WizardStyles.DownloadProgress size={18} />
                                            ) : (
                                                isCustomizeUrl &&
                                                (customSamlRes?.data?.set_custom_urls?.success ? (
                                                    <StatusChip status="SUCCESS" />
                                                ) : (
                                                    <StatusChip status="ERROR" />
                                                ))
                                            )}
                                        </CustomUrlsStyle.InputLabelWrapper>
                                        {customSamlRes &&
                                            !customSamlRes?.data?.set_custom_urls?.success && (
                                                <CustomUrlsStyle.ErrorText>
                                                    {customSamlRes?.data?.set_custom_urls?.message}
                                                </CustomUrlsStyle.ErrorText>
                                            )}
                                    </CustomUrlsStyle.InputWrapper>
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

                            <Accordion
                                sx={{width: "100%"}}
                                expanded={expanded === "voting-portal-countdown-policy"}
                                onChange={() => setExpanded("voting-portal-countdown-policy")}
                            >
                                <AccordionSummary
                                    expandIcon={
                                        <ExpandMoreIcon id="voting-portal-countdown-policy" />
                                    }
                                >
                                    <ElectionHeaderStyles.Wrapper>
                                        <ElectionHeaderStyles.Title>
                                            {t("electionEventScreen.edit.advancedConfigurations")}
                                        </ElectionHeaderStyles.Title>
                                    </ElectionHeaderStyles.Wrapper>
                                </AccordionSummary>
                                <AccordionDetails>
                                    <Typography
                                        variant="body1"
                                        component="span"
                                        sx={{
                                            fontWeight: "bold",
                                            margin: 0,
                                            display: {xs: "none", sm: "block"},
                                        }}
                                    >
                                        {t(
                                            "electionEventScreen.field.countDownPolicyOptions.sectionTitle"
                                        )}
                                    </Typography>
                                    <SelectInput
                                        source={`presentation.voting_portal_countdown_policy.policy`}
                                        choices={votingPortalCountDownPolicies()}
                                        label={t(
                                            "electionEventScreen.field.countDownPolicyOptions.policyLabel"
                                        )}
                                        defaultValue={EVotingPortalCountdownPolicy.NO_COUNTDOWN}
                                        emptyText={undefined}
                                        validate={required()}
                                    />
                                    <Box
                                        sx={{
                                            display: "flex",
                                            flexDirection: "row",
                                            justifyContent: "flex-end",
                                            alignItems: "center",
                                            gap: "16px",
                                        }}
                                    >
                                        <ManagedNumberInput
                                            source={
                                                "presentation.voting_portal_countdown_policy.countdown_anticipation_secs"
                                            }
                                            label={t(
                                                "electionEventScreen.field.countDownPolicyOptions.coundownSecondsLabel"
                                            )}
                                            defaultValue={defaultSecondsForCountdown}
                                            sourceToWatch="presentation.voting_portal_countdown_policy.policy"
                                        />

                                        <ManagedNumberInput
                                            source={
                                                "presentation.voting_portal_countdown_policy.countdown_alert_anticipation_secs"
                                            }
                                            label={t(
                                                "electionEventScreen.field.countDownPolicyOptions.alertSecondsLabel"
                                            )}
                                            defaultValue={defaultSecondsForAlret}
                                            sourceToWatch="presentation.voting_portal_countdown_policy.policy"
                                        />
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
function useCallBack(
    arg0: (presentation: IElectionEventPresentation, recordId: string) => Promise<void>,
    arg1: never[]
) {
    throw new Error("Function not implemented.")
}
