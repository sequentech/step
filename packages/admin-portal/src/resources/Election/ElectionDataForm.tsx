// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    BooleanInput,
    SelectInput,
    TextInput,
    useRecordContext,
    SimpleForm,
    useGetOne,
    RadioButtonGroupInput,
    Toolbar,
    SaveButton,
    useNotify,
    useRefresh,
    useUpdate,
    RaRecord,
    Identifier,
    RecordContext,
    NumberInput,
    useGetList,
    FormDataConsumer,
    required,
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
import {
    GetUploadUrlMutation,
    Sequent_Backend_Template,
    Sequent_Backend_Contest,
    Sequent_Backend_Document,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tenant,
    ManageElectionDatesMutation,
} from "../../gql/graphql"

import React, {useCallback, useContext, useEffect, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {ElectionStyles} from "../../components/styles/ElectionStyles"
import {
    ContestsOrder,
    ECastVoteGoldLevelPolicy,
    EStartScreenTitlePolicy,
    EGracePeriodPolicy,
    ESecurityConfirmationPolicy,
    EVotingPortalAuditButtonCfg,
    IContestPresentation,
    EInitializeReportPolicy,
    IElectionEventPresentation,
    IElectionPresentation,
    EAllowTally,
} from "@sequentech/ui-core"
import {DropFile} from "@sequentech/ui-essentials"
import FileJsonInput from "../../components/FileJsonInput"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ITemplateMethod} from "@/types/templates"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import styled from "@emotion/styled"
import CustomOrderInput from "@/components/custom-order/CustomOrderInput"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {ManagedSelectInput} from "@/components/managed-inputs/ManagedSelectInput"
import {ManagedNumberInput} from "@/components/managed-inputs/ManagedNumberInput"
import {MANAGE_ELECTION_DATES} from "@/queries/ManageElectionDates"
import {JsonEditor, UpdateFunction} from "json-edit-react"
import {CustomFilter} from "@/types/filters"

const LangsWrapper = styled(Box)`
    margin-top: 46px;
`

const ContestRows = styled.div`
    display: flex;
    flex-direction: column;
    width: 100%;
    cursor: pointer;
    margin-bottom: 0.1rem;
    padding: 1rem;
`

export type Sequent_Backend_Election_Extended = RaRecord<Identifier> & {
    enabled_languages?: {[key: string]: boolean}
    contestsOrder?: Array<Sequent_Backend_Contest>
} & Sequent_Backend_Election

