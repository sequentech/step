// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useMemo, useState} from "react"
import {SaveButton, SimpleForm, useNotify, Toolbar, Identifier, useGetList} from "react-admin"
import {
    Alert,
    Box,
    Accordion,
    AccordionDetails,
    AccordionSummary,
    MenuItem,
    FormControlLabel,
    SelectChangeEvent,
    Switch,
    Typography,
} from "@mui/material"
import {ChannelIcon} from "@sequentech/ui-essentials"
import {useMutation} from "@apollo/client"
import {FieldValues, SubmitHandler} from "react-hook-form"
import MailIcon from "@mui/icons-material/Mail"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
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
import {IEmail, IMethods, ISmsConfig} from "@/types/templates"
import {EMessageChannel, IInstantMessageConfig} from "@/types/messaging"
import {useMessagingAccounts} from "@/hooks/useMessagingAccounts"
import {useEventMessagingConfig} from "@/hooks/useEventMessagingConfig"
import {useLocation} from "react-router-dom"
import {
    ISendContent,
    buildSendTemplatePayload,
    missingChannelContent,
    sendNoticeChannels,
    sendableTemplates,
    sendingAccountFor,
} from "./sendTemplatePayload"
import {SendTemplateAudience} from "./SendTemplateAudience"
import {ESendsFrom, type IAudiencePreview} from "@/queries/GetMessagingAudiencePreview"
import {
    ZonedDateTimeInput,
    type IStoredZonedDateTime,
} from "@/components/timezones/ZonedDateTimeInput"
import {useTimeZoneContext} from "@/components/timezones/useTimeZoneContext"
import {useTimeZoneService} from "@/components/timezones/timeZoneService"

export enum AudienceSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED",
}

// Without an event configuration, notices go by SMS or email, in channel order.
const INSTANT_METHODS = [
    EMessageChannel.WHATSAPP,
    EMessageChannel.VIBER,
    EMessageChannel.MESSENGER,
] as const

type InstantContentKey = "whatsapp" | "viber" | "messenger"

