// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useMemo, useState} from "react"
import {
    SaveButton,
    SimpleForm,
    useNotify,
    Toolbar,
    DateTimeInput,
    Identifier,
    useGetList,
    useGetOne,
} from "react-admin"
import {
    Alert,
    AccordionDetails,
    AccordionSummary,
    MenuItem,
    FormControlLabel,
    SelectChangeEvent,
    Switch,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import {useMutation} from "@apollo/client"
import {FieldValues, SubmitHandler} from "react-hook-form"
import MailIcon from "@mui/icons-material/Mail"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import {ChannelLabel} from "@sequentech/ui-essentials"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import EmailEditor from "@/components/EmailEditor"
import {useTranslation} from "react-i18next"
import {FormStyles} from "@/components/styles/FormStyles"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {CREATE_SCHEDULED_EVENT} from "@/queries/CreateScheduledEvent"
import {getReferencedSecretAttributeNames} from "@/services/secretAttributeTemplates"
import {CreateScheduledEventMutation, Sequent_Backend_Template} from "@/gql/graphql"
import {ScheduledEventType} from "@/services/ScheduledEvent"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {
    ITemplateMethod,
    IEmail,
    IMethods,
    ISmsConfig,
    MESSAGE_TEMPLATE_METHODS,
} from "@/types/templates"
import {
    EChannelSelection,
    EMessageChannel,
    EMessagePurpose,
    IInstantMessageConfig,
    MESSAGING_CONFIG_ANNOTATION,
} from "@/types/messaging"
import {enabledChannels, parseEventMessagingConfig} from "@/services/messaging"
import {methodChannel} from "@/services/templateMethods"
import {useMessagingAccounts} from "@/hooks/useMessagingAccounts"
import {useLocation} from "react-router-dom"
import {
    ISendContent,
    buildSendTemplatePayload,
    missingChannelContent,
    templatesForChannelSelection,
} from "./sendTemplatePayload"

export enum AudienceSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED",
}

const EACH_VOTER = EChannelSelection.VOTER_PREFERENCE
const DEFAULT_CHANNELS = [EMessageChannel.EMAIL, EMessageChannel.SMS]
const INSTANT_METHODS = [ITemplateMethod.WHATSAPP, ITemplateMethod.VIBER, ITemplateMethod.MESSENGER]

type InstantContentKey = "whatsapp" | "viber" | "messenger"

const instantKey = (method: ITemplateMethod): InstantContentKey | undefined => {
    switch (method) {
        case ITemplateMethod.WHATSAPP:
            return "whatsapp"
        case ITemplateMethod.VIBER:
            return "viber"
        case ITemplateMethod.MESSENGER:
            return "messenger"
        default:
            return undefined
    }
}

interface ITemplate {
    audience: {
        selection: AudienceSelection
        voter_ids?: Array<Identifier> | undefined
    }
    channel_selection: EChannelSelection
    communication_method: ITemplateMethod
    alias?: string
    schedule: {
        now: boolean
        date?: Date
    }
    i18n: {
        [lang_code: string]: ISendContent
    }
    language_conf: {
        enabled_languages: Array<string>
        default_language_code: string
    }
}

interface ITemplateContents {
    alias?: string
    selected_methods?: IMethods
    email?: IEmail
    sms?: ISmsConfig
    whatsapp?: IInstantMessageConfig
    viber?: IInstantMessageConfig
    messenger?: IInstantMessageConfig
}

interface IEventRecord {
    id: string
    annotations?: Record<string, unknown> | null
}

interface SendTemplateProps {
    ids?: Array<Identifier>
    audienceSelection?: AudienceSelection
    electionEventId?: string
    close?: () => void
    secretAttributeNames?: string[]
}

