// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {
    SaveButton,
    SimpleForm,
    useListContext,
    useNotify,
    Toolbar,
    DateTimeInput,
    Identifier,
    useGetList,
} from "react-admin"
import {
    AccordionDetails,
    AccordionSummary,
    MenuItem,
    FormControlLabel,
    Switch,
    Typography,
} from "@mui/material"
import {useMutation} from "@apollo/client"
import {SubmitHandler} from "react-hook-form"
import MailIcon from "@mui/icons-material/Mail"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import EmailEditor from "@/components/EmailEditor"
import {useTranslation} from "react-i18next"
import {FormStyles} from "@/components/styles/FormStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {CREATE_SCHEDULED_EVENT} from "@/queries/CreateScheduledEvent"
import {CreateScheduledEventMutation, Sequent_Backend_Communication_Template} from "@/gql/graphql"
import {ScheduledEventType} from "@/services/ScheduledEvent"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {
    ICommunicationMethod,
    ICommunicationType,
    IEmail,
    ISendCommunicationBody,
} from "@/types/communications"
import {useLocation} from "react-router"

export enum AudienceSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED",
}

interface ICommunicationPayload {
    audience_selection: AudienceSelection
    audience_voter_ids?: Array<Identifier>
    communication_type: ICommunicationType
    communication_method: ICommunicationMethod
    schedule_now: boolean
    schedule_date?: Date
    email?: {
        subject: string
        plaintext_body: string
        html_body: string
    }
    sms?: {
        message: string
    }
}

interface ICommunication {
    audience: {
        selection: AudienceSelection
        voter_ids?: Array<Identifier> | undefined
    }
    communication_type: ICommunicationType
    communication_method: ICommunicationMethod
    alias?: string
    schedule: {
        now: boolean
        date?: Date
    }
    i18n: {
        [lang_code: string]: {
            email?: {
                subject: string
                plaintext_body: string
                html_body: string
            }
            sms?: {
                message: string
            }
        }
    }
    language_conf: {
        enabled_languages: Array<string>
        default_language_code: string
    }
}

interface SendCommunicationProps {
    ids?: Array<Identifier>
    audienceSelection?: AudienceSelection
    electionEventId?: string
    close?: () => void
}