export const ElectionDataForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const [tenantId] = useTenantStore()

    const {t} = useTranslation()
    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const notify = useNotify()
    const refresh = useRefresh()
    const authContext = useContext(AuthContext)
    const canEditPermissionLabel = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.PERMISSION_LABEL_WRITE
    )

    const canEdit = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_WRITE
    )

    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("election-data-general")
    const [languageSettings, setLanguageSettings] = useState<Array<string>>(["en"])

    const {globalSettings} = useContext(SettingsContext)
    const [customFilters, setCustomFilters] = useState<CustomFilter[] | undefined>()
    const [activateSave, setActivateSave] = useState(false)

    const {data} = useGetOne<Sequent_Backend_Election_Event>("sequent_backend_election_event", {
        id: record?.election_event_id,
    })

    const {data: tenantData} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
        id: record?.tenant_id || tenantId,
    })

    const {data: contests} = useGetList<Sequent_Backend_Contest>("sequent_backend_contest", {
        filter: {
            election_id: record?.id,
            tenant_id: record?.tenant_id,
            election_event_id: record?.election_event_id,
        },
    })

    const {data: imageData, refetch: refetchImage} = useGetOne<Sequent_Backend_Document>(
        "sequent_backend_document",
        {
            id: record?.image_document_id || record?.tenant_id,
            meta: {tenant_id: record?.tenant_id},
        },
        {
            enabled: !!record?.image_document_id || !!record?.tenant_id,
            onError: (error: any) => {
                console.log(`error fetching image doc: ${error.message}`)
            },
            onSuccess: () => {
                console.log(`success fetching image doc`)
            },
        }
    )

    const [updateImage] = useUpdate()

    useEffect(() => {
        if (!data || !record) {
            return
        }
        let eventAvailableLangs = (data?.presentation as IElectionEventPresentation | undefined)
            ?.language_conf?.enabled_language_codes ?? ["en"]
        let electionAvailableLangs =
            (record?.presentation as IElectionPresentation | undefined)?.language_conf
                ?.enabled_language_codes ?? []
        let newElectionLangs = electionAvailableLangs.filter(
            (electionLang) => !eventAvailableLangs.includes(electionLang)
        )
        let completeList = eventAvailableLangs.concat(newElectionLangs)

        setLanguageSettings(completeList)
    }, [
        data?.presentation?.language_conf?.enabled_language_codes,
        record?.presentation?.language_conf?.enabled_language_codes,
    ])

    const parseValues = useCallback(
        (
            incoming: Sequent_Backend_Election_Extended,
            languageSettings: Array<string>
        ): Sequent_Backend_Election_Extended => {
            if (!data) {
                return incoming as Sequent_Backend_Election_Extended
            }
            const temp: Sequent_Backend_Election_Extended = {
                ...incoming,
            }
            const incomingLangConf = (incoming?.presentation as IElectionPresentation | undefined)
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

            if (!incoming?.presentation) {
                temp.presentation = {}
            }

            if (temp.presentation && !temp.presentation.language_conf) {
                temp.presentation.language_conf = {}
            }

            if (temp.presentation && !temp.presentation.dates) {
                temp.presentation.dates = {}
            }

            if (temp.presentation) {
                temp.scheduledOpening = temp.presentation?.dates?.scheduled_opening
                temp.scheduledClosing = temp.presentation?.dates?.scheduled_closing
            }

            temp.presentation.contests_order =
                temp.presentation.contests_order || ContestsOrder.ALPHABETICAL

            const votingSettings = data?.voting_channels || tenantData?.voting_channels

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

            // name, alias and description fields
            if (!temp.presentation) {
                temp.presentation = {}
            }
            if (!temp.presentation?.i18n) {
                temp.presentation.i18n = {}
            }
            if (!temp.presentation?.i18n?.en) {
                temp.presentation.i18n.en = {}
            }
            temp.presentation.i18n.en.name = temp.name
            temp.presentation.i18n.en.alias = temp.alias
            temp.presentation.i18n.en.description = temp.description

            // receipts
            const template: {[key: string]: string | null} = {}
            const allowed: {[key: string]: boolean} = {}

            if (temp.receipts) {
                for (const value in Object.values(ITemplateMethod) as ITemplateMethod[]) {
                    const key = Object.keys(ITemplateMethod)[value]

                    allowed[key] = temp.receipts[key]?.allowed
                    template[key] = temp.receipts[key]?.template
                }
                temp.allowed = allowed
                temp.template = template
            }

            // defaults
            temp.presentation.initialization_report_policy =
                temp.presentation.initialization_report_policy ||
                EInitializeReportPolicy.NOT_REQUIRED
            temp.num_allowed_revotes =
                temp.num_allowed_revotes != null ? temp.num_allowed_revotes : 1
            temp.presentation.grace_period_policy =
                temp.presentation.grace_period_policy || EGracePeriodPolicy.NO_GRACE_PERIOD
            temp.presentation.grace_period_secs = temp.presentation.grace_period_secs || 0

            if (!customFilters && temp?.presentation?.custom_filters) {
                setCustomFilters(temp.presentation.custom_filters)
            }

            return temp
        },
        [data, tenantData?.voting_channels]
    )

    const handleChange = (_event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    const renderLangs = (parsedValue: Sequent_Backend_Election_Extended) => {
        return (
            <LangsWrapper>
                {languageSettings.map((lang) => (
                    <BooleanInput
                        key={lang}
                        source={`enabled_languages.${lang}`}
                        label={t(`common.language.${lang}`)}
                        helperText={false}
                    />
                ))}
            </LangsWrapper>
        )
    }

    const renderDefaultLangs = (_parsedValue: Sequent_Backend_Election_Extended) => {
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

    const renderVotingChannels = (parsedValue: Sequent_Backend_Election_Extended) => {
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

    const renderTabs = (parsedValue: Sequent_Backend_Election_Extended) => {
        let tabNodes = []
        for (const lang in parsedValue?.enabled_languages) {
            if (parsedValue?.enabled_languages?.[lang]) {
                tabNodes.push(<Tab key={lang} label={t(`common.language.${lang}`)} id={lang}></Tab>)
            }
        }

        // reset actived tab to first tab if only one
        if (tabNodes.length === 1) {
            setValue(0)
        }

        return tabNodes
    }

    // TODO: renderReceipts

    const renderTabContent = (parsedValue: Sequent_Backend_Election_Extended) => {
        let tabNodes = []
        let index = 0
        let hasTos =
            ESecurityConfirmationPolicy.MANDATORY ===
            (parsedValue.presentation as IElectionPresentation | undefined)
                ?.security_confirmation_policy
        for (const lang in parsedValue?.enabled_languages) {
            if (parsedValue?.enabled_languages?.[lang]) {
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
                            {hasTos ? (
                                <TextInput
                                    source={`presentation.i18n[${lang}].security_confirmation_html`}
                                    label={t("electionScreen.field.securityConfirmationHtml")}
                                />
                            ) : null}
                        </div>
                    </CustomTabPanel>
                )
                index++
            }
        }
        return tabNodes
    }

    const handleFiles = async (files: FileList | null) => {
        // https://fullstackdojo.medium.com/s3-upload-with-presigned-url-react-and-nodejs-b77f348d54cc

        const theFile = files?.[0]

        if (theFile) {
            let {data} = await getUploadUrl({
                variables: {
                    name: theFile.name,
                    media_type: theFile.type,
                    size: theFile.size,
                },
            })
            if (data?.get_upload_url?.document_id) {
                try {
                    await fetch(data.get_upload_url.url, {
                        method: "PUT",
                        headers: {
                            "Content-Type": "image/*",
                        },
                        body: theFile,
                    })
                    notify(t("electionScreen.error.fileLoaded"), {type: "success"})

                    updateImage("sequent_backend_election", {
                        id: record?.id,
                        data: {
                            image_document_id: data.get_upload_url.document_id,
                        },
                    })

                    refetchImage()
                    refresh()
                } catch (e) {
                    notify(t("electionScreen.error.fileError"), {type: "error"})
                }
            } else {
                notify(t("electionScreen.error.fileError"), {type: "error"})
            }
        }
    }

    const gracePeriodPolicyChoices = () => {
        return (Object.values(EGracePeriodPolicy) as EGracePeriodPolicy[]).map((value) => ({
            id: value,
            name: t(`electionScreen.gracePeriodPolicy.${value.toLowerCase()}`),
        }))
    }

    const allowTallyChoices = () => {
        return (Object.values(EAllowTally) as EAllowTally[]).map((value) => ({
            id: value,
            name: t(`electionScreen.allowTallyPolicy.${value.toLowerCase()}`),
        }))
    }

    const securityConfirmationPolicyChoices = () => {
        return (Object.values(ESecurityConfirmationPolicy) as ESecurityConfirmationPolicy[]).map(
            (value) => ({
                id: value,
                name: t(`electionScreen.securityConfirmationPolicy.${value.toLowerCase()}`),
            })
        )
    }

    const sortedContests = (contests ?? []).sort((a, b) => {
        let presentationA = a.presentation as IContestPresentation | undefined
        let presentationB = b.presentation as IContestPresentation | undefined
        let sortOrderA = presentationA?.sort_order ?? -1
        let sortOrderB = presentationB?.sort_order ?? -1
        return sortOrderA - sortOrderB
    })

    interface EnumChoice<T> {
        id: T
        name: string
    }

    const orderAnswerChoices = (): Array<EnumChoice<ContestsOrder>> => {
        return Object.values(ContestsOrder).map((value) => ({
            id: value,
            name: t(`contestScreen.options.${value.toLowerCase()}`),
        }))
    }

    const startScreenTitleChoices = (): Array<EnumChoice<EStartScreenTitlePolicy>> => {
        return Object.values(EStartScreenTitlePolicy).map((value) => ({
            id: value,
            name: t(`electionScreen.startScreenTitlePolicy.options.${value.toLowerCase()}`),
        }))
    }

    const goldLevelChoices = (): Array<EnumChoice<ECastVoteGoldLevelPolicy>> => {
        return Object.values(ECastVoteGoldLevelPolicy).map((value) => ({
            id: value,
            name: t(`electionScreen.castVoteGoldLevelPolicy.options.${value.toLowerCase()}`),
        }))
    }

    const auditButtonConfigChoices = (): Array<EnumChoice<EVotingPortalAuditButtonCfg>> => {
        return Object.values(EVotingPortalAuditButtonCfg).map((value) => ({
            id: value,
            name: t(`contestScreen.auditButtonConfig.${value.toLowerCase()}`),
        }))
    }
    type UpdateFunctionProps = Parameters<UpdateFunction>[0]

    const initializationReportChoices = (): Array<EnumChoice<EInitializeReportPolicy>> => {
        return Object.values(EInitializeReportPolicy).map((value) => ({
            id: value,
            name: t(`electionScreen.initializeReportPolicy.${value.toLowerCase()}`),
        }))
    }

    const updateCustomFilters = (
        values: Sequent_Backend_Election_Extended,
        {newData}: UpdateFunctionProps
    ) => {
        values.presentation.custom_filters = newData
        setCustomFilters(newData as CustomFilter[])
        setActivateSave(true)
    }
    return record && data ? (
        <RecordContext.Consumer>
            {(incoming) => {
                const parsedValue = parseValues(
                    incoming as Sequent_Backend_Election_Extended,
                    languageSettings
                )

                const onSave = async () => {}

                return (
                    <SimpleForm
                        defaultValues={{contestsOrder: sortedContests}}
                        record={parsedValue}
                        toolbar={
                            <Toolbar>
                                {canEdit && (
                                    <SaveButton
                                        onClick={() => {
                                            onSave()
                                        }}
                                        type="button"
                                    />
                                )}
                            </Toolbar>
                        }
                    >
                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-general"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "election-data-general" ? "" : "election-data-general"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-general" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.general")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
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
                            expanded={expanded === "election-data-language"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "election-data-language"
                                        ? ""
                                        : "election-data-language"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-language" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.language")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
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
                            expanded={expanded === "election-data-allowed"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "election-data-allowed" ? "" : "election-data-allowed"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-allowed" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.allowed")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
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
                            expanded={expanded === "contest-data-design"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "contest-data-design" ? "" : "contest-data-design"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="contest-data-design" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("contestScreen.edit.design")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <SelectInput
                                    source={`presentation.audit_button_cfg`}
                                    choices={auditButtonConfigChoices()}
                                    label={t(`contestScreen.auditButtonConfig.label`)}
                                    defaultValue={EVotingPortalAuditButtonCfg.SHOW}
                                    validate={required()}
                                />
                                <SelectInput
                                    source="presentation.contests_order"
                                    choices={orderAnswerChoices()}
                                    validate={required()}
                                />
                                <FormDataConsumer>
                                    {({formData, ...rest}) => {
                                        return (
                                            formData?.presentation as
                                                | IElectionPresentation
                                                | undefined
                                        )?.contests_order === ContestsOrder.CUSTOM ? (
                                            <ContestRows>
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
                                                    {t("electionScreen.edit.reorder")}
                                                </Typography>
                                                <CustomOrderInput source="contestsOrder" />
                                                <Box sx={{width: "100%", height: "180px"}}></Box>
                                            </ContestRows>
                                        ) : null
                                    }}
                                </FormDataConsumer>
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-image"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "election-data-image" ? "" : "election-data-image"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-image" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.image")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <Grid container spacing={1}>
                                    <Grid size={2}>
                                        {parsedValue?.image_document_id &&
                                        parsedValue?.image_document_id !== "" ? (
                                            <img
                                                width={200}
                                                height={200}
                                                src={`${globalSettings.PUBLIC_BUCKET_URL}tenant-${parsedValue?.tenant_id}/document-${parsedValue?.image_document_id}/${imageData?.name}`}
                                                alt={`tenant-${parsedValue?.tenant_id}/document-${parsedValue?.image_document_id}/${imageData?.name}`}
                                            />
                                        ) : null}
                                    </Grid>
                                    <Grid size={10}>
                                        <DropFile
                                            handleFiles={async (files) => handleFiles(files)}
                                        />
                                    </Grid>
                                </Grid>
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-advanced"}
                            onChange={() =>
                                setExpanded((prev) =>
                                    prev === "election-data-advanced"
                                        ? ""
                                        : "election-data-advanced"
                                )
                            }
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-advanced" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.advanced")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <BooleanInput
                                    source={"presentation.cast_vote_confirm"}
                                    label={t(`electionScreen.edit.castVoteConfirm`)}
                                />
                                <NumberInput
                                    source="num_allowed_revotes"
                                    label={t("electionScreen.edit.numAllowedVotes")}
                                    min={0}
                                />
                                <SelectInput
                                    label={t("electionScreen.castVoteGoldLevelPolicy.label")}
                                    source="presentation.cast_vote_gold_level"
                                    choices={goldLevelChoices()}
                                    defaultValue={ECastVoteGoldLevelPolicy.NO_GOLD_LEVEL}
                                    validate={required()}
                                />
                                <SelectInput
                                    label={t("electionScreen.startScreenTitlePolicy.label")}
                                    source="presentation.start_screen_title_policy"
                                    choices={startScreenTitleChoices()}
                                    defaultValue={EStartScreenTitlePolicy.ELECTION}
                                    validate={required()}
                                />
                                {canEditPermissionLabel && (
                                    <TextInput
                                        label={t("electionScreen.edit.permissionLabel")}
                                        source="permission_label"
                                    />
                                )}
                                <FileJsonInput
                                    parsedValue={parsedValue}
                                    fileSource="configuration"
                                    jsonSource="presentation"
                                />
                                <SelectInput
                                    source={`presentation.initialization_report_policy`}
                                    choices={initializationReportChoices()}
                                    label={t("electionScreen.initializeReportPolicy.label")}
                                    validate={required()}
                                />
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
                                        {t("electionScreen.edit.custom_filters")}
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
                                <ManagedSelectInput
                                    source={`presentation.grace_period_policy`}
                                    choices={gracePeriodPolicyChoices()}
                                    label={t(`electionScreen.gracePeriodPolicy.label`)}
                                    defaultValue={EGracePeriodPolicy.NO_GRACE_PERIOD}
                                />
                                <ManagedNumberInput
                                    source={"presentation.grace_period_secs"}
                                    label={t("electionScreen.gracePeriodPolicy.gracePeriodSecs")}
                                    defaultValue={0}
                                    sourceToWatch="presentation.grace_period_policy"
                                    isDisabled={(selectedPolicy: any) =>
                                        selectedPolicy === EGracePeriodPolicy.NO_GRACE_PERIOD
                                    }
                                />
                                <ManagedSelectInput
                                    source={`status.allow_tally`}
                                    choices={allowTallyChoices()}
                                    label={t(`electionScreen.edit.allowTallyPolicy`)}
                                    defaultValue={EAllowTally.ALLOWED}
                                />

                                <ManagedSelectInput
                                    source={`presentation.security_confirmation_policy`}
                                    choices={securityConfirmationPolicyChoices()}
                                    label={t(`electionScreen.securityConfirmationPolicy.label`)}
                                    defaultValue={ESecurityConfirmationPolicy.NONE}
                                />
                            </AccordionDetails>
                        </Accordion>
                    </SimpleForm>
                )
            }}
        </RecordContext.Consumer>
    ) : null
}