const instantKey = (method: EMessageChannel): InstantContentKey | undefined => {
    switch (method) {
        case EMessageChannel.WHATSAPP:
            return "whatsapp"
        case EMessageChannel.VIBER:
            return "viber"
        case EMessageChannel.MESSENGER:
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
    alias?: string
    schedule: {
        now: boolean
        /** The time as entered: the instant, its wall time and zone. */
        zoned?: IStoredZonedDateTime | null
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

/** Closing a channel keeps its editor mounted and its content in the send. */
const SendChannelSection: React.FC<{
    channel: EMessageChannel
    children: React.ReactNode
}> = ({channel, children}) => {
    const {t} = useTranslation()
    const id = React.useId()
    return (
        <Accordion sx={{width: "100%", minWidth: 0}}>
            <AccordionSummary
                id={`${id}-summary`}
                aria-controls={`${id}-content`}
                expandIcon={<ExpandMoreIcon />}
                // The always-open outer sections hide their arrows, including descendants.
                sx={{"&& .MuiAccordionSummary-expandIconWrapper": {display: "flex"}}}
            >
                <ElectionHeaderStyles.AccordionTitle
                    sx={{display: "flex", alignItems: "center", gap: 1}}
                >
                    <ChannelIcon channel={channel} fontSize="small" />
                    {t(`template.method.${channel.toLowerCase()}`)}
                </ElectionHeaderStyles.AccordionTitle>
            </AccordionSummary>
            <AccordionDetails sx={{display: "flex", flexDirection: "column", gap: 2, minWidth: 0}}>
                {children}
            </AccordionDetails>
        </Accordion>
    )
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
    const [audiencePreview, setAudiencePreview] = useState<IAudiencePreview>()
    // A time is entered in the event's primary zone, else the viewer's.
    const zones = useTimeZoneContext(electionEventId)
    const timeZones = useTimeZoneService()

    const [template, setTemplate] = useState<ITemplate>({
        audience: {
            selection: audienceSelection ?? AudienceSelection.SELECTED,
            voter_ids: ids ?? undefined,
        },
        schedule: {
            now: true,
            zoned: undefined,
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

    const {accounts} = useMessagingAccounts({skip: !electionEventId, summaries: true})
    const {
        storedConfig,
        canRead: canReadMessaging,
        loading: messagingLoading,
    } = useEventMessagingConfig(electionEventId)
    // The channels the backend will send on; nothing is guessed when they can't be read.
    const notices = useMemo(
        () => sendNoticeChannels(canReadMessaging, storedConfig),
        [canReadMessaging, storedConfig]
    )
    const noticeChannels = notices.kind === "CHANNELS" ? notices.channels : []
    const sendsFrom = (channel: EMessageChannel): string => {
        const account = sendingAccountFor(storedConfig, accounts, channel)
        if (account === "ENVIRONMENT") return t("messaging.send.environment")
        return account?.name ?? t("messaging.send.noAccount")
    }

    const content = template.i18n["en"]

    const getPayload = (formData: ITemplate) => {
        const templateContents = JSON.stringify(formData.i18n)
        return buildSendTemplatePayload({
            audienceSelection: formData.audience.selection,
            voterIds: formData.audience.voter_ids,
            scheduleNow: formData.schedule.now,
            scheduleDate: formData.schedule.zoned?.scheduled_date ?? undefined,
            scheduleLocal: formData.schedule.zoned?.local ?? undefined,
            scheduleTimezone: formData.schedule.zoned?.timezone ?? undefined,
            alias: formData.alias,
            content: formData.i18n["en"],
            secretAttributeNames: getReferencedSecretAttributeNames(
                templateContents,
                secretAttributeNames
            ),
        })
    }

    const onSubmit: SubmitHandler<FieldValues> = async (formData) => {
        const zoned = (formData as Partial<ITemplate>).schedule?.zoned
        setErrors(null)
        if (electionEventId && notices.kind === "NONE") {
            // Every voter's notice would be refused: nothing is scheduled.
            setErrors(t("messaging.send.noNoticeChannels"))
            return
        }
        if (missing.length > 0) {
            setErrors(
                t("messaging.send.missingContent", {channels: missing.map(channelName).join(", ")})
            )
            return
        }
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
                        schedule: {now: template.schedule.now, zoned},
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

    const handleInstantMessageChange =
        (key: InstantContentKey) => (e: React.ChangeEvent<HTMLInputElement>) =>
            updateContent({
                [key]: {
                    ...content[key],
                    parameters: content[key]?.parameters ?? [],
                    message: e.target.value,
                },
            })

    const handleSelectChange = (e: SelectChangeEvent<unknown>) => {
        setAudiencePreview(undefined)
        setTemplate({
            ...template,
            audience: {...template.audience, selection: e.target.value as AudienceSelection},
        })
    }

    const {data: receipts} = useGetList<Sequent_Backend_Template>("sequent_backend_template", {
        filter: {
            tenant_id: tenantId,
        },
        pagination: {page: 1, perPage: 1000},
        sort: {field: "alias", order: "ASC"},
    })

    const selectableTemplates = useMemo(
        () =>
            sendableTemplates(
                (receipts ?? []).map((receipt) => ({
                    communication_method: receipt.communication_method,
                    template: receipt.template as ITemplateContents,
                }))
            ).map((receipt) => receipt.template),
        [receipts]
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

    const validateDate = (value: IStoredZonedDateTime | null | undefined) => {
        if (!template.schedule.now && !value?.scheduled_date) {
            return t("sendCommunication.chooseDate")
        }
    }

    // Every voter gets this notification on their own channel: there's no other choice.
    const audienceChannels = audiencePreview
        ? audiencePreview.channels.flatMap((row) =>
              row.channel &&
              row.voters > row.not_offered &&
              row.sends_from &&
              row.sends_from !== ESendsFrom.NONE
                  ? [row.channel]
                  : []
          )
        : noticeChannels
    const missing = missingChannelContent(content, audienceChannels)
    const channelName = (channel: EMessageChannel) => t(`messaging.channel.${channel}`)

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
                        <ZonedDateTimeInput
                            validate={validateDate}
                            disabled={template.schedule.now}
                            source="schedule.zoned"
                            label={String(t("sendCommunication.dateInput"))}
                            defaultZone={electionEventId ? zones.primary : timeZones.myTimeZone}
                            zones={electionEventId ? zones.configured : undefined}
                            primary={electionEventId ? zones.primary : undefined}
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
                        {electionEventId && notices.kind === "NO_PERMISSION" ? (
                            <Alert severity="info">{t("messaging.send.noReadPermission")}</Alert>
                        ) : null}
                        {electionEventId && notices.kind === "NONE" && !messagingLoading ? (
                            <Alert severity="error">{t("messaging.send.noNoticeChannels")}</Alert>
                        ) : null}
                        {electionEventId && noticeChannels.length > 0 ? (
                            <SendTemplateAudience
                                electionEventId={electionEventId}
                                audienceSelection={template.audience.selection}
                                voterIds={template.audience.voter_ids}
                                channels={noticeChannels}
                                sendsFrom={sendsFrom}
                                onPreview={setAudiencePreview}
                            />
                        ) : null}
                        <Typography variant="caption" color="text.secondary" component="p">
                            {t("messaging.send.eachVoterHelp")}
                        </Typography>
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
                        {content.email && (
                            <SendChannelSection channel={EMessageChannel.EMAIL}>
                                <EmailEditor record={content.email} setRecord={setEmail} />
                            </SendChannelSection>
                        )}
                        <SendChannelSection channel={EMessageChannel.SMS}>
                            <FormStyles.TextField
                                name="sms"
                                label={String(t("sendCommunication.smsMessage"))}
                                value={content.sms?.message ?? ""}
                                onChange={handleSmsChange}
                                multiline={true}
                                minRows={4}
                            />
                        </SendChannelSection>
                        {INSTANT_METHODS.map((method) => {
                            const key = instantKey(method)
                            if (!key) {
                                return null
                            }
                            const value = content[key]
                            const editable = method === EMessageChannel.MESSENGER
                            if (!value) {
                                return null
                            }
                            const channel = t(`template.method.${key}`)
                            return (
                                <SendChannelSection key={method} channel={method}>
                                    {editable ? (
                                        <FormStyles.TextField
                                            name={key}
                                            label={String(channel)}
                                            value={value.message}
                                            onChange={handleInstantMessageChange(key)}
                                            multiline
                                            minRows={3}
                                        />
                                    ) : (
                                        <>
                                            <Alert severity="info">
                                                {t("messaging.send.approvedTemplateHelp", {
                                                    channel,
                                                })}
                                            </Alert>
                                            <Box>
                                                <Typography
                                                    variant="caption"
                                                    color="text.secondary"
                                                >
                                                    {t("messaging.templates.approvedWording")}
                                                </Typography>
                                                <Typography
                                                    component="p"
                                                    variant="body2"
                                                    sx={{
                                                        m: 0,
                                                        p: 2,
                                                        borderRadius: 1,
                                                        bgcolor: "action.hover",
                                                        whiteSpace: "pre-wrap",
                                                        overflowWrap: "anywhere",
                                                    }}
                                                >
                                                    {value.message}
                                                </Typography>
                                            </Box>
                                        </>
                                    )}
                                </SendChannelSection>
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
