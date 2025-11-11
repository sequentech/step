// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
import {CreateScheduledEventMutation, Sequent_Backend_Template} from "@/gql/graphql"
import {ScheduledEventType} from "@/services/ScheduledEvent"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ITemplateMethod, IEmail, ISendTemplateBody} from "@/types/templates"
import {useLocation} from "react-router"

export enum AudienceSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED",
}

interface ITemplatePayload {
    audience_selection: AudienceSelection
    audience_voter_ids?: Array<Identifier>
    communication_method: ITemplateMethod
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

interface ITemplate {
    audience: {
        selection: AudienceSelection
        voter_ids?: Array<Identifier> | undefined
    }
    communication_method: ITemplateMethod
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

interface SendTemplateProps {
    ids?: Array<Identifier>
    audienceSelection?: AudienceSelection
    electionEventId?: string
    close?: () => void
}

export const SendTemplate: React.FC<SendTemplateProps> = ({
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

    const [template, setTemplate] = useState<ITemplate>({
        audience: {
            selection: audienceSelection ?? AudienceSelection.SELECTED,
            voter_ids: ids ?? undefined,
        },
        communication_method: ITemplateMethod.EMAIL,
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

    const getPayload: (formData: ITemplate) => ITemplatePayload = (formData: ITemplate) => {
        return {
            audience_selection: formData.audience.selection,
            audience_voter_ids: formData.audience.voter_ids,
            communication_method: formData.communication_method,
            schedule_now: formData.schedule.now,
            schedule_date: formData.schedule.date,
            email: formData.i18n["en"].email,
            sms: formData.i18n["en"].sms,
        }
    }

    const onSubmit: SubmitHandler<any> = async (formData: ITemplate) => {
        setErrors(null)
        setShowProgress(true)
        try {
            const {errors} = await createScheduledEvent({
                variables: {
                    tenantId: tenantId,
                    electionEventId: electionEventId,
                    eventProcessor: ScheduledEventType.SEND_TEMPLATE,
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
        var newTemplate = {...template}
        newTemplate.schedule.now = checked
        setTemplate(newTemplate)
    }
    const handleSmsChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {value} = e.target
        var newTemplate = {...template}
        newTemplate.i18n["en"].sms = {message: value}
        setTemplate(newTemplate)
    }

    const handleSelectChange = async (e: any) => {
        const {value} = e.target
        var newTemplate = {...template}
        newTemplate.audience.selection = value
        setTemplate(newTemplate)
    }

    const handleSelectMethodChange = async (e: any) => {
        const {value} = e.target
        var newTemplate = {...template}
        newTemplate.communication_method = value
        setTemplate(newTemplate)

        setSelectedMethod(value)

        // filter receipts by communication method
        const selectedReceipts = receipts
            ?.filter((receipt) => receipt.communication_method === value)
            .map((receipt) => receipt.template)

        setSelectedList(selectedReceipts ?? null)
    }

    const handleSelectAliasChange = async (e: any) => {
        console.log("handleSelectAliasChange", e.target.value)

        const {value} = e.target
        var newTemplate = {...template}
        newTemplate.alias = value
        console.log("handleSelectAliasChange newTemplate", newTemplate)
        setTemplate(newTemplate)

        const selectedReceipt = receipts?.filter(
            (receipt) =>
                receipt.communication_method === selectedMethod &&
                receipt.template.alias === value
        )

        if (selectedReceipt && selectedReceipt.length > 0) {
            console.log(
                "selectedReceipt",
                selectedReceipt[0]["template"][selectedMethod.toLowerCase()]
            )
            if (selectedMethod === ITemplateMethod.EMAIL) {
                let newEmail = selectedReceipt[0]["template"][
                    selectedMethod.toLowerCase()
                ] as IEmail
                setEmail(newEmail, value)
            } else {
                let newSms = selectedReceipt[0]["template"][selectedMethod.toLowerCase()]
                    .message as string
                let newSMSCommunication = {...newTemplate}
                let a = newSMSCommunication.i18n?.["en"]
                if (a?.sms?.message) {
                    a.sms.message = newSms
                }
                setTemplate(newSMSCommunication)
            }
        }
    }

    useEffect(() => {
        console.log("handleSelectAliasChange communication", template)
    }, [template])

    const setEmail = async (newEmail: any, alias = "") => {
        var newTemplate = {...template, alias}
        newTemplate.i18n["en"].email = newEmail
        setTemplate(newTemplate)
    }

    const handleLangChange = (lang: string) => async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {checked} = e.target
        var newTemplate = {...template}
        if (checked) {
            // Add the language if it's not already in the array
            if (!newTemplate.language_conf.enabled_languages.includes(lang)) {
                newTemplate.language_conf.enabled_languages.push(lang)
            }
        } else {
            // Remove the language if it's in the array
            newTemplate.language_conf.enabled_languages =
                newTemplate.language_conf.enabled_languages.filter((l) => l !== lang)
        }
        setTemplate(newTemplate)
    }

    const validateDate = (value: any) => {
        if (!template.schedule.now && !value) {
            return t("sendCommunication.chooseDate")
        }
    }

    // communication templates

    const [selectedMethod, setSelectedMethod] = useState<ITemplateMethod>(ITemplateMethod.EMAIL)
    const [selectedList, setSelectedList] = useState<ISendTemplateBody[] | null>(null)
    /*const [selectedReceipt, setSelectedReceipt] = useState<IEmail | string>({
        subject: "",
        plaintext_body: "",
        html_body: "",
    })*/

    const {data: receipts} = useGetList<Sequent_Backend_Template>("sequent_backend_template", {
        filter: {
            tenant_id: tenantId,
        },
    })

    useEffect(() => {
        // filter receipts by communication method
        const selectedReceipts = receipts
            ?.filter((receipt) => receipt.communication_method === selectedMethod)
            .map((receipt) => receipt.template)

        setSelectedList(selectedReceipts ?? null)
    }, [selectedMethod, receipts])

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
                record={template}
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
                            value={template.audience.selection}
                            onChange={handleSelectChange}
                        >
                            {(Object.keys(AudienceSelection) as Array<AudienceSelection>).map(
                                (key) => (
                                    <MenuItem key={key} value={key}>
                                        {t(`sendCommunication.votersSelection.${key}`, {
                                            total: template.audience.voter_ids?.length ?? 0,
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
                                    checked={template.schedule.now}
                                    onChange={handleNowChange}
                                />
                            }
                        />
                        <DateTimeInput
                            validate={validateDate}
                            disabled={template.schedule.now}
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
                            value={template.communication_method}
                            onChange={handleSelectMethodChange}
                        >
                            {Object.values(ITemplateMethod)
                                .filter((method) => method !== ITemplateMethod.DOCUMENT)
                                .map((key) => (
                                    <MenuItem key={key} value={key}>
                                        {t(`sendCommunication.communicationMethod.${key}`)}
                                    </MenuItem>
                                ))}
                        </FormStyles.Select>
                        <Typography variant="body2" sx={{margin: "0"}}>
                            {t("sendCommunication.alias")}
                        </Typography>
                        <FormStyles.Select
                            name="alias"
                            value={template.alias || ""}
                            onChange={handleSelectAliasChange}
                        >
                            {selectedList
                                ? selectedList?.map((key: ISendTemplateBody, index: number) => (
                                      <MenuItem key={index} value={key.alias}>
                                          {key.alias}
                                      </MenuItem>
                                  ))
                                : null}
                        </FormStyles.Select>
                        {template.communication_method === ITemplateMethod.EMAIL &&
                            template.i18n["en"].email && (
                                <EmailEditor
                                    record={template.i18n["en"].email}
                                    setRecord={setEmail}
                                />
                            )}
                        {template.communication_method === ITemplateMethod.SMS && (
                            <FormStyles.TextField
                                name="sms"
                                label={t("sendCommunication.smsMessage")}
                                value={template.i18n["en"].sms?.message ?? ""}
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