export const SendTemplate: React.FC<SendTemplateProps> = ({
    ids,
    audienceSelection,
    close,
    electionEventId,
    secretAttributeNames = [],
}) => {
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
        channel_selection: EACH_VOTER,
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

    const {data: electionEvent} = useGetOne<IEventRecord>(
        "sequent_backend_election_event",
        {id: electionEventId ?? ""},
        {enabled: !!electionEventId}
    )
    const {accounts} = useMessagingAccounts({skip: !electionEventId})
    const messagingConfig = useMemo(
        () => parseEventMessagingConfig(electionEvent?.annotations?.[MESSAGING_CONFIG_ANNOTATION]),
        [electionEvent]
    )
    const noticeChannels = useMemo(() => {
        const channels = enabledChannels(messagingConfig, EMessagePurpose.NOTICE)
        return channels.length ? channels : DEFAULT_CHANNELS
    }, [messagingConfig])
    const sendingAccount = (channel: EMessageChannel) => {
        const accountId = messagingConfig.channels.find(
            (config) => config.channel === channel
        )?.account_id
        return accounts.find((account) => account.id === accountId)
    }

    const content = template.i18n["en"]

    const getPayload = (formData: ITemplate) => {
        const templateContents = JSON.stringify(formData.i18n)
        return buildSendTemplatePayload({
            audienceSelection: formData.audience.selection,
            voterIds: formData.audience.voter_ids,
            channelSelection: formData.channel_selection,
            communicationMethod: formData.communication_method,
            scheduleNow: formData.schedule.now,
            scheduleDate: formData.schedule.date,
            content: formData.i18n["en"],
            secretAttributeNames: getReferencedSecretAttributeNames(
                templateContents,
                secretAttributeNames
            ),
        })
    }

    const onSubmit: SubmitHandler<FieldValues> = async (formData) => {
        const scheduleDate = (formData as Partial<ITemplate>).schedule?.date
        setErrors(null)
        setShowProgress(true)
        try {
            const {errors} = await createScheduledEvent({
                variables: {
                    tenantId: tenantId,
                    electionEventId: electionEventId,
                    eventProcessor: ScheduledEventType.SEND_TEMPLATE,
                    cronConfig: undefined,
                    eventPayload: getPayload({
                        ...template,
                        schedule: {now: template.schedule.now, date: scheduleDate},
                    }),
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
        } catch (error) {
            setShowProgress(false)
            setErrors(t("sendCommunication.errorSending", {error: String(error)}))
        }
    }

    const updateContent = (next: ISendContent, alias = template.alias) =>
        setTemplate({...template, alias, i18n: {...template.i18n, en: {...content, ...next}}})

    const handleNowChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const {checked} = e.target
        setTemplate({...template, schedule: {...template.schedule, now: checked}})
    }

    const handleSmsChange = (e: React.ChangeEvent<HTMLInputElement>) =>
        updateContent({sms: {message: e.target.value}})

    const handleInstantChange =
        (key: InstantContentKey) => (e: React.ChangeEvent<HTMLInputElement>) =>
            updateContent({
                [key]: {message: e.target.value, parameters: content[key]?.parameters ?? []},
            })

    const handleSelectChange = (e: SelectChangeEvent<unknown>) =>
        setTemplate({
            ...template,
            audience: {...template.audience, selection: e.target.value as AudienceSelection},
        })

    const handleChannelChange = (e: SelectChangeEvent<unknown>) => {
        const value = e.target.value as string
        if (value === EACH_VOTER) {
            setTemplate({...template, channel_selection: EACH_VOTER, alias: undefined})
        } else {
            setTemplate({
                ...template,
                channel_selection: EChannelSelection.SINGLE_CHANNEL,
                communication_method: value as ITemplateMethod,
                alias: undefined,
            })
        }
    }

    const {data: receipts} = useGetList<Sequent_Backend_Template>("sequent_backend_template", {
        filter: {
            tenant_id: tenantId,
        },
    })

    const selectableTemplates = useMemo(
        () =>
            templatesForChannelSelection(
                (receipts ?? []).map((receipt) => ({
                    communication_method: receipt.communication_method,
                    template: receipt.template as ITemplateContents,
                })),
                template.channel_selection,
                template.communication_method
            ).map((receipt) => receipt.template),
        [receipts, template.channel_selection, template.communication_method]
    )

    const handleSelectAliasChange = (e: SelectChangeEvent<unknown>) => {
        const value = e.target.value as string
        const selected = selectableTemplates.find((candidate) => candidate.alias === value)
        if (!selected) {
            return
        }
        updateContent(
            {
                email: selected.email ?? content.email,
                sms: selected.sms ?? content.sms,
                whatsapp: selected.whatsapp,
                viber: selected.viber,
                messenger: selected.messenger,
            },
            value
        )
    }

    const setEmail = (newEmail: IEmail) => updateContent({email: newEmail})

    const validateDate = (value: unknown) => {
        if (!template.schedule.now && !value) {
            return t("sendCommunication.chooseDate")
        }
    }

    const eachVoter = template.channel_selection === EACH_VOTER
    const shownMethods = eachVoter ? MESSAGE_TEMPLATE_METHODS : [template.communication_method]
    const shows = (method: ITemplateMethod) => shownMethods.includes(method)
    const missing = eachVoter ? missingChannelContent(content, noticeChannels) : []
    const channelName = (channel: EMessageChannel) => t(`messaging.channel.${channel}`)
    const singleChannel = methodChannel(template.communication_method)

    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={
                    <Toolbar>
                        <SaveButton
                            icon={<MailIcon />}
                            label={String(t("sendCommunication.sendButton"))}
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
                            label={String(t("sendCommunication.nowInput"))}
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
                            label={String(t("sendCommunication.dateInput"))}
                            parse={(value) => new Date(value).toISOString()}
                        />
                    </AccordionDetails>
                </FormStyles.AccordionExpanded>

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
                            {t("messaging.send.channel")}
                        </Typography>
                        <FormStyles.Select
                            name="channel"
                            value={eachVoter ? EACH_VOTER : template.communication_method}
                            onChange={handleChannelChange}
                        >
                            <MenuItem value={EACH_VOTER}>{t("messaging.send.eachVoter")}</MenuItem>
                            {MESSAGE_TEMPLATE_METHODS.map((method) => (
                                <MenuItem key={method} value={method}>
                                    {t("messaging.send.only", {
                                        channel: t(
                                            `sendCommunication.communicationMethod.${method}`
                                        ),
                                    })}
                                </MenuItem>
                            ))}
                        </FormStyles.Select>
                        <Typography variant="caption" color="text.secondary" component="p">
                            {eachVoter
                                ? t("messaging.send.eachVoterHelp")
                                : t("messaging.send.onlyHelp", {
                                      channel: singleChannel ? channelName(singleChannel) : "",
                                  })}
                        </Typography>
                        {electionEventId && messagingConfig.channels.length > 0 ? (
                            <Table size="small" aria-label={String(t("messaging.send.sendsFrom"))}>
                                <TableHead>
                                    <TableRow>
                                        <TableCell>{t("messaging.send.channelColumn")}</TableCell>
                                        <TableCell>{t("messaging.send.sendsFrom")}</TableCell>
                                    </TableRow>
                                </TableHead>
                                <TableBody>
                                    {(eachVoter
                                        ? noticeChannels
                                        : singleChannel
                                          ? [singleChannel]
                                          : []
                                    ).map((channel) => (
                                        <TableRow key={channel}>
                                            <TableCell>
                                                <ChannelLabel
                                                    channel={channel}
                                                    label={channelName(channel)}
                                                />
                                            </TableCell>
                                            <TableCell>
                                                {sendingAccount(channel)?.name ??
                                                    t("messaging.send.noAccount")}
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
                        ) : null}
                        <Typography variant="body2" sx={{margin: "0"}}>
                            {t("sendCommunication.alias")}
                        </Typography>
                        <FormStyles.Select
                            name="alias"
                            value={template.alias || ""}
                            onChange={handleSelectAliasChange}
                        >
                            {selectableTemplates.map((candidate, index) => (
                                <MenuItem key={index} value={candidate.alias}>
                                    {candidate.alias}
                                </MenuItem>
                            ))}
                        </FormStyles.Select>
                        {missing.length > 0 ? (
                            <Alert severity="warning">
                                {t("messaging.send.missingContent", {
                                    channels: missing.map(channelName).join(", "),
                                })}
                            </Alert>
                        ) : null}
                        {shows(ITemplateMethod.EMAIL) && content.email && (
                            <EmailEditor record={content.email} setRecord={setEmail} />
                        )}
                        {shows(ITemplateMethod.SMS) && (
                            <FormStyles.TextField
                                name="sms"
                                label={String(t("sendCommunication.smsMessage"))}
                                value={content.sms?.message ?? ""}
                                onChange={handleSmsChange}
                                multiline={true}
                                minRows={4}
                            />
                        )}
                        {INSTANT_METHODS.filter(shows).map((method) => {
                            const key = instantKey(method)
                            if (!key) {
                                return null
                            }
                            const value = content[key]
                            const editable = method === ITemplateMethod.MESSENGER
                            if (!value && eachVoter) {
                                return null
                            }
                            return (
                                <FormStyles.TextField
                                    key={method}
                                    name={key}
                                    label={String(t(`template.method.${key}`))}
                                    value={value?.message ?? ""}
                                    onChange={editable ? handleInstantChange(key) : undefined}
                                    InputProps={{readOnly: !editable}}
                                    helperText={
                                        editable
                                            ? undefined
                                            : String(t("messaging.send.approvedTemplateHelp"))
                                    }
                                    multiline={true}
                                    minRows={3}
                                />
                            )
                        })}
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