export const SendCommunication: React.FC<SendCommunicationProps> = ({
    ids,
    audienceSelection,
    close,
    electionEventId,
}) => {
    const {isLoading} = useListContext()
    const {globalSettings} = useContext(SettingsContext)
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const location = useLocation()
    const notify = useNotify()
    const [errors, setErrors] = useState<String | null>(null)
    const [createScheduledEvent] = useMutation<CreateScheduledEventMutation>(CREATE_SCHEDULED_EVENT)
    const [showProgress, setShowProgress] = useState(false)

    const [communication, setCommunication] = useState<ICommunication>({
        audience: {
            selection: audienceSelection ?? AudienceSelection.SELECTED,
            voter_ids: ids ?? undefined,
        },
        communication_type: ICommunicationType.BALLOT_RECEIPT,
        communication_method: ICommunicationMethod.EMAIL,
        schedule: {
            now: true,
            date: undefined,
        },
        i18n: {
            en: {
                email: {
                    subject: globalSettings.DEFAULT_EMAIL_SUBJECT["en"] ?? "",
                    plaintext_body: globalSettings.DEFAULT_EMAIL_PLAINTEXT_BODY["en"] ?? "",
                    html_body: globalSettings.DEFAULT_EMAIL_HTML_BODY["en"] ?? "",
                },
                sms: {
                    message: globalSettings.DEFAULT_SMS_MESSAGE["en"] ?? "",
                },
            },
        },
        language_conf: {
            enabled_languages: ["en"],
            default_language_code: "en",
        },
    })

    const getPayload: (formData: ICommunication) => ICommunicationPayload = (
        formData: ICommunication
    ) => {
        return {
            audience_selection: formData.audience.selection,
            audience_voter_ids: formData.audience.voter_ids,
            communication_type: formData.communication_type,
            communication_method: formData.communication_method,
            schedule_now: formData.schedule.now,
            schedule_date: formData.schedule.date,
            email: formData.i18n["en"].email,
            sms: formData.i18n["en"].sms,
        }
    }

    const onSubmit: SubmitHandler<any> = async (formData: ICommunication) => {
        setErrors(null)
        setShowProgress(true)
        try {
            const {errors} = await createScheduledEvent({
                variables: {
                    tenantId: tenantId,
                    electionEventId: electionEventId,
                    eventProcessor: ScheduledEventType.SEND_COMMUNICATION,
                    cronConfig: undefined,
                    eventPayload: getPayload(formData),
                },
            })
            setShowProgress(false)
            if (errors) {
                setErrors(t("sendCommunication.errorSending", {error: errors.toString()}))
                return
            } else {
                notify(t("sendCommunication.successSending"), {type: "success"})
                close?.()
            }
        } catch (error: any) {
            setShowProgress(false)
            setErrors(t("sendCommunication.errorSending", {error: error.toString()}))
        }
    }

    const handleNowChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {checked} = e.target
        var newCommunication = {...communication}
        newCommunication.schedule.now = checked
        setCommunication(newCommunication)
    }
    const handleSmsChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {value} = e.target
        var newCommunication = {...communication}
        newCommunication.i18n["en"].sms = {message: value}
        setCommunication(newCommunication)
    }

    const handleSelectChange = async (e: any) => {
        const {value} = e.target
        var newCommunication = {...communication}
        newCommunication.audience.selection = value
        setCommunication(newCommunication)
    }

    const handleSelectMethodChange = async (e: any) => {
        const {value} = e.target
        var newCommunication = {...communication}
        newCommunication.communication_method = value
        setCommunication(newCommunication)

        setSelectedMethod(value)

        // filter receipts by communication method
        const selectedReceipts = receipts
            ?.filter(
                (receipt) =>
                    receipt.communication_method === value &&
                    receipt.communication_type === selectedType
            )
            .map((receipt) => receipt.template)

        setSelectedList(selectedReceipts ?? null)
    }

    const handleSelectTypeChange = async (e: any) => {
        const {value} = e.target
        var newCommunication = {...communication}
        newCommunication.communication_type = value
        setCommunication(newCommunication)

        setSelectedType(value)

        // filter receipts by communication method
        const selectedReceipts = receipts
            ?.filter(
                (receipt) =>
                    receipt.communication_type === value &&
                    receipt.communication_method === selectedMethod
            )
            .map((receipt) => receipt.template)

        setSelectedList(selectedReceipts ?? null)
    }

    const handleSelectAliasChange = async (e: any) => {
        console.log("handleSelectAliasChange", e.target.value)

        const {value} = e.target
        var newCommunication = {...communication}
        newCommunication.alias = value
        console.log("handleSelectAliasChange newCommunication", newCommunication)
        setCommunication(newCommunication)

        const selectedReceipt = receipts?.filter(
            (receipt) =>
                receipt.communication_type === selectedType &&
                receipt.communication_method === selectedMethod &&
                receipt.template.alias === value
        )

        if (selectedReceipt && selectedReceipt.length > 0) {
            console.log(
                "selectedReceipt",
                selectedReceipt[0]["template"][selectedMethod.toLowerCase()]
            )
            if (selectedMethod === ICommunicationMethod.EMAIL) {
                let newEmail = selectedReceipt[0]["template"][
                    selectedMethod.toLowerCase()
                ] as IEmail
                setEmail(newEmail, value)
            } else {
                let newSms = selectedReceipt[0]["template"][selectedMethod.toLowerCase()]
                    .message as string
                let newSMSCommunication = {...newCommunication}
                let a = newSMSCommunication.i18n?.["en"]
                if (a?.sms?.message) {
                    a.sms.message = newSms
                }
                setCommunication(newSMSCommunication)
            }
        }
    }

    useEffect(() => {
        console.log("handleSelectAliasChange communication", communication)
    }, [communication])

    const setEmail = async (newEmail: any, alias = "") => {
        var newCommunication = {...communication, alias}
        newCommunication.i18n["en"].email = newEmail
        setCommunication(newCommunication)
    }

    const handleLangChange = (lang: string) => async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {checked} = e.target
        var newCommunication = {...communication}
        if (checked) {
            // Add the language if it's not already in the array
            if (!newCommunication.language_conf.enabled_languages.includes(lang)) {
                newCommunication.language_conf.enabled_languages.push(lang)
            }
        } else {
            // Remove the language if it's in the array
            newCommunication.language_conf.enabled_languages =
                newCommunication.language_conf.enabled_languages.filter((l) => l !== lang)
        }
        setCommunication(newCommunication)
    }

    const validateDate = (value: any) => {
        if (!communication.schedule.now && !value) {
            return t("sendCommunication.chooseDate")
        }
    }

    // communication templates

    const [selectedMethod, setSelectedMethod] = useState<ICommunicationMethod>(
        ICommunicationMethod.EMAIL
    )
    const [selectedType, setSelectedType] = useState<ICommunicationType>(
        ICommunicationType.BALLOT_RECEIPT
    )
    const [selectedList, setSelectedList] = useState<ISendCommunicationBody[] | null>(null)
    /*const [selectedReceipt, setSelectedReceipt] = useState<IEmail | string>({
        subject: "",
        plaintext_body: "",
        html_body: "",
    })*/

    const {data: receipts} = useGetList<Sequent_Backend_Communication_Template>(
        "sequent_backend_communication_template",
        {
            filter: {
                tenant_id: tenantId,
            },
        }
    )

    useEffect(() => {
        // filter receipts by communication method and sert email by default
        const selectedReceipts = receipts
            ?.filter(
                (receipt) =>
                    receipt.communication_type === selectedType &&
                    receipt.communication_method === selectedMethod
            )
            .map((receipt) => receipt.template)

        setSelectedList(selectedReceipts ?? null)
    }, [selectedMethod, selectedType, receipts])

    //const possibleLanguages = ["en", "es"]
    //const renderLangs = () => {
    //    let langNodes = []
    //    for (let lang of possibleLanguages) {
    //        let checked = communication.language_conf.enabled_languages.includes(lang)
    //        langNodes.push(
    //            <FormControlLabel
    //                key={lang}
    //                sx={{width: "100%"}}
    //                label={t(`common.language.${lang}`)}
    //                control={<Switch checked={checked} onChange={handleLangChange(lang)} />}
    //            />
    //        )
    //    }
    //    return <div>{langNodes}</div>
    //}

    if (isLoading) {
        return <></>
    }

    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={
                    <Toolbar>
                        <SaveButton
                            icon={<MailIcon />}
                            label={t("sendCommunication.sendButton")}
                            alwaysEnable
                        />
                    </Toolbar>
                }
                record={communication}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <PageHeaderStyles.Title>{t(`sendCommunication.title`)}</PageHeaderStyles.Title>
                <PageHeaderStyles.SubTitle>
                    {t(`sendCommunication.subtitle`)}
                </PageHeaderStyles.SubTitle>

                {/* Voters */}
                <FormStyles.AccordionExpanded expanded={true} disableGutters>
                    <AccordionSummary
                        expandIcon={<ExpandMoreIcon id="send-communication-voters" />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("sendCommunication.voters")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <FormStyles.Select
                            name="audience.selection"
                            value={communication.audience.selection}
                            onChange={handleSelectChange}
                        >
                            {(Object.keys(AudienceSelection) as Array<AudienceSelection>).map(
                                (key) => (
                                    <MenuItem key={key} value={key}>
                                        {t(`sendCommunication.votersSelection.${key}`, {
                                            total: communication.audience.voter_ids?.length ?? 0,
                                            voters: location.pathname.includes("user")
                                                ? t("sendCommunication.path.users")
                                                : t("sendCommunication.path.voters"),
                                        })}
                                    </MenuItem>
                                )
                            )}
                        </FormStyles.Select>
                    </AccordionDetails>
                </FormStyles.AccordionExpanded>

                {/* Schedule */}
                <FormStyles.AccordionExpanded expanded={true} disableGutters>
                    <AccordionSummary
                        expandIcon={<ExpandMoreIcon id="send-communication-schedule" />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("sendCommunication.schedule")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <FormControlLabel
                            key="nowInput"
                            label={t("sendCommunication.nowInput")}
                            control={
                                <Switch
                                    checked={communication.schedule.now}
                                    onChange={handleNowChange}
                                />
                            }
                        />
                        <DateTimeInput
                            validate={validateDate}
                            disabled={communication.schedule.now}
                            source="schedule.date"
                            label={t("sendCommunication.dateInput")}
                            parse={(value) => new Date(value).toISOString()}
                        />
                    </AccordionDetails>
                </FormStyles.AccordionExpanded>

                {/* Languages 
                <FormStyles.AccordionExpanded expanded={true} disableGutters>
                    <AccordionSummary
                        expandIcon={<ExpandMoreIcon id="send-communication-languages" />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("sendCommunication.languages")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <Grid container spacing={4}>
                            <Grid item xs={12} md={6}>
                                {renderLangs()}
                            </Grid>
                        </Grid>
                    </AccordionDetails>
                </FormStyles.AccordionExpanded>*/}

                {/* Communication Method */}
                <FormStyles.AccordionExpanded expanded={true} disableGutters>
                    <AccordionSummary
                        expandIcon={<ExpandMoreIcon id="send-communication-method" />}
                    >
                        <ElectionHeaderStyles.Wrapper>
                            <ElectionHeaderStyles.Title>
                                {t("sendCommunication.methodTitle")}
                            </ElectionHeaderStyles.Title>
                        </ElectionHeaderStyles.Wrapper>
                    </AccordionSummary>
                    <AccordionDetails>
                        <Typography variant="body2" sx={{margin: "0"}}>
                            {t("sendCommunication.method")}
                        </Typography>{" "}
                        <FormStyles.Select
                            name="voters.selection"
                            value={communication.communication_method}
                            onChange={handleSelectMethodChange}
                        >
                            {Object.values(ICommunicationMethod)
                                .filter((method) => method !== ICommunicationMethod.DOCUMENT)
                                .map((key) => (
                                    <MenuItem key={key} value={key}>
                                        {t(`sendCommunication.communicationMethod.${key}`)}
                                    </MenuItem>
                                ))}
                        </FormStyles.Select>
                        <Typography variant="body2" sx={{margin: "0"}}>
                            {t("sendCommunication.type")}
                        </Typography>{" "}
                        <FormStyles.Select
                            name="voters.selection"
                            value={communication.communication_type}
                            onChange={handleSelectTypeChange}
                        >
                            {Object.values(ICommunicationType)
                                .filter((type) => type !== ICommunicationType.MANUALLY_VERIFY_VOTER)
                                .map((key) => (
                                <MenuItem key={key} value={key}>
                                    {t(`sendCommunication.communicationType.${key}`)}
                                </MenuItem>
                            ))}
                        </FormStyles.Select>
                        <Typography variant="body2" sx={{margin: "0"}}>
                            {t("sendCommunication.alias")}
                        </Typography>
                        <FormStyles.Select
                            name="alias"
                            value={communication.alias || ""}
                            onChange={handleSelectAliasChange}
                        >
                            {selectedList
                                ? selectedList?.map(
                                      (key: ISendCommunicationBody, index: number) => (
                                          <MenuItem key={index} value={key.alias}>
                                              {key.alias}
                                          </MenuItem>
                                      )
                                  )
                                : null}
                        </FormStyles.Select>
                        {communication.communication_method === ICommunicationMethod.EMAIL &&
                            communication.i18n["en"].email && (
                                <EmailEditor
                                    record={communication.i18n["en"].email}
                                    setRecord={setEmail}
                                />
                            )}
                        {communication.communication_method === ICommunicationMethod.SMS && (
                            <FormStyles.TextField
                                name="sms"
                                label={t("sendCommunication.smsMessage")}
                                value={communication.i18n["en"].sms?.message ?? ""}
                                onChange={handleSmsChange}
                                multiline={true}
                                minRows={4}
                            />
                        )}
                    </AccordionDetails>
                </FormStyles.AccordionExpanded>
                <FormStyles.StatusBox>
                    {showProgress ? <FormStyles.ShowProgress /> : null}
                    {errors ? (
                        <FormStyles.ErrorMessage variant="body2">{errors}</FormStyles.ErrorMessage>
                    ) : null}
                </FormStyles.StatusBox>
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}
