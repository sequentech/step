// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
    useNotify,
    Button,
    SelectInput,
    RadioButtonGroupInput,
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
import {styled} from "@mui/material/styles"
import DownloadIcon from "@mui/icons-material/Download"
import VideoCallIcon from "@mui/icons-material/VideoCall"
import React, {useCallback, useContext, useEffect, useMemo, useRef, useState} from "react"
import {useFormContext} from "react-hook-form"
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
    EVoterCertificatePolicy,
    EShowCastVoteLogsPolicy,
    EElectionEventDecodedBallots,
    EElectionEventCeremoniesPolicy,
    EElectionEventAutomaticRecountPolicy,
    EElectionEventWeightedVotingPolicy,
    EElectionEventDelegatedVotingPolicy,
    EResultsWebsiteAccess,
    EResultsWebsiteStatus,
    EResultsWebsiteVisibilityScope,
    IResultsWebsitePolicy,
    defaultResultsWebsitePolicy,
    parseResultsWebsitePolicy,
    EVotingPortalDateTimeFormat,
    VotingPortalDateTimeFormat,
    isCustomVotingPortalDateTimeFormat,
    isValidVotingPortalDateTimePattern,
    ELanguageDetectionPolicy,
    getDefaultLanguageDetectionPolicy,
    REALM_ATTR_VOTER_CERTIFICATE_POLICY,
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
import {FetchResult, useMutation, useQuery} from "@apollo/client"
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
import {
    UPDATE_REALM_ATTRIBUTES,
    UpdateRealmAttributesMutation,
} from "@/queries/UpdateRealmAttributes"
import {GET_REALM_ATTRIBUTES, GetRealmAttributesQuery} from "@/queries/GetRealmAttributes"
import {GoogleMeetLinkGenerator} from "@/components/election-event/google-meet/GoogleMeetLinkGenerator"
import {
    PasswordPolicyAccordion,
    PasswordPolicyAccordionHandle,
} from "@/components/election-event/PasswordPolicyAccordion"
import {SettingsLanguageSelector} from "../../components/SettingsLanguageSelector"
import {
    CONFIGURE_RESULTS_WEBSITE_POLICY,
    ConfigureResultsWebsitePolicyData,
    ConfigureResultsWebsitePolicyVariables,
} from "@/queries/ResultsWebsitePublication"

export type Sequent_Backend_Election_Event_Extended = RaRecord<Identifier> & {
    enabled_languages?: {[key: string]: boolean}
    defaultLanguage?: string
    electionsOrder?: Array<Sequent_Backend_Election>
    resultsWebsitePolicy?: IResultsWebsitePolicy
} & Sequent_Backend_Election_Event

const ResultsWebsitePolicyFields: React.FC = () => {
    const {t} = useTranslation()
    const statusOptions = [
        {id: EResultsWebsiteStatus.DISABLED, name: t("tally.resultsPublication.disabled")},
        {id: EResultsWebsiteStatus.ENABLED, name: t("tally.resultsPublication.enabled")},
    ]
    const accessOptions = [
        {id: EResultsWebsiteAccess.PUBLIC, name: t("tally.resultsPublication.publicAccess")},
        {
            id: EResultsWebsiteAccess.AUTHENTICATED,
            name: t("tally.resultsPublication.authenticatedAccess"),
        },
    ]
    const visibilityOptions = [
        {
            id: EResultsWebsiteVisibilityScope.FULL_EVENT,
            name: t("tally.resultsPublication.fullEvent"),
        },
        {
            id: EResultsWebsiteVisibilityScope.AREA_BASED,
            name: t("tally.resultsPublication.areaBased"),
        },
    ]

    return (
        <>
            <Typography
                variant="body1"
                component="span"
                sx={{
                    fontWeight: "bold",
                    margin: 0,
                    display: {xs: "none", sm: "block"},
                }}
            >
                {t("tally.resultsPublication.policyTitle")}
            </Typography>
            <SelectInput
                source={"resultsWebsitePolicy.status"}
                choices={statusOptions}
                label={t("tally.resultsPublication.policyTitle")}
                defaultValue={EResultsWebsiteStatus.DISABLED}
                emptyText={undefined}
                validate={required()}
            />
            <SelectInput
                source={"resultsWebsitePolicy.access"}
                choices={accessOptions}
                label={t("tally.resultsPublication.policyAccess")}
                defaultValue={EResultsWebsiteAccess.PUBLIC}
                emptyText={undefined}
                validate={required()}
            />
            <SelectInput
                source={"resultsWebsitePolicy.visibility_scope"}
                choices={visibilityOptions}
                label={t("tally.resultsPublication.policyVisibility")}
                defaultValue={EResultsWebsiteVisibilityScope.FULL_EVENT}
                emptyText={undefined}
                validate={required()}
            />
        </>
    )
}

const ElectionRows = styled("div")`
    display: flex;
    flex-direction: column;
    width: 100%;
    cursor: pointer;
    margin-bottom: 0.1rem;
    padding: 1rem;
`

