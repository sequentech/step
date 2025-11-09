// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    BooleanInput,
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
    required,
    FormDataConsumer,
    useGetList,
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
import VideoCallIcon from "@mui/icons-material/VideoCall"
import React, {useContext, useEffect, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ETemplateType} from "@/types/templates"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {
    ElectionsOrder,
    IElectionEventPresentation,
    IElectionPresentation,
    ITenantSettings,
    EVotingPortalCountdownPolicy,
    EElectionEventLockedDown,
    EElectionEventEnrollment,
    EElectionEventOTP,
    EElectionEventContestEncryptionPolicy,
    EVoterSigningPolicy,
    EShowCastVoteLogsPolicy,
    EElectionEventDecodedBallots,
    EElectionEventCeremoniesPolicy,
    EElectionEventWeightedVotingPolicy,
} from "@sequentech/ui-core"
import {ListActions} from "@/components/ListActions"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {ListSupportMaterials} from "../SupportMaterials/ListSuportMaterial"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {TVotingSetting} from "@/types/settings"
import {
    ImportCandidatesMutation,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    SetCustomUrlsMutation,
    Sequent_Backend_Template,
} from "@/gql/graphql"
import {ElectionStyles} from "@/components/styles/ElectionStyles"
import {FetchResult, useMutation} from "@apollo/client"
import {IMPORT_CANDIDTATES} from "@/queries/ImportCandidates"
import CustomOrderInput from "@/components/custom-order/CustomOrderInput"
import {convertToNumber} from "@/lib/helpers"
import {ExportElectionEventDrawer} from "../../components/election-event/export-data/ExportElectionEventDrawer"
import {ManagedNumberInput} from "@/components/managed-inputs/ManagedNumberInput"
import {ETasksExecution} from "@/types/tasksExecution"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {SET_CUSTOM_URLS} from "@/queries/SetCustomUrls"
import {getAuthUrl} from "@/services/UrlGeneration"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {CustomUrlsStyle} from "@/components/styles/CustomUrlsStyle"
import {StatusChip} from "@/components/StatusChip"
import {JsonEditor, UpdateFunction} from "json-edit-react"
import {CustomFilter} from "@/types/filters"
import {SET_VOTER_AOTHENTICATION} from "@/queries/SetVoterAuthentication"
import {GoogleMeetLinkGenerator} from "@/components/election-event/google-meet/GoogleMeetLinkGenerator"

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

    const canCreateGoogleMeeting = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.GOOGLE_MEET_LINK
    )

    const [value, setValue] = useState(0)
    const [valueMaterials, setValueMaterials] = useState(0)
    const [expanded, setExpanded] = useState("election-event-data-general")
    const [languageSettings, setLanguageSettings] = useState<Array<string>>(["en"])
    const [openExport, setOpenExport] = useState(false)
    const [loadingExport, setLoadingExport] = useState(false)
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const [openDrawer, setOpenDrawer] = useState<boolean>(false)
    const [openImportCandidates, setOpenImportCandidates] = useState(false)
    const [openGoogleMeet, setOpenGoogleMeet] = useState(false)
    const [importCandidates] = useMutation<ImportCandidatesMutation>(IMPORT_CANDIDTATES)
    const defaultSecondsForCountdown = convertToNumber(process.env.SECONDS_TO_SHOW_COUNTDOWN) ?? 60
    const defaultSecondsForAlert = convertToNumber(process.env.SECONDS_TO_SHOW_ALERT) ?? 180
    const [customUrlsValues, setCustomUrlsValues] = useState({login: "", enrollment: "", saml: ""})
    const [customLoginRes, setCustomLoginRes] = useState<FetchResult<SetCustomUrlsMutation>>()
    const [customEnrollmentRes, setCustomEnrollmentRes] =
        useState<FetchResult<SetCustomUrlsMutation>>()
    const [customSamlRes, setCustomSamlRes] = useState<FetchResult<SetCustomUrlsMutation>>()
    const [isCustomUrlLoading, setIsCustomUrlLoading] = useState(false)
    const [isCustomizeUrl, setIsCustomizeUrl] = useState(false)
    const [customFilters, setCustomFilters] = useState<CustomFilter[] | undefined>()
    const [activateSave, setActivateSave] = useState(false)
    const [voterAuthentication, setVoterAuthentication] = useState({
        enrollment: "",
        otp: "",
    })
    const [manageCustomUrls, response] = useMutation<SetCustomUrlsMutation>(SET_CUSTOM_URLS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EVENT_WRITE,
            },
        },
    })

    const [manageVoterAuthentication] = useMutation<SetCustomUrlsMutation>(SET_VOTER_AOTHENTICATION)

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

    const {data: verifyVoterTemplates} = useGetList<Sequent_Backend_Template>(
        "sequent_backend_template",
        {
            filter: {
                tenant_id: tenantId,
                type: ETemplateType.MANUAL_VERIFICATION,
            },
        }
    )

    const manuallyVerifyVoterTemplates = (): Array<EnumChoice<string>> => {
        if (!verifyVoterTemplates) {
            return []
        }
        const template_names = (verifyVoterTemplates as Sequent_Backend_Template[]).map((entry) => {
            return {
                id: entry.id,
                name: entry.template?.name,
            }
        })
        return template_names
    }

    const [votingSettings] = useState<TVotingSetting>({
        online: tenant?.voting_channels?.online || true,
        kiosk: tenant?.voting_channels?.kiosk || false,
        early_voting: tenant?.voting_channels?.early_voting || false,
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
        if (!customFilters) {
            if (
                temp?.presentation?.custom_filters &&
                temp?.presentation?.custom_filters.length > 0
            ) {
                setCustomFilters(temp.presentation.custom_filters)
            }
        }

        temp.presentation.enrollment = temp?.presentation.enrollment
        temp.presentation.otp = temp?.presentation.otp

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

    const showCastVoteLogsChoices = (): Array<EnumChoice<EShowCastVoteLogsPolicy>> => {
        return Object.values(EShowCastVoteLogsPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.showCastVoteLogs.options.${value.toLowerCase()}`),
        }))
    }

    const handleImportCandidates = async (documentId: string, sha256: string) => {
        setOpenImportCandidates(false)
        const currWidget = addWidget(ETasksExecution.IMPORT_CANDIDATES, undefined)
        try {
            let {data, errors} = await importCandidates({
                variables: {
                    documentId,
                    electionEventId: record.id,
                    sha256,
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

    const handleUpdateVoterAuthentication = async (
        presentation: IElectionEventPresentation,
        recordId: string
    ) => {
        try {
            const data = manageVoterAuthentication({
                variables: {
                    electionEventId: recordId,
                    enrollment: voterAuthentication.enrollment,
                    otp: voterAuthentication.otp,
                },
            })
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

    const decodedBallotsStateChoices = () => {
        return Object.values(EElectionEventDecodedBallots).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.decodedBallots.options.${value}`),
        }))
    }

    const lockdownStateChoices = () => {
        return Object.values(EElectionEventLockedDown).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.lockdownState.options.${value}`),
        }))
    }

    const contestEncryptionPolicyChoices = () => {
        return Object.values(EElectionEventContestEncryptionPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.contestEncryptionPolicy.options.${value}`),
        }))
    }

    const votingPortalCountDownPolicies = () => {
        return Object.values(EVotingPortalCountdownPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.countDownPolicyOptions.${value}`),
        }))
    }

    const voterSigningPolicyChoices = () => {
        return Object.values(EVoterSigningPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.voterSigningPolicy.${value}`),
        }))
    }

    const enrollmentChoices = () => {
        return Object.values(EElectionEventEnrollment).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.enrollment.options.${value}`),
        }))
    }

    const otpChoices = () => {
        return Object.values(EElectionEventOTP).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.otp.options.${value}`),
        }))
    }

    const ceremonyPolicyOptions = () => {
        return Object.values(EElectionEventCeremoniesPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.ceremoniesPolicy.options.${value}`),
        }))
    }

    const weightedVotingPolicyOptions = () => {
        return Object.values(EElectionEventWeightedVotingPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.weightedVotingPolicy.options.${value}`),
        }))
    }

    type UpdateFunctionProps = Parameters<UpdateFunction>[0]

    const updateCustomFilters = (
        values: Sequent_Backend_Election_Event_Extended,
        {newData}: UpdateFunctionProps
    ) => {
        values.presentation.custom_filters = newData
        setCustomFilters(newData as CustomFilter[])
        setActivateSave(true)
    }

    const handleEnrollmentChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
        setVoterAuthentication((prev) => ({
            ...prev,
            enrollment: event.target.value,
        }))
    }

    const handleOtpChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
        setVoterAuthentication((prev) => ({
            ...prev,
            otp: event.target.value,
        }))
    }

    const extraActionsButtons = () => {
        let buttons = [
            <Button
                className="import-candidates"
                onClick={() => setOpenImportCandidates(true)}
                label={t("electionEventScreen.edit.importCandidates")}
                key="1"
            >
                <DownloadIcon />
            </Button>,
        ]
        if (canCreateGoogleMeeting) {
            buttons.push(
                <Button
                    className="google-meet-generator"
                    onClick={() => setOpenGoogleMeet(true)}
                    label={t("googleMeet.generateButton", "Generate Google Meet")}
                    key="2"
                >
                    <VideoCallIcon />
                </Button>
            )
        }
        return buttons
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
                    isExportDisabled={openExport || loadingExport}
                    withColumns={false}
                    withFilter={false}
                    extraActions={extraActionsButtons()}
                />
            </Box>
            <RecordContext.Consumer>
                {(incoming) => {
                    const parsedValue = parseValues(
                        incoming as Sequent_Backend_Election_Event_Extended,
                        languageSettings
                    )
                    const onSave = async () => {
                        await handleUpdateCustomUrls(
                            parsedValue.presentation as IElectionEventPresentation,
                            record.id
                        )
                        await handleUpdateVoterAuthentication(
                            parsedValue.presentation as IElectionEventPresentation,
                            record.id
                        )
                        setActivateSave(false)
                    }
                    return (
                        <SimpleForm
                            defaultValues={{electionsOrder: sortedElections}}
                            validate={formValidator}
                            record={parsedValue}
                            toolbar={
                                <Toolbar>
                                    {canEdit && (
                                        <SaveButton
                                            onClick={() => {
                                                onSave()
                                            }}
                                            type="button"
                                            alwaysEnable={activateSave}
                                        />
                                    )}
                                </Toolbar>
                            }
                        >
                            <Accordion
                                sx={{width: "100%"}}
                                expanded={expanded === "election-event-data-general"}
                                onChange={() =>
                                    setExpanded((prev) =>
                                        prev === "election-event-data-general"
                                            ? ""
                                            : "election-event-data-general"
                                    )
                                }
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
                                expanded={expanded === "election-event-data-language"}
                                onChange={() =>
                                    setExpanded((prev) =>
                                        prev === "election-event-data-language"
                                            ? ""
                                            : "election-event-data-language"
                                    )
                                }
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
                                onChange={() =>
                                    setExpanded((prev) =>
                                        prev === "election-event-data-ballot-style"
                                            ? ""
                                            : "election-event-data-ballot-style"
                                    )
                                }
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
                                    <SelectInput
                                        source="presentation.show_cast_vote_logs"
                                        choices={showCastVoteLogsChoices()}
                                        validate={required()}
                                        defaultValue={EShowCastVoteLogsPolicy.HIDE_LOGS_TAB}
                                        label={t(
                                            "electionEventScreen.field.showCastVoteLogs.policyLabel"
                                        )}
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
                                expanded={expanded === "election-event-data-allowed"}
                                onChange={() =>
                                    setExpanded((prev) =>
                                        prev === "election-event-data-allowed"
                                            ? ""
                                            : "election-event-data-allowed"
                                    )
                                }
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
                                        <Grid size={{xs: 12, md: 6}}>
                                            {renderVotingChannels(parsedValue)}
                                        </Grid>
                                    </Grid>
                                </AccordionDetails>
                            </Accordion>

                            <Accordion
                                sx={{width: "100%"}}
                                expanded={expanded === "election-event-data-custom-urls"}
                                onChange={() =>
                                    setExpanded((prev) =>
                                        prev === "election-event-data-custom-urls"
                                            ? ""
                                            : "election-event-data-custom-urls"
                                    )
                                }
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
                                onChange={() =>
                                    setExpanded((prev) =>
                                        prev === "election-event-data-materials"
                                            ? ""
                                            : "election-event-data-materials"
                                    )
                                }
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
                                onChange={() =>
                                    setExpanded((prev) =>
                                        prev === "voting-portal-countdown-policy"
                                            ? ""
                                            : "voting-portal-countdown-policy"
                                    )
                                }
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
                                    <SelectInput
                                        source={"presentation.contest_encryption_policy"}
                                        choices={contestEncryptionPolicyChoices()}
                                        label={t(
                                            "electionEventScreen.field.contestEncryptionPolicy.policyLabel"
                                        )}
                                        defaultValue={
                                            EElectionEventContestEncryptionPolicy.SINGLE_CONTEST
                                        }
                                        emptyText={undefined}
                                        validate={required()}
                                    />
                                    <SelectInput
                                        source={"presentation.locked_down"}
                                        choices={lockdownStateChoices()}
                                        label={t(
                                            "electionEventScreen.field.lockdownState.policyLabel"
                                        )}
                                        defaultValue={EElectionEventLockedDown.NOT_LOCKED_DOWN}
                                        emptyText={undefined}
                                        validate={required()}
                                    />
                                    <SelectInput
                                        source={"presentation.decoded_ballot_inclusion_policy"}
                                        choices={decodedBallotsStateChoices()}
                                        label={t(
                                            "electionEventScreen.field.decodedBallots.policyLabel"
                                        )}
                                        defaultValue={EElectionEventDecodedBallots.NOT_INCLUDED}
                                        emptyText={undefined}
                                        validate={required()}
                                    />
                                    <SelectInput
                                        source={"presentation.ceremonies_policy"}
                                        choices={ceremonyPolicyOptions()}
                                        label={t(
                                            "electionEventScreen.field.ceremoniesPolicy.policyLabel"
                                        )}
                                        defaultValue={
                                            EElectionEventCeremoniesPolicy.MANUAL_CEREMONIES
                                        }
                                        emptyText={undefined}
                                        validate={required()}
                                    />
                                    <SelectInput
                                        source={"presentation.weighted_voting_policy"}
                                        choices={weightedVotingPolicyOptions()}
                                        label={"Weighted Voting Policy"}
                                        defaultValue={
                                            EElectionEventWeightedVotingPolicy.DISABLED_WEIGHTED_VOTING
                                        }
                                        emptyText={undefined}
                                        validate={required()}
                                    />
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
                                    <SelectInput
                                        source={"presentation.voter_signing_policy"}
                                        choices={voterSigningPolicyChoices()}
                                        label={t(
                                            "electionEventScreen.field.voterSigningPolicy.policyLabel"
                                        )}
                                        defaultValue={EVoterSigningPolicy.NO_SIGNATURE}
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
                                            isDisabled={(selectedPolicy) =>
                                                selectedPolicy ===
                                                EVotingPortalCountdownPolicy.NO_COUNTDOWN
                                            }
                                        />

                                        <ManagedNumberInput
                                            source={
                                                "presentation.voting_portal_countdown_policy.countdown_alert_anticipation_secs"
                                            }
                                            label={t(
                                                "electionEventScreen.field.countDownPolicyOptions.alertSecondsLabel"
                                            )}
                                            defaultValue={defaultSecondsForAlert}
                                            sourceToWatch="presentation.voting_portal_countdown_policy.policy"
                                            isDisabled={(selectedPolicy) =>
                                                selectedPolicy !==
                                                EVotingPortalCountdownPolicy.COUNTDOWN_WITH_ALERT
                                            }
                                        />
                                    </Box>
                                    <Box>
                                        <Typography
                                            variant="body1"
                                            component="span"
                                            sx={{
                                                padding: "1rem 0rem",
                                                fontWeight: "bold",
                                                margin: 0,
                                                display: {xs: "none", sm: "block"},
                                            }}
                                        >
                                            {t("electionEventScreen.edit.custom_filters")}
                                        </Typography>

                                        <JsonEditor
                                            data={customFilters ?? []}
                                            onUpdate={(data) =>
                                                updateCustomFilters(
                                                    parsedValue,
                                                    data as UpdateFunctionProps
                                                )
                                            }
                                        />
                                    </Box>
                                    <Box>
                                        <Typography
                                            variant="body1"
                                            component="span"
                                            sx={{
                                                padding: "1rem 0rem",
                                                fontWeight: "bold",
                                                margin: 0,
                                                display: {xs: "none", sm: "block"},
                                            }}
                                        >
                                            {t("electionEventScreen.edit.voter_authentication")}
                                        </Typography>
                                        <SelectInput
                                            label={t(
                                                `electionEventScreen.field.enrollment.policyLabel`
                                            )}
                                            source="presentation.enrollment"
                                            choices={enrollmentChoices()}
                                            onChange={(value) => handleEnrollmentChange(value)}
                                        />
                                        <SelectInput
                                            label={t(`electionEventScreen.field.otp.policyLabel`)}
                                            source="presentation.otp"
                                            choices={otpChoices()}
                                            onChange={(value) => handleOtpChange(value)}
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

            <ExportElectionEventDrawer
                electionEventId={record.id}
                openExport={openExport}
                setOpenExport={setOpenExport}
                exportDocumentId={exportDocumentId}
                setExportDocumentId={setExportDocumentId}
                setLoadingExport={setLoadingExport}
            />

            {canCreateGoogleMeeting && (
                <GoogleMeetLinkGenerator
                    open={openGoogleMeet}
                    onClose={() => setOpenGoogleMeet(false)}
                    electionEventName={
                        (record?.presentation as IElectionEventPresentation | undefined)?.i18n?.en
                            ?.name ||
                        (record?.presentation as IElectionEventPresentation | undefined)?.i18n?.[
                            Object.keys(
                                (record?.presentation as IElectionEventPresentation | undefined)
                                    ?.i18n || {}
                            )[0]
                        ]?.name ||
                        "Election Event"
                    }
                />
            )}
        </>
    )
}
function useCallBack(
    arg0: (presentation: IElectionEventPresentation, recordId: string) => Promise<void>,
    arg1: never[]
) {
    throw new Error("Function not implemented.")
}
