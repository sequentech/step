// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    BooleanInput,
    DateTimeInput,
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
    Sequent_Backend_Communication_Template,
    Sequent_Backend_Contest,
    Sequent_Backend_Document,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tenant,
} from "../../gql/graphql"

import React, {useCallback, useContext, useEffect, useState} from "react"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CustomTabPanel} from "../../components/CustomTabPanel"
import {ElectionStyles} from "../../components/styles/ElectionStyles"
import {
    ContestsOrder,
    EVotingPortalAuditButtonCfg,
    IContestPresentation,
    IElectionDates,
    IElectionEventPresentation,
    IElectionPresentation,
} from "@sequentech/ui-core"
import {DropFile} from "@sequentech/ui-essentials"
import FileJsonInput from "../../components/FileJsonInput"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ICommunicationMethod, ICommunicationType} from "@/types/communications"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import styled from "@emotion/styled"
import {MANAGE_ELECTION_DATES} from "@/queries/ManageElectionDates"
import {ManageElectionDatesMutation} from "@/gql/graphql"
import CustomOrderInput from "@/components/custom-order/CustomOrderInput"

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

const ListWrapper = styled.div`
    display: flex;
    flex-direction: column;
    border-radius: 4px;
    border: 1px solid #777;
    padding: 8px;
    margin-bottom: 4px;
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

    const [value, setValue] = useState(0)
    const [expanded, setExpanded] = useState("election-data-general")
    const [languageSettings, setLanguageSettings] = useState<Array<string>>(["en"])
    const {globalSettings} = useContext(SettingsContext)
    const [startDateValue, setStartDateValue] = useState<string | undefined>(undefined)
    const [endDateValue, setEndDateValue] = useState<string | undefined>(undefined)

    const [manageElectionDates] = useMutation<ManageElectionDatesMutation>(MANAGE_ELECTION_DATES)

    const {data} = useGetOne<Sequent_Backend_Election_Event>("sequent_backend_election_event", {
        id: record.election_event_id,
    })

    const {data: tenantData} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
        id: record.tenant_id || tenantId,
    })

    const {data: contests} = useGetList<Sequent_Backend_Contest>("sequent_backend_contest", {
        filter: {
            election_id: record.id,
            tenant_id: record.tenant_id,
            election_event_id: record.election_event_id,
        },
    })

    const {data: imageData, refetch: refetchImage} = useGetOne<Sequent_Backend_Document>(
        "sequent_backend_document",
        {
            id: record.image_document_id || record.tenant_id,
            meta: {tenant_id: record.tenant_id},
        }
    )

    const {data: receipts} = useGetList<Sequent_Backend_Communication_Template>(
        "sequent_backend_communication_template",
        {
            filter: {
                tenant_id: record.tenant_id || tenantId,
                communication_type: ICommunicationType.BALLOT_RECEIPT,
            },
        }
    )

    const [updateImage] = useUpdate()

    useEffect(() => {
        let dates = record.dates as IElectionDates | undefined
        if (dates?.start_date && !startDateValue) {
            setStartDateValue(dates.start_date)
        }
        if (dates?.end_date && !endDateValue) {
            setEndDateValue(dates.end_date)
        }
    }, [
        record.dates,
        record.dates?.start_date,
        record.dates?.end_date,
        startDateValue,
        setStartDateValue,
        endDateValue,
        setEndDateValue,
    ])

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

            if (!incoming.presentation) {
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
                for (const value in Object.values(ICommunicationMethod) as ICommunicationMethod[]) {
                    const key = Object.keys(ICommunicationMethod)[value]

                    allowed[key] = temp.receipts[key]?.allowed
                    template[key] = temp.receipts[key]?.template
                }
                temp.allowed = allowed
                temp.template = template
            }

            // defaults
            temp.num_allowed_revotes = temp.num_allowed_revotes || 1

            return temp
        },
        [data, tenantData?.voting_channels]
    )

    const formValidator = (values: any): any => {
        const errors: any = {dates: {}}
        if (values?.dates?.start_date && values?.dates?.end_date <= values?.dates?.start_date) {
            errors.dates.end_date = t("electionScreen.error.endDate")
        }
        return errors
    }

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
                        id: record.id,
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

    const communicationMethodChoices = () => {
        return (Object.values(ICommunicationMethod) as ICommunicationMethod[]).map((value) => ({
            id: value,
            name: t(`communicationTemplate.method.${value.toLowerCase()}`),
        }))
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

    const auditButtonConfigChoices = (): Array<EnumChoice<EVotingPortalAuditButtonCfg>> => {
        return Object.values(EVotingPortalAuditButtonCfg).map((value) => ({
            id: value,
            name: t(`contestScreen.auditButtonConfig.${value.toLowerCase()}`),
        }))
    }

    return data ? (
        <RecordContext.Consumer>
            {(incoming) => {
                const parsedValue = parseValues(
                    incoming as Sequent_Backend_Election_Extended,
                    languageSettings
                )

                const onSave = async () => {
                    await manageElectionDates({
                        variables: {
                            electionEventId: parsedValue.election_event_id,
                            electionId: parsedValue.id,
                            start_date: startDateValue,
                            end_date: endDateValue,
                        },
                    })
                }

                return (
                    <SimpleForm
                        defaultValues={{contestsOrder: sortedContests}}
                        record={parsedValue}
                        validate={formValidator}
                        toolbar={
                            <Toolbar>
                                <SaveButton
                                    onClick={() => {
                                        onSave()
                                    }}
                                />
                            </Toolbar>
                        }
                    >
                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-general"}
                            onChange={() => setExpanded("election-data-general")}
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
                            expanded={expanded === "election-data-dates"}
                            onChange={() => setExpanded("election-data-dates")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-dates" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.dates")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
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
                                    {t("electionScreen.edit.votingPeriod")}
                                </Typography>
                                <Grid container spacing={4}>
                                    <Grid item xs={12} md={6}>
                                        <DateTimeInput
                                            source={`dates.start_date`}
                                            label={t("electionScreen.field.startDateTime")}
                                            parse={(value) =>
                                                value && new Date(value).toISOString()
                                            }
                                            onChange={(value) => {
                                                setStartDateValue(
                                                    value.target.value !== ""
                                                        ? new Date(value.target.value).toISOString()
                                                        : undefined
                                                )
                                            }}
                                        />
                                    </Grid>
                                    <Grid item xs={12} md={6}>
                                        <DateTimeInput
                                            source="dates.end_date"
                                            label={t("electionScreen.field.endDateTime")}
                                            parse={(value) =>
                                                value && new Date(value).toISOString()
                                            }
                                            onChange={(value) => {
                                                setEndDateValue(
                                                    value.target.value !== ""
                                                        ? new Date(value.target.value).toISOString()
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
                            expanded={expanded === "election-data-language"}
                            onChange={() => setExpanded("election-data-language")}
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
                            onChange={() => setExpanded("election-data-allowed")}
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
                                    <Grid item xs={12} md={6}>
                                        {renderVotingChannels(parsedValue)}
                                    </Grid>
                                </Grid>
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "contest-data-design"}
                            onChange={() => setExpanded("contest-data-design")}
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
                            expanded={expanded === "election-data-receipts"}
                            onChange={() => setExpanded("election-data-receipts")}
                        >
                            <AccordionSummary
                                expandIcon={<ExpandMoreIcon id="election-data-receipts" />}
                            >
                                <ElectionStyles.Wrapper>
                                    <ElectionStyles.Title>
                                        {t("electionScreen.edit.receipts")}
                                    </ElectionStyles.Title>
                                </ElectionStyles.Wrapper>
                            </AccordionSummary>
                            <AccordionDetails>
                                <ElectionStyles.AccordionContainer>
                                    {communicationMethodChoices().map((choice) => (
                                        <ElectionStyles.AccordionWrapper
                                            alignment="center"
                                            key={choice.id}
                                        >
                                            <BooleanInput
                                                source={`allowed.${choice.id}`}
                                                label={choice.name}
                                                defaultValue={true}
                                            />
                                            <SelectInput
                                                source={`template.${choice.id}`}
                                                label={choice.name}
                                                choices={
                                                    receipts
                                                        ?.filter(
                                                            (item) =>
                                                                item.communication_method ===
                                                                choice.id
                                                        )
                                                        .map((type) => ({
                                                            id: type.id,
                                                            name: type.template.alias,
                                                        })) ?? []
                                                }
                                            />
                                        </ElectionStyles.AccordionWrapper>
                                    ))}
                                </ElectionStyles.AccordionContainer>
                            </AccordionDetails>
                        </Accordion>

                        <Accordion
                            sx={{width: "100%"}}
                            expanded={expanded === "election-data-image"}
                            onChange={() => setExpanded("election-data-image")}
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
                                    <Grid item xs={2}>
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
                                    <Grid item xs={10}>
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
                            onChange={() => setExpanded("election-data-advanced")}
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
                                <FileJsonInput
                                    parsedValue={parsedValue}
                                    fileSource="configuration"
                                    jsonSource="presentation"
                                />
                            </AccordionDetails>
                        </Accordion>
                    </SimpleForm>
                )
            }}
        </RecordContext.Consumer>
    ) : null
}