// Mirrors the Localization tab's notify.invalidDateTimeFormat feedback for the
// main custom date/time format field. Reads live form values via getValues()
// at click time (like the Localization tab's own save handlers do) instead of
// watching react-hook-form state: submitCount/errors get reset whenever
// react-admin's SimpleForm receives a new `record` reference (e.g. an Apollo
// refetch), which happens far more often than actual submit attempts here.
const CustomDateTimeFormatInvalidNotifier: React.FC<{
    checkRef: React.MutableRefObject<() => void>
}> = ({checkRef}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const {getValues} = useFormContext()

    checkRef.current = () => {
        const configured = getValues(
            "presentation.voting_portal_datetime_format"
        ) as VotingPortalDateTimeFormat

        if (
            isCustomVotingPortalDateTimeFormat(configured) &&
            !isValidVotingPortalDateTimePattern(configured.custom)
        ) {
            notify(t("electionEventScreen.localization.notify.invalidDateTimeFormat"), {
                type: "error",
            })
        }
    }

    return null
}

export const EditElectionEventDataForm: React.FC<{
    transform: (data: Sequent_Backend_Election_Event_Extended) => Promise<RaRecord<Identifier>>
}> = ({transform}) => {
    const {t} = useTranslation()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const notify = useNotify()
    const checkCustomDateTimeFormatRef = useRef<() => void>(() => {})
    const passwordPolicyRef = useRef<PasswordPolicyAccordionHandle | null>(null)

    const canEdit = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )
    const canReadPasswordPolicy = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_READ
    )
    const canReadRealmAttributes = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.KEYCLOAK_REALM_ATTRIBUTES_READ
    )
    const canEditRealmAttributes = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.KEYCLOAK_REALM_ATTRIBUTES_WRITE
    )
    const canSave = canEdit || (canReadRealmAttributes && canEditRealmAttributes)

    const canCreateGoogleMeeting = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.GOOGLE_MEET_LINK
    )

    const canConfigureResultsWebsite = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.PUBLISH_RESULTS_WRITE
    )

    const [value, setValue] = useState(0)
    const [valueMaterials, setValueMaterials] = useState(0)
    const [expanded, setExpanded] = useState("election-event-data-general")
    const [languageSettings, setLanguageSettings] = useState<Array<string>>(["en"])
    const [openExport, setOpenExport] = useState(false)
    const [loadingExport, setLoadingExport] = useState(false)
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
    const [realmAttributes, setRealmAttributes] = useState<Record<string, string>>({})
    const [realmAttributesError, setRealmAttributesError] = useState<string>()
    const [realmAttributesDirty, setRealmAttributesDirty] = useState(false)
    const [manageCustomUrls, response] = useMutation<SetCustomUrlsMutation>(SET_CUSTOM_URLS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EVENT_WRITE,
            },
        },
    })

    const [manageVoterAuthentication] = useMutation<SetCustomUrlsMutation>(SET_VOTER_AOTHENTICATION)
    const {
        data: realmAttributesData,
        loading: isRealmAttributesLoading,
        error: realmAttributesQueryError,
        refetch: refetchRealmAttributes,
    } = useQuery<GetRealmAttributesQuery>(GET_REALM_ATTRIBUTES, {
        variables: {
            election_event_id: record?.id,
        },
        skip: !record?.id || !canReadRealmAttributes,
        fetchPolicy: "network-only",
        context: {
            headers: {
                "x-hasura-role": IPermissions.KEYCLOAK_REALM_ATTRIBUTES_READ,
            },
        },
    })
    const [manageRealmAttributes] = useMutation<UpdateRealmAttributesMutation>(
        UPDATE_REALM_ATTRIBUTES,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.KEYCLOAK_REALM_ATTRIBUTES_WRITE,
                },
            },
        }
    )
    const [configureResultsWebsitePolicy] = useMutation<
        ConfigureResultsWebsitePolicyData,
        ConfigureResultsWebsitePolicyVariables
    >(CONFIGURE_RESULTS_WEBSITE_POLICY, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.PUBLISH_RESULTS_WRITE,
            },
        },
    })

    const {record: tenant} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })
    const {data: elections} = useGetList<Sequent_Backend_Election>("sequent_backend_election", {
        filter: {
            tenant_id: record?.tenant_id,
            election_event_id: record?.id,
        },
        pagination: {page: 1, perPage: 9999},
    })

    const [votingSettings] = useState<TVotingSetting>({
        online: tenant?.voting_channels?.online || true,
        kiosk: tenant?.voting_channels?.kiosk || false,
        early_voting: tenant?.voting_channels?.early_voting || false,
        telephone: tenant?.voting_channels?.telephone || false,
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

    const parseValues = useCallback(
        (
            incoming: Sequent_Backend_Election_Event_Extended,
            languageSettings: Array<string>
        ): Sequent_Backend_Election_Event_Extended => {
            const temp = {...incoming}

            temp.presentation = {...(incoming.presentation || {})}

            // languages
            temp.enabled_languages = {}

            const incomingLangConf = (
                incoming?.presentation as IElectionEventPresentation | undefined
            )?.language_conf

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

            // Force English first
            if (temp.enabled_languages.en !== undefined) {
                const {en, ...rest} = temp.enabled_languages
                temp.enabled_languages = {en, ...rest}
            }

            // delete incoming.voting_channels
            temp.voting_channels = {}
            const defaultChannels: TVotingSetting = {
                online: true,
                kiosk: false,
                early_voting: false,
                telephone: false,
            }
            for (const channel of Object.keys(defaultChannels)) {
                temp.voting_channels[channel] =
                    incoming.voting_channels?.[channel] ?? defaultChannels[channel]
            }

            temp.presentation.elections_order ??= ElectionsOrder.ALPHABETICAL

            if (!temp.presentation.voting_portal_countdown_policy) {
                temp.presentation.voting_portal_countdown_policy = {
                    policy: EVotingPortalCountdownPolicy.NO_COUNTDOWN,
                }
            }

            temp.presentation.custom_urls ??= {}
            temp.resultsWebsitePolicy =
                parseResultsWebsitePolicy(temp.presentation.results_website) ??
                defaultResultsWebsitePolicy()

            return temp
        },
        [votingSettings]
    )

    useEffect(() => {
        if (
            record?.presentation?.custom_filters &&
            record.presentation.custom_filters.length > 0 &&
            !customFilters
        ) {
            setCustomFilters(record.presentation.custom_filters)
        }
    }, [record?.presentation?.custom_filters, customFilters])

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    const handleChangeMaterials = (event: React.SyntheticEvent, newValue: number) => {
        setValueMaterials(newValue)
    }

    // This form uses form-level validation: react-admin turns this `validate`
    // prop into a react-hook-form resolver, and react-hook-form ignores all
    // input-level `validate` props when a resolver is present. Any field
    // validation for this form must therefore live here, keyed by the field's
    // source path so the error reaches the input's helper text.
    const formValidator = (values: {
        presentation?: {
            voting_portal_datetime_format?: VotingPortalDateTimeFormat
            weighted_voting_policy?: EElectionEventWeightedVotingPolicy
            delegated_voting_policy?: EElectionEventDelegatedVotingPolicy
            decoded_ballot_inclusion_policy?: EElectionEventDecodedBallots
        }
    }): Record<string, unknown> => {
        const errors: Record<string, unknown> = {}
        const presentationErrors: Record<string, unknown> = {}
        const dateTimeFormat = values?.presentation?.voting_portal_datetime_format
        if (
            isCustomVotingPortalDateTimeFormat(dateTimeFormat) &&
            !isValidVotingPortalDateTimePattern(dateTimeFormat.custom)
        ) {
            presentationErrors.voting_portal_datetime_format = {
                custom: String(
                    t("electionEventScreen.field.votingPortalDateTimeFormat.customFormat.invalid")
                ),
            }
        }

        // A voter's weight is applied by counting their ballot more than once, so it has no
        // defined meaning combined with a delegated ballot, and publishing the
        // decoded ballots would show the weight as a run of identical
        // plaintexts. The tally refuses both, but only once voting has closed.
        // Each message is also keyed onto the field it conflicts with, so the
        // error is visible whichever of the two the operator is looking at.
        const weightedPolicyMessages: string[] = []
        if (
            values?.presentation?.weighted_voting_policy ===
            EElectionEventWeightedVotingPolicy.VOTERS_WEIGHTED_VOTING
        ) {
            if (
                values?.presentation?.delegated_voting_policy ===
                EElectionEventDelegatedVotingPolicy.ENABLED
            ) {
                const message = String(
                    t("electionEventScreen.field.weightedVotingPolicy.noDelegated")
                )
                weightedPolicyMessages.push(message)
                presentationErrors.delegated_voting_policy = message
            }
            if (
                values?.presentation?.decoded_ballot_inclusion_policy ===
                EElectionEventDecodedBallots.INCLUDED
            ) {
                const message = String(
                    t("electionEventScreen.field.weightedVotingPolicy.noDecodedBallots")
                )
                weightedPolicyMessages.push(message)
                presentationErrors.decoded_ballot_inclusion_policy = message
            }
        }

        // Both conflicts can hold at once, and one assignment would replace the
        // other, so the weighted field reports every conflict it has.
        if (weightedPolicyMessages.length > 0) {
            presentationErrors.weighted_voting_policy = weightedPolicyMessages.join(" ")
        }

        if (Object.keys(presentationErrors).length > 0) {
            errors.presentation = presentationErrors
        }
        return errors
    }

    const renderVotingChannels = (parsedValue: Sequent_Backend_Election_Event_Extended) => {
        let channelNodes = []
        for (const channel in parsedValue?.voting_channels) {
            channelNodes.push(
                <BooleanInput
                    disabled={!canEdit}
                    key={channel}
                    source={`voting_channels[${channel}]`}
                    label={String(t(`common.channel.${channel}`))}
                />
            )
        }
        return channelNodes
    }

    const renderTabs = useCallback(
        (parsedValue: Sequent_Backend_Election_Event_Extended, type: string = "general") => {
            let tabNodes = []
            for (const lang in parsedValue?.enabled_languages) {
                if (parsedValue?.enabled_languages[lang]) {
                    tabNodes.push(
                        <Tab
                            key={lang}
                            label={String(t(`common.language.${lang}`))}
                            id={lang}
                        ></Tab>
                    )
                }
            }

            return tabNodes
        },
        [t]
    )

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
                                label={String(t("electionEventScreen.field.name"))}
                            />
                            <TextInput
                                disabled={!canEdit}
                                source={`presentation.i18n[${lang}].alias`}
                                label={String(t("electionEventScreen.field.alias"))}
                            />
                            <TextInput
                                disabled={!canEdit}
                                source={`presentation.i18n[${lang}].description`}
                                label={String(t("electionEventScreen.field.description"))}
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
                                label={String(t("electionEventScreen.field.materialTitle"))}
                            />
                            <TextInput
                                disabled={!canEdit}
                                source={`presentation.i18n[${lang}].materialsSubtitle`}
                                label={String(t("electionEventScreen.field.materialSubTitle"))}
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
                    electionEventId: record?.id,
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

    const sortedElections = (elections ?? []).sort((a, b) => {
        let presentationA = a.presentation as IElectionPresentation | undefined
        let presentationB = b.presentation as IElectionPresentation | undefined
        let sortOrderA = presentationA?.sort_order ?? -1
        let sortOrderB = presentationB?.sort_order ?? -1
        return sortOrderA - sortOrderB
    })

    const parsedValue = useMemo(
        () => parseValues(record as Sequent_Backend_Election_Event_Extended, languageSettings),
        [record, languageSettings, parseValues]
    )

    const defaultValues = useMemo(() => ({electionsOrder: sortedElections}), [sortedElections])

    useEffect(() => {
        const enabledCount = Object.values(parsedValue?.enabled_languages ?? {}).filter(
            Boolean
        ).length

        if (enabledCount === 1) {
            setValue(0)
            setValueMaterials(0)
        }
    }, [parsedValue?.enabled_languages, setValue, setValueMaterials])

    useEffect(() => {
        const attributes = realmAttributesData?.get_realm_attributes?.attributes
        if (attributes) {
            setRealmAttributes(attributes)
            setRealmAttributesError(undefined)
            setRealmAttributesDirty(false)
        }
    }, [realmAttributesData])

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

    const votingPortalDateTimeFormatChoices = () => {
        return Object.values(EVotingPortalDateTimeFormat).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.votingPortalDateTimeFormat.options.${value}`),
        }))
    }

    // The policy dropdown edits a scalar discriminant, but the CUSTOM policy stores its
    // pattern inline as `{custom: "..."}`. These map between the two representations so the
    // preset and custom variants share the single `voting_portal_datetime_format` field.
    const dateTimePolicyToSelectValue = (
        value: VotingPortalDateTimeFormat | undefined
    ): EVotingPortalDateTimeFormat | "" =>
        isCustomVotingPortalDateTimeFormat(value)
            ? EVotingPortalDateTimeFormat.CUSTOM
            : (value ?? "")

    const selectValueToDateTimePolicy = (
        id: EVotingPortalDateTimeFormat
    ): VotingPortalDateTimeFormat => (id === EVotingPortalDateTimeFormat.CUSTOM ? {custom: ""} : id)

    const voterSigningPolicyChoices = () => {
        return Object.values(EVoterSigningPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.voterSigningPolicy.${value}`),
        }))
    }

    const VoterCertificatePolicyChoices = () => {
        return Object.values(EVoterCertificatePolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.VoterCertificatePolicy.${value}`),
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

    const automaticRecountPolicyOptions = () => {
        return Object.values(EElectionEventAutomaticRecountPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.automaticRecountPolicy.options.${value}`),
        }))
    }

    const weightedVotingPolicyOptions = () => {
        return Object.values(EElectionEventWeightedVotingPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.weightedVotingPolicy.options.${value}`),
        }))
    }

    const delegatedVotingPolicyOptions = () => {
        return Object.values(EElectionEventDelegatedVotingPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.delegatedVotingPolicy.options.${value}`),
        }))
    }

    const languageDetectionPolicyOptions = () => {
        return Object.values(ELanguageDetectionPolicy).map((value) => ({
            id: value,
            name: t(`electionEventScreen.field.languageDetectionPolicy.options.${value}`),
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

    const normalizeRealmAttributes = (data: unknown): Record<string, string> => {
        if (!data || Array.isArray(data) || typeof data !== "object") {
            throw new Error("Realm attributes must be a JSON object")
        }

        return Object.entries(data).reduce<Record<string, string>>((acc, [key, value]) => {
            if (key.trim().length === 0) {
                throw new Error("Realm attribute names cannot be blank")
            }
            let hasControlCharacter = false
            for (let index = 0; index < key.length; index++) {
                const code = key.charCodeAt(index)
                if (code <= 31 || (code >= 127 && code <= 159)) {
                    hasControlCharacter = true
                    break
                }
            }
            if (hasControlCharacter) {
                throw new Error("Realm attribute names cannot contain control characters")
            }
            if (typeof value !== "string") {
                throw new Error("Realm attribute values must be strings")
            }

            acc[key] = value
            return acc
        }, {})
    }

    const updateRealmAttributesDraft = ({newData}: UpdateFunctionProps) => {
        try {
            setRealmAttributes(normalizeRealmAttributes(newData))
            setRealmAttributesError(undefined)
            setRealmAttributesDirty(true)
            setActivateSave(true)
        } catch (error) {
            const message = error instanceof Error ? error.message : "Invalid realm attributes"
            setRealmAttributesError(message)
            setActivateSave(true)
            return false
        }
    }

    const setRealmAttributeDraftValue = (key: string, value: string) => {
        if (!canEditRealmAttributes) {
            return
        }
        setRealmAttributes((prev) => ({...prev, [key]: value}))
        setRealmAttributesDirty(true)
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
                label={String(t("electionEventScreen.edit.importCandidates"))}
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
                    label={String(t("googleMeet.generateButton", "Generate Google Meet"))}
                    key="2"
                >
                    <VideoCallIcon />
                </Button>
            )
        }
        return buttons
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

    const handleUpdateRealmAttributes = async (recordId: string) => {
        if (realmAttributesError) {
            notify(realmAttributesError, {type: "error"})
            return false
        }
        // Never push a draft based on attributes that failed to load: the
        // edits were made against incomplete data.
        if (canReadRealmAttributes && (isRealmAttributesLoading || realmAttributesQueryError)) {
            notify(t("electionEventScreen.edit.realm_attributes_not_loaded"), {type: "error"})
            return false
        }

        try {
            await manageRealmAttributes({
                variables: {
                    election_event_id: recordId,
                    attributes: normalizeRealmAttributes(realmAttributes),
                },
            })
            if (canReadRealmAttributes) {
                await refetchRealmAttributes()
            }
            setRealmAttributesDirty(false)
            return true
        } catch (err: any) {
            console.error(err)
            notify(t("electionEventScreen.edit.realm_attributes_update_error"), {type: "error"})
            return false
        }
    }

    const handleConfigureResultsWebsitePolicy = async (
        policy: IResultsWebsitePolicy | undefined,
        recordId: string
    ) => {
        if (!canConfigureResultsWebsite) {
            return
        }
        if (!policy) {
            throw new Error("Results website policy is missing")
        }
        if (
            policy.access === EResultsWebsiteAccess.PUBLIC &&
            policy.visibility_scope !== EResultsWebsiteVisibilityScope.FULL_EVENT
        ) {
            throw new Error("Public results must use full event visibility")
        }

        await configureResultsWebsitePolicy({
            variables: {
                election_event_id: recordId,
                status: policy.status,
                access: policy.access,
                visibility_scope: policy.visibility_scope,
            },
        })
    }

    const onSave = async (values: Sequent_Backend_Election_Event_Extended) => {
        const recordId = values.id?.toString() ?? record?.id?.toString()
        if (!recordId) {
            throw new Error("Election event ID is missing")
        }
        checkCustomDateTimeFormatRef.current()

        if (canEdit) {
            await handleUpdateCustomUrls(
                values.presentation as IElectionEventPresentation,
                recordId
            )
            await handleUpdateVoterAuthentication(
                values.presentation as IElectionEventPresentation,
                recordId
            )
        }

        if (canEditRealmAttributes && realmAttributesDirty) {
            const updatedRealmAttributes = await handleUpdateRealmAttributes(recordId)
            if (!updatedRealmAttributes) {
                throw new Error("Realm attributes could not be updated")
            }
        }

        await handleConfigureResultsWebsitePolicy(values.resultsWebsitePolicy, recordId)
        if (canEdit) {
            const passwordPolicyUpdated = await passwordPolicyRef.current?.save()
            if (passwordPolicyUpdated === false) {
                throw new Error("Password policy could not be updated")
            }
        }
        setActivateSave(false)

        return {
            ...values,
            presentation: {
                ...values.presentation,
                ...(canConfigureResultsWebsite && values.resultsWebsitePolicy
                    ? {results_website: JSON.stringify(values.resultsWebsitePolicy)}
                    : {}),
            },
        }
    }

    const saveTransform = async (values: Sequent_Backend_Election_Event_Extended) => {
        try {
            return await transform(await onSave(values))
        } catch (error) {
            notify(error instanceof Error ? error.message : String(error), {type: "error"})
            throw error
        }
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
            <SimpleForm
                defaultValues={defaultValues}
                validate={formValidator}
                record={parsedValue}
                toolbar={
                    <Toolbar>
                        {canSave && (
                            <SaveButton
                                type="button"
                                transform={saveTransform}
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
                        expandIcon={<ExpandMoreIcon id="election-event-data-language" />}
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
                                <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
                                    <SettingsLanguageSelector languageSettings={languageSettings} />
                                    <SelectInput
                                        source={
                                            "presentation.language_conf.language_detection_policy"
                                        }
                                        choices={languageDetectionPolicyOptions()}
                                        label={String(
                                            t(
                                                "electionEventScreen.field.languageDetectionPolicy.policyLabel"
                                            )
                                        )}
                                        defaultValue={getDefaultLanguageDetectionPolicy()}
                                        emptyText={undefined}
                                        validate={required()}
                                    />
                                </Box>
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
                        expandIcon={<ExpandMoreIcon id="election-event-data-ballot-style" />}
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
                            label={String(t(`electionEventScreen.field.skipElectionList`))}
                        />
                        <BooleanInput
                            disabled={!canEdit}
                            source={"presentation.show_user_profile"}
                            label={String(t(`electionEventScreen.field.showUserProfile`))}
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
                            label={String(
                                t("electionEventScreen.field.showCastVoteLogs.policyLabel")
                            )}
                        />
                        <FormDataConsumer>
                            {({formData, ...rest}) => {
                                return (
                                    formData?.presentation as IElectionEventPresentation | undefined
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
                                        <Box sx={{width: "100%", height: "180px"}}></Box>
                                    </ElectionRows>
                                ) : null
                            }}
                        </FormDataConsumer>
                        <TextInput
                            resettable={true}
                            source={"presentation.logo_url"}
                            label={String(t("electionEventScreen.field.logoUrl"))}
                        />
                        <TextInput
                            resettable={true}
                            source={"presentation.redirect_finish_url"}
                            label={String(t("electionEventScreen.field.redirectFinishUrl"))}
                        />
                        <TextInput
                            resettable={true}
                            multiline={true}
                            source={"presentation.css"}
                            label={String(t("electionEventScreen.field.css"))}
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
                            <Grid size={{xs: 12, md: 6}}>{renderVotingChannels(parsedValue)}</Grid>
                            <Grid size={{xs: 12, md: 6}}>
                                <RadioButtonGroupInput
                                    disabled={!canEdit}
                                    source={"presentation.automatic_recount_policy"}
                                    choices={automaticRecountPolicyOptions()}
                                    label={String(
                                        t(
                                            "electionEventScreen.field.automaticRecountPolicy.policyLabel"
                                        )
                                    )}
                                    defaultValue={EElectionEventAutomaticRecountPolicy.DISABLED}
                                    validate={required()}
                                />
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
                        expandIcon={<ExpandMoreIcon id="election-event-data-custom-urls" />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("electionEventScreen.edit.customUrls")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <CustomUrlsStyle.InputWrapper>
                            <CustomUrlsStyle.InputLabel>Login:</CustomUrlsStyle.InputLabel>
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
                            {customLoginRes && !customLoginRes?.data?.set_custom_urls?.success && (
                                <CustomUrlsStyle.ErrorText>
                                    {customLoginRes?.data?.set_custom_urls?.message}
                                </CustomUrlsStyle.ErrorText>
                            )}
                        </CustomUrlsStyle.InputWrapper>
                        <CustomUrlsStyle.InputWrapper>
                            <CustomUrlsStyle.InputLabel>Enrollment:</CustomUrlsStyle.InputLabel>
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
                                    (customEnrollmentRes?.data?.set_custom_urls?.success ? (
                                        <StatusChip status="SUCCESS" />
                                    ) : (
                                        <StatusChip status="ERROR" />
                                    ))
                                )}
                            </CustomUrlsStyle.InputLabelWrapper>
                            {customEnrollmentRes &&
                                !customEnrollmentRes?.data?.set_custom_urls?.success && (
                                    <CustomUrlsStyle.ErrorText>
                                        {customEnrollmentRes?.data?.set_custom_urls?.message}
                                    </CustomUrlsStyle.ErrorText>
                                )}
                        </CustomUrlsStyle.InputWrapper>
                        <CustomUrlsStyle.InputWrapper>
                            <CustomUrlsStyle.InputLabel>SAML:</CustomUrlsStyle.InputLabel>
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
                            {customSamlRes && !customSamlRes?.data?.set_custom_urls?.success && (
                                <CustomUrlsStyle.ErrorText>
                                    {customSamlRes?.data?.set_custom_urls?.message}
                                </CustomUrlsStyle.ErrorText>
                            )}
                        </CustomUrlsStyle.InputWrapper>
                    </AccordionDetails>
                </Accordion>

                {canReadRealmAttributes && (
                    <Accordion
                        sx={{width: "100%"}}
                        expanded={expanded === "election-event-data-realm-attributes"}
                        onChange={() =>
                            setExpanded((prev) =>
                                prev === "election-event-data-realm-attributes"
                                    ? ""
                                    : "election-event-data-realm-attributes"
                            )
                        }
                    >
                        <AccordionSummary
                            expandIcon={
                                <ExpandMoreIcon id="election-event-data-realm-attributes" />
                            }
                        >
                            <ElectionHeaderStyles.Wrapper>
                                <ElectionHeaderStyles.Title>
                                    {t("electionEventScreen.edit.realm_attributes")}
                                </ElectionHeaderStyles.Title>
                            </ElectionHeaderStyles.Wrapper>
                        </AccordionSummary>
                        <AccordionDetails>
                            {realmAttributesQueryError && (
                                <Typography color="error">
                                    {t("electionEventScreen.edit.realm_attributes_load_error")}
                                </Typography>
                            )}
                            {realmAttributesError && (
                                <Typography color="error">{realmAttributesError}</Typography>
                            )}
                            {isRealmAttributesLoading ? (
                                <Typography>{t("loading")}</Typography>
                            ) : canEditRealmAttributes ? (
                                <JsonEditor
                                    data={realmAttributes}
                                    onUpdate={(data) =>
                                        updateRealmAttributesDraft(data as UpdateFunctionProps)
                                    }
                                />
                            ) : (
                                <Box
                                    component="pre"
                                    sx={{
                                        backgroundColor: "rgba(0, 0, 0, 0.04)",
                                        borderRadius: "4px",
                                        margin: 0,
                                        overflowX: "auto",
                                        padding: "1rem",
                                    }}
                                >
                                    {JSON.stringify(realmAttributes, null, 2)}
                                </Box>
                            )}
                        </AccordionDetails>
                    </Accordion>
                )}

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
                        expandIcon={<ExpandMoreIcon id="election-event-data-materials" />}
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
                            label={String(t(`electionEventScreen.field.materialActivated`))}
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

                {canReadPasswordPolicy && (
                    <PasswordPolicyAccordion
                        ref={passwordPolicyRef}
                        electionEventId={record?.id?.toString()}
                        canEdit={canEdit}
                        expanded={expanded === "election-event-data-password-policy"}
                        onChange={() =>
                            setExpanded((previous) =>
                                previous === "election-event-data-password-policy"
                                    ? ""
                                    : "election-event-data-password-policy"
                            )
                        }
                        onDirty={() => setActivateSave(true)}
                    />
                )}

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
                        expandIcon={<ExpandMoreIcon id="voting-portal-countdown-policy" />}
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
                            label={String(
                                t("electionEventScreen.field.contestEncryptionPolicy.policyLabel")
                            )}
                            defaultValue={EElectionEventContestEncryptionPolicy.SINGLE_CONTEST}
                            emptyText={undefined}
                            validate={required()}
                        />
                        <SelectInput
                            source={"presentation.locked_down"}
                            choices={lockdownStateChoices()}
                            label={String(t("electionEventScreen.field.lockdownState.policyLabel"))}
                            defaultValue={EElectionEventLockedDown.NOT_LOCKED_DOWN}
                            emptyText={undefined}
                            validate={required()}
                        />
                        <SelectInput
                            source={"presentation.decoded_ballot_inclusion_policy"}
                            choices={decodedBallotsStateChoices()}
                            label={String(
                                t("electionEventScreen.field.decodedBallots.policyLabel")
                            )}
                            defaultValue={EElectionEventDecodedBallots.NOT_INCLUDED}
                            emptyText={undefined}
                            validate={required()}
                        />
                        <SelectInput
                            source={"presentation.ceremonies_policy"}
                            choices={ceremonyPolicyOptions()}
                            label={String(
                                t("electionEventScreen.field.ceremoniesPolicy.policyLabel")
                            )}
                            defaultValue={EElectionEventCeremoniesPolicy.MANUAL_CEREMONIES}
                            emptyText={undefined}
                            validate={required()}
                        />
                        <SelectInput
                            source={"presentation.weighted_voting_policy"}
                            choices={weightedVotingPolicyOptions()}
                            label={String(
                                t("electionEventScreen.field.weightedVotingPolicy.policyLabel")
                            )}
                            defaultValue={
                                EElectionEventWeightedVotingPolicy.DISABLED_WEIGHTED_VOTING
                            }
                            emptyText={undefined}
                            validate={required()}
                        />
                        <SelectInput
                            source={"presentation.delegated_voting_policy"}
                            choices={delegatedVotingPolicyOptions()}
                            label={String(
                                t("electionEventScreen.field.delegatedVotingPolicy.policyLabel")
                            )}
                            defaultValue={EElectionEventDelegatedVotingPolicy.DISABLED}
                            emptyText={undefined}
                            validate={required()}
                        />
                        {canConfigureResultsWebsite ? <ResultsWebsitePolicyFields /> : null}
                        <Typography
                            variant="body1"
                            component="span"
                            sx={{
                                fontWeight: "bold",
                                margin: 0,
                                display: {xs: "none", sm: "block"},
                            }}
                        >
                            {t("electionEventScreen.field.countDownPolicyOptions.sectionTitle")}
                        </Typography>
                        <SelectInput
                            source={"presentation.voting_portal_datetime_format"}
                            choices={votingPortalDateTimeFormatChoices()}
                            label={String(
                                t(
                                    "electionEventScreen.field.votingPortalDateTimeFormat.policyLabel"
                                )
                            )}
                            helperText={String(
                                t("electionEventScreen.field.votingPortalDateTimeFormat.helperText")
                            )}
                            defaultValue={EVotingPortalDateTimeFormat.LEGACY_GB_24H}
                            format={dateTimePolicyToSelectValue}
                            parse={selectValueToDateTimePolicy}
                            emptyText={undefined}
                            validate={required()}
                            slotProps={{
                                input: {error: false},
                                inputLabel: {error: false},
                                formHelperText: {error: false},
                            }}
                            sx={{marginBottom: "1.5em"}}
                        />
                        <FormDataConsumer>
                            {({formData}) =>
                                isCustomVotingPortalDateTimeFormat(
                                    formData?.presentation?.voting_portal_datetime_format
                                ) ? (
                                    <TextInput
                                        source={"presentation.voting_portal_datetime_format.custom"}
                                        label={String(
                                            t(
                                                "electionEventScreen.field.votingPortalDateTimeFormat.customFormat.label"
                                            )
                                        )}
                                        helperText={String(
                                            t(
                                                "electionEventScreen.field.votingPortalDateTimeFormat.customFormat.helperText"
                                            )
                                        )}
                                        sx={{marginBottom: "1.5em"}}
                                    />
                                ) : null
                            }
                        </FormDataConsumer>
                        <CustomDateTimeFormatInvalidNotifier
                            checkRef={checkCustomDateTimeFormatRef}
                        />
                        <SelectInput
                            source={`presentation.voting_portal_countdown_policy.policy`}
                            choices={votingPortalCountDownPolicies()}
                            label={String(
                                t("electionEventScreen.field.countDownPolicyOptions.policyLabel")
                            )}
                            defaultValue={EVotingPortalCountdownPolicy.NO_COUNTDOWN}
                            emptyText={undefined}
                            validate={required()}
                        />
                        <SelectInput
                            source={"presentation.voter_signing_policy"}
                            choices={voterSigningPolicyChoices()}
                            label={String(
                                t("electionEventScreen.field.voterSigningPolicy.policyLabel")
                            )}
                            defaultValue={EVoterSigningPolicy.NO_SIGNATURE}
                            emptyText={undefined}
                            validate={required()}
                        />
                        <SelectInput
                            source={"presentation.voter_certificate_policy"}
                            choices={VoterCertificatePolicyChoices()}
                            label={String(
                                t("electionEventScreen.field.VoterCertificatePolicy.policyLabel")
                            )}
                            defaultValue={EVoterCertificatePolicy.DISABLED}
                            emptyText={undefined}
                            validate={required()}
                            onChange={(e) =>
                                setRealmAttributeDraftValue(
                                    REALM_ATTR_VOTER_CERTIFICATE_POLICY,
                                    e.target.value as EVoterCertificatePolicy
                                )
                            }
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
                                label={String(
                                    t(
                                        "electionEventScreen.field.countDownPolicyOptions.coundownSecondsLabel"
                                    )
                                )}
                                defaultValue={defaultSecondsForCountdown}
                                sourceToWatch="presentation.voting_portal_countdown_policy.policy"
                                isDisabled={(selectedPolicy) =>
                                    selectedPolicy === EVotingPortalCountdownPolicy.NO_COUNTDOWN
                                }
                            />

                            <ManagedNumberInput
                                source={
                                    "presentation.voting_portal_countdown_policy.countdown_alert_anticipation_secs"
                                }
                                label={String(
                                    t(
                                        "electionEventScreen.field.countDownPolicyOptions.alertSecondsLabel"
                                    )
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
                                    updateCustomFilters(parsedValue, data as UpdateFunctionProps)
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
                                label={String(
                                    t(`electionEventScreen.field.enrollment.policyLabel`)
                                )}
                                source="presentation.enrollment"
                                choices={enrollmentChoices()}
                                onChange={(value) => handleEnrollmentChange(value)}
                            />
                            <SelectInput
                                label={String(t(`electionEventScreen.field.otp.policyLabel`))}
                                source="presentation.otp"
                                choices={otpChoices()}
                                onChange={(value) => handleOtpChange(value)}
                            />
                        </Box>
                    </AccordionDetails>
                </Accordion>
            </SimpleForm>

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
                electionEventId={record?.id}
                openExport={openExport}
                setOpenExport={setOpenExport}
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
