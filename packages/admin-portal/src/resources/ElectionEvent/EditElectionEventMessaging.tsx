// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useGetList, useNotify, useRecordContext, useRefresh} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Alert,
    Autocomplete,
    Box,
    Button,
    Checkbox,
    Chip,
    FormControlLabel,
    IconButton,
    MenuItem,
    Switch,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import AddIcon from "@mui/icons-material/Add"
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward"
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward"
import {ChannelIcon, ChannelLabel} from "@sequentech/ui-essentials"
import {
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Template,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useMessagingAccounts} from "@/hooks/useMessagingAccounts"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {GET_MESSAGE_DELIVERY_STATS} from "@/queries/GetMessageDeliveryStats"
import {UPDATE_EVENT_MESSAGING_CONFIG} from "@/queries/UpdateEventMessagingConfig"
import {IPermissions} from "@/types/keycloak"
import {
    EDeliveryFeedback,
    EMessageAttemptState,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    IEventMessagingConfig,
    IMessagingAccount,
    IMessagingConfigError,
    ITemplateBinding,
    KEYCLOAK_NOTICE_MESSAGE_KEYS,
    KEYCLOAK_OTP_MESSAGE_KEYS,
    MESSAGE_ATTEMPT_STATES,
    MESSAGE_CHANNELS,
    MESSAGE_PURPOSES,
    MESSAGING_CONFIG_ANNOTATION,
} from "@/types/messaging"
import {
    accountCapabilities,
    accountSummary,
    deliverySummary,
    parseEventMessagingConfig,
    purposeReadiness,
    supportsOutOfWindow,
    validateEventMessagingConfig,
} from "@/services/messaging"
import {
    EBindingApproval,
    EMessagingErrorArea,
    IBindingPosition,
    IDeliveryStatsResponse,
    IMessagingErrorLocation,
    activeChannels,
    addTemplateBinding,
    bindingApproval,
    electionChannelsOf,
    errorsAt,
    incompleteBindings,
    moveFallback,
    parseDeliveryStats,
    parseMessagingConfigErrors,
    preparedConfig,
    removeTemplateBinding,
    setChannelAccount,
    setOutOfWindow,
    setReplyText,
    templateChannels,
    toggleElectionChannel,
    togglePurpose,
    updateTemplateBinding,
} from "./eventMessagingConfig"

const POSTS_SHOWN = 15

interface IUpdateEventMessagingConfigResult {
    update_event_messaging_config?: {errors: unknown} | null
}

type TOnChange = (next: IEventMessagingConfig) => void

const accountOf = (
    config: IEventMessagingConfig,
    accounts: IMessagingAccount[],
    channel: EMessageChannel
): IMessagingAccount | undefined => {
    const accountId = config.channels.find((entry) => entry.channel === channel)?.account_id
    return accounts.find((account) => account.id === accountId)
}

const Section: React.FC<React.PropsWithChildren<{title: string; id: string}>> = ({
    title,
    id,
    children,
}) => (
    <Accordion
        defaultExpanded
        disableGutters
        sx={{width: "100%"}}
        id={id}
        slotProps={{region: {"aria-labelledby": `${id}-header`, "id": `${id}-content`}}}
    >
        <AccordionSummary
            expandIcon={<ExpandMoreIcon />}
            id={`${id}-header`}
            aria-controls={`${id}-content`}
        >
            <ElectionHeaderStyles.AccordionTitle>{title}</ElectionHeaderStyles.AccordionTitle>
        </AccordionSummary>
        <AccordionDetails sx={{display: "flex", flexDirection: "column", gap: 1.5}}>
            {children}
        </AccordionDetails>
    </Accordion>
)

const Helper: React.FC<React.PropsWithChildren> = ({children}) => (
    <Typography variant="body2" sx={{color: "text.secondary", maxWidth: 900}}>
        {children}
    </Typography>
)

export interface MessagingErrorsProps {
    errors: IMessagingConfigError[]
    electionName?: (electionId: string) => string
}

/** Configuration problems, worded for the administrator. */
export const MessagingErrors: React.FC<MessagingErrorsProps> = ({errors, electionName}) => {
    const {t} = useTranslation()
    if (!errors.length) return null
    const text = (error: IMessagingConfigError) => {
        const channel = "channel" in error ? t(`messaging.channel.${error.channel}`) : ""
        const purpose = "purpose" in error ? t(`messaging.purpose.${error.purpose}`) : ""
        const blockers =
            error.kind === "PURPOSE_NOT_READY"
                ? error.blockers.map((blocker) => t(`messaging.blocker.${blocker}`)).join(", ")
                : ""
        const election =
            "election_id" in error ? (electionName?.(error.election_id) ?? error.election_id) : ""
        return t(`messagingEvent.error.${error.kind}`, {
            channel,
            purpose,
            blockers,
            election,
            language: "language" in error ? error.language : "",
            version: "version" in error ? error.version : "",
        })
    }
    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 0.5}}>
            {errors.map((error) => (
                <Typography
                    key={JSON.stringify(error)}
                    variant="caption"
                    role="alert"
                    sx={{color: "error.main"}}
                >
                    {text(error)}
                </Typography>
            ))}
        </Box>
    )
}

export interface MessagingChannelsProps {
    config: IEventMessagingConfig
    accounts: IMessagingAccount[]
    errors: IMessagingConfigError[]
    canEdit: boolean
    onChange: TOnChange
}

/** Per channel: the account it sends from and its separate OTP and notice switches. */
export const MessagingChannels: React.FC<MessagingChannelsProps> = ({
    config,
    accounts,
    errors,
    canEdit,
    onChange,
}) => {
    const {t} = useTranslation()
    const at = (location: IMessagingErrorLocation) => errorsAt(errors, config, location)
    return (
        <Table size="small" aria-label={t("messagingEvent.sections.channels")}>
            <TableHead>
                <TableRow>
                    <TableCell>{t("messagingEvent.column.channel")}</TableCell>
                    <TableCell>{t("messagingEvent.column.account")}</TableCell>
                    {MESSAGE_PURPOSES.map((purpose) => (
                        <TableCell key={purpose}>{t(`messaging.purpose.${purpose}`)}</TableCell>
                    ))}
                </TableRow>
            </TableHead>
            <TableBody>
                {MESSAGE_CHANNELS.map((channel) => {
                    const options = accounts.filter((account) => account.channel === channel)
                    const entry = config.channels.find((item) => item.channel === channel)
                    const account = accountOf(config, accounts, channel)
                    const capabilities = account ? accountCapabilities(account) : null
                    const channelName = t(`messaging.channel.${channel}`)
                    return (
                        <TableRow key={channel} data-channel={channel}>
                            <TableCell sx={{verticalAlign: "top"}}>
                                <ChannelLabel channel={channel} label={channelName} />
                            </TableCell>
                            <TableCell sx={{minWidth: 260, verticalAlign: "top"}}>
                                {options.length || entry ? (
                                    <TextField
                                        select
                                        size="small"
                                        fullWidth
                                        label={t("messagingEvent.accountLabel", {
                                            channel: channelName,
                                        })}
                                        value={entry?.account_id ?? ""}
                                        disabled={!canEdit}
                                        onChange={(event) =>
                                            onChange(
                                                setChannelAccount(
                                                    config,
                                                    channel,
                                                    event.target.value || null
                                                )
                                            )
                                        }
                                    >
                                        <MenuItem value="">{t("messagingEvent.notUsed")}</MenuItem>
                                        {options.map((option) => (
                                            <MenuItem key={option.id} value={option.id}>
                                                {option.name}
                                            </MenuItem>
                                        ))}
                                        {entry && !account ? (
                                            <MenuItem value={entry.account_id}>
                                                {t("messagingEvent.missingAccount")}
                                            </MenuItem>
                                        ) : null}
                                    </TextField>
                                ) : (
                                    <Helper>{t("messagingEvent.noAccount")}</Helper>
                                )}
                                {entry &&
                                (supportsOutOfWindow(capabilities) ||
                                    entry.out_of_window !== EOutOfWindowPolicy.DISABLED) ? (
                                    <OutOfWindowPolicySelect
                                        channel={channel}
                                        value={entry.out_of_window}
                                        noticeTemplateBound={entry.templates.some(
                                            (binding) =>
                                                binding.purpose === EMessagePurpose.NOTICE &&
                                                !!binding.provider_template.trim()
                                        )}
                                        canEdit={canEdit}
                                        onChange={(policy) =>
                                            onChange(setOutOfWindow(config, channel, policy))
                                        }
                                    />
                                ) : null}
                                <MessagingErrors
                                    errors={at({area: EMessagingErrorArea.CHANNEL, channel})}
                                />
                            </TableCell>
                            {MESSAGE_PURPOSES.map((purpose) => {
                                const enabled = entry?.purposes.includes(purpose) ?? false
                                const blockers = account
                                    ? purposeReadiness(
                                          capabilities,
                                          account.provider_approval,
                                          account.readiness,
                                          account.status,
                                          purpose,
                                          null
                                      ).blockers
                                    : []
                                const blocked = !account || blockers.length > 0
                                return (
                                    <TableCell key={purpose} sx={{verticalAlign: "top"}}>
                                        <FormControlLabel
                                            label={t(`messaging.purpose.${purpose}`)}
                                            control={
                                                <Switch
                                                    checked={enabled}
                                                    disabled={!canEdit || (!enabled && blocked)}
                                                    onChange={(event) =>
                                                        onChange(
                                                            togglePurpose(
                                                                config,
                                                                channel,
                                                                purpose,
                                                                event.target.checked
                                                            )
                                                        )
                                                    }
                                                    slotProps={{
                                                        input: {
                                                            "aria-label": t(
                                                                "messagingEvent.purposeSwitch",
                                                                {
                                                                    channel: channelName,
                                                                    purpose: t(
                                                                        `messaging.purpose.${purpose}`
                                                                    ),
                                                                }
                                                            ),
                                                        },
                                                    }}
                                                />
                                            }
                                        />
                                        {account && blockers.length ? (
                                            <Typography
                                                variant="caption"
                                                sx={{
                                                    display: "block",
                                                    color: enabled
                                                        ? "error.main"
                                                        : "text.secondary",
                                                }}
                                            >
                                                {t("messagingEvent.missing", {
                                                    blockers: blockers
                                                        .map((blocker) =>
                                                            t(`messaging.blocker.${blocker}`)
                                                        )
                                                        .join(", "),
                                                })}
                                            </Typography>
                                        ) : null}
                                        <MessagingErrors
                                            errors={at({
                                                area: EMessagingErrorArea.PURPOSE,
                                                channel,
                                                purpose,
                                            })}
                                        />
                                    </TableCell>
                                )
                            })}
                        </TableRow>
                    )
                })}
            </TableBody>
        </Table>
    )
}

export interface OutOfWindowPolicySelectProps {
    channel: EMessageChannel
    value: EOutOfWindowPolicy
    /** Whether a template is bound for notices, which utility messages are sent with. */
    noticeTemplateBound: boolean
    canEdit: boolean
    onChange: (policy: EOutOfWindowPolicy) => void
}

/** How a channel with a conversation window sends notices once the window has closed. */
export const OutOfWindowPolicySelect: React.FC<OutOfWindowPolicySelectProps> = ({
    channel,
    value,
    noticeTemplateBound,
    canEdit,
    onChange,
}) => {
    const {t} = useTranslation()
    return (
        <Box sx={{marginTop: 1.5}}>
            <TextField
                select
                size="small"
                fullWidth
                disabled={!canEdit}
                label={t("messagingEvent.outOfWindow.label", {
                    channel: t(`messaging.channel.${channel}`),
                })}
                value={value}
                onChange={(event) => onChange(event.target.value as EOutOfWindowPolicy)}
            >
                {Object.values(EOutOfWindowPolicy).map((policy) => (
                    <MenuItem key={policy} value={policy}>
                        {t(`messagingEvent.outOfWindow.${policy}`)}
                    </MenuItem>
                ))}
            </TextField>
            <Typography variant="caption" sx={{display: "block", color: "text.secondary"}}>
                {t("messagingEvent.outOfWindow.help")}
            </Typography>
            {value === EOutOfWindowPolicy.UTILITY_MESSAGES && !noticeTemplateBound ? (
                <Typography
                    variant="caption"
                    role="status"
                    sx={{display: "block", color: "warning.dark"}}
                >
                    {t("messagingEvent.outOfWindow.noTemplate")}
                </Typography>
            ) : null}
        </Box>
    )
}

const APPROVAL_COLOR: Record<EBindingApproval, "success" | "default"> = {
    [EBindingApproval.APPROVED]: "success",
    [EBindingApproval.ADMIN_CONFIRMED]: "success",
    [EBindingApproval.NOT_APPROVED]: "default",
    [EBindingApproval.NOT_CHECKED]: "default",
}

export interface MessagingTemplateBindingsProps {
    config: IEventMessagingConfig
    accounts: IMessagingAccount[]
    languages: string[]
    /** Aliases of the tenant's templates, offered as the message a notice template is for. */
    templateAliases: string[]
    errors: IMessagingConfigError[]
    /** Rows to mark as missing the language or the provider's template. */
    incomplete: IBindingPosition[]
    canEdit: boolean
    onChange: TOnChange
}

/**
 * Per channel, the provider-approved templates of the event: which message
 * and language each one is for, and the name and language the provider knows
 * it by.
 */
export const MessagingTemplateBindings: React.FC<MessagingTemplateBindingsProps> = ({
    config,
    accounts,
    languages,
    templateAliases,
    errors,
    incomplete,
    canEdit,
    onChange,
}) => {
    const {t} = useTranslation()
    const summaries = useMemo(() => accounts.map(accountSummary), [accounts])
    const channels = templateChannels(config, summaries)
    if (!channels.length) {
        return <Helper>{t("messagingEvent.templates.empty")}</Helper>
    }
    const keyOptions = (purpose: EMessagePurpose) =>
        purpose === EMessagePurpose.NOTICE
            ? Array.from(new Set([...templateAliases, ...KEYCLOAK_NOTICE_MESSAGE_KEYS]))
            : KEYCLOAK_OTP_MESSAGE_KEYS
    return (
        <>
            <Helper>{t("messagingEvent.templates.help")}</Helper>
            <Helper>{t("messagingEvent.templates.order")}</Helper>
            {channels.map((channel) => {
                const entry = config.channels.find((item) => item.channel === channel)
                const account = accountOf(config, accounts, channel)
                const summary = summaries.find((item) => item.id === entry?.account_id)
                const channelName = t(`messaging.channel.${channel}`)
                const required = summary?.capabilities?.template_required_for ?? []
                const sender = account?.sender
                const templateOptions = (purpose: EMessagePurpose) =>
                    sender?.provider === EMessagingProvider.VIBER_INFOBIP
                        ? Object.values(sender.approved_templates?.[purpose] ?? {})
                        : []
                const bindings = entry?.templates ?? []
                return (
                    <Box
                        key={channel}
                        data-template-channel={channel}
                        sx={{display: "flex", flexDirection: "column", gap: 1.5}}
                    >
                        <ChannelLabel channel={channel} label={channelName} />
                        {bindings.length === 0 ? (
                            <Helper>
                                {required.length
                                    ? t("messagingEvent.templates.noneRequired", {
                                          channel: channelName,
                                      })
                                    : t("messagingEvent.templates.noneOptional", {
                                          channel: channelName,
                                      })}
                            </Helper>
                        ) : null}
                        {bindings.map((binding, index) => {
                            const update = (patch: Partial<ITemplateBinding>) =>
                                onChange(updateTemplateBinding(config, channel, index, patch))
                            const missing = incomplete.some(
                                (position) =>
                                    position.channel === channel && position.index === index
                            )
                            const approval = bindingApproval(summary, binding)
                            const group = t("messagingEvent.templates.row", {
                                channel: channelName,
                                position: index + 1,
                            })
                            return (
                                <Box
                                    key={index}
                                    role="group"
                                    aria-label={group}
                                    sx={{display: "flex", flexDirection: "column", gap: 0.5}}
                                >
                                    <Box
                                        sx={{
                                            display: "flex",
                                            gap: 1,
                                            flexWrap: "wrap",
                                            alignItems: "center",
                                        }}
                                    >
                                        <TextField
                                            select
                                            size="small"
                                            label={t("messagingEvent.column.purpose")}
                                            value={binding.purpose}
                                            disabled={!canEdit}
                                            onChange={(event) =>
                                                update({
                                                    purpose: event.target.value as EMessagePurpose,
                                                })
                                            }
                                            sx={{width: 130}}
                                        >
                                            {MESSAGE_PURPOSES.map((purpose) => (
                                                <MenuItem key={purpose} value={purpose}>
                                                    {t(`messaging.purpose.${purpose}`)}
                                                </MenuItem>
                                            ))}
                                        </TextField>
                                        <Autocomplete
                                            freeSolo
                                            size="small"
                                            disabled={!canEdit}
                                            options={keyOptions(binding.purpose)}
                                            inputValue={binding.key ?? ""}
                                            onInputChange={(_event, value) => update({key: value})}
                                            renderInput={(params) => (
                                                <TextField
                                                    {...params}
                                                    label={t("messagingEvent.column.key")}
                                                    placeholder={t(
                                                        "messagingEvent.templates.keyDefault"
                                                    )}
                                                />
                                            )}
                                            sx={{width: 220}}
                                        />
                                        <Autocomplete
                                            freeSolo
                                            size="small"
                                            disabled={!canEdit}
                                            options={languages}
                                            inputValue={binding.language}
                                            onInputChange={(_event, value) =>
                                                update({language: value})
                                            }
                                            renderInput={(params) => (
                                                <TextField
                                                    {...params}
                                                    required
                                                    error={missing && !binding.language.trim()}
                                                    label={t("messagingEvent.column.language")}
                                                />
                                            )}
                                            sx={{width: 150}}
                                        />
                                        <Autocomplete
                                            freeSolo
                                            size="small"
                                            disabled={!canEdit}
                                            options={templateOptions(binding.purpose)}
                                            inputValue={binding.provider_template}
                                            onInputChange={(_event, value) =>
                                                update({provider_template: value})
                                            }
                                            renderInput={(params) => (
                                                <TextField
                                                    {...params}
                                                    required
                                                    error={
                                                        missing && !binding.provider_template.trim()
                                                    }
                                                    label={t("messagingEvent.column.template")}
                                                />
                                            )}
                                            sx={{width: 240}}
                                        />
                                        <TextField
                                            size="small"
                                            label={t("messagingEvent.column.providerLanguage")}
                                            value={binding.provider_language ?? ""}
                                            disabled={!canEdit}
                                            onChange={(event) =>
                                                update({provider_language: event.target.value})
                                            }
                                            sx={{width: 190}}
                                        />
                                        <Chip
                                            size="small"
                                            color={APPROVAL_COLOR[approval]}
                                            label={t(
                                                `messagingEvent.templates.approval.${approval}`
                                            )}
                                        />
                                        {canEdit ? (
                                            <IconButton
                                                aria-label={t("messagingEvent.templates.remove", {
                                                    channel: channelName,
                                                    position: index + 1,
                                                })}
                                                onClick={() =>
                                                    onChange(
                                                        removeTemplateBinding(
                                                            config,
                                                            channel,
                                                            index
                                                        )
                                                    )
                                                }
                                            >
                                                <DeleteOutlineIcon />
                                            </IconButton>
                                        ) : null}
                                    </Box>
                                    {missing ? (
                                        <Typography
                                            variant="caption"
                                            role="alert"
                                            sx={{color: "error.main"}}
                                        >
                                            {t("messagingEvent.templates.incomplete")}
                                        </Typography>
                                    ) : null}
                                    <MessagingErrors
                                        errors={
                                            bindings.findIndex(
                                                (other) =>
                                                    other.purpose === binding.purpose &&
                                                    other.language === binding.language
                                            ) === index
                                                ? errorsAt(errors, config, {
                                                      area: EMessagingErrorArea.TEMPLATE,
                                                      channel,
                                                      purpose: binding.purpose,
                                                      language: binding.language,
                                                  })
                                                : []
                                        }
                                    />
                                </Box>
                            )
                        })}
                        {canEdit ? (
                            <Box>
                                <Button
                                    startIcon={<AddIcon />}
                                    onClick={() =>
                                        onChange(
                                            addTemplateBinding(
                                                config,
                                                channel,
                                                entry?.purposes[0] ??
                                                    required[0] ??
                                                    EMessagePurpose.NOTICE,
                                                languages[0] ?? ""
                                            )
                                        )
                                    }
                                >
                                    {t("messagingEvent.templates.add", {channel: channelName})}
                                </Button>
                            </Box>
                        ) : null}
                        <MessagingErrors
                            errors={errors.filter(
                                (error) =>
                                    error.kind === "TEMPLATE_NOT_APPROVED" &&
                                    error.channel === channel &&
                                    !bindings.some(
                                        (binding) =>
                                            binding.purpose === error.purpose &&
                                            binding.language === error.language
                                    )
                            )}
                        />
                    </Box>
                )
            })}
        </>
    )
}

export interface MessagingFallbackOrderProps {
    config: IEventMessagingConfig
    errors: IMessagingConfigError[]
    canEdit: boolean
    onChange: TOnChange
}

/** The order notices try the voter's other verified channels in. Never used for codes. */
export const MessagingFallbackOrder: React.FC<MessagingFallbackOrderProps> = ({
    config,
    errors,
    canEdit,
    onChange,
}) => {
    const {t} = useTranslation()
    const order = config.notice_fallback
    return (
        <>
            <Helper>{t("messagingEvent.fallback.help")}</Helper>
            {order.length ? (
                <Box
                    component="ol"
                    sx={{display: "flex", gap: 1, flexWrap: "wrap", padding: 0, margin: 0}}
                >
                    {order.map((channel, index) => {
                        const name = t(`messaging.channel.${channel}`)
                        return (
                            <Box
                                component="li"
                                key={channel}
                                sx={{
                                    display: "flex",
                                    alignItems: "center",
                                    gap: 0.5,
                                    border: 1,
                                    borderColor: "divider",
                                    borderRadius: "18px",
                                    padding: "2px 4px 2px 10px",
                                    listStyle: "none",
                                }}
                            >
                                <Typography variant="body2" sx={{fontWeight: 500}}>
                                    {index + 1}.
                                </Typography>
                                <ChannelIcon channel={channel} fontSize="small" />
                                <Typography variant="body2">{name}</Typography>
                                <IconButton
                                    size="small"
                                    aria-label={t("messagingEvent.fallback.earlier", {
                                        channel: name,
                                    })}
                                    disabled={!canEdit || index === 0}
                                    onClick={() => onChange(moveFallback(config, index, -1))}
                                >
                                    <ArrowUpwardIcon fontSize="inherit" />
                                </IconButton>
                                <IconButton
                                    size="small"
                                    aria-label={t("messagingEvent.fallback.later", {
                                        channel: name,
                                    })}
                                    disabled={!canEdit || index === order.length - 1}
                                    onClick={() => onChange(moveFallback(config, index, 1))}
                                >
                                    <ArrowDownwardIcon fontSize="inherit" />
                                </IconButton>
                            </Box>
                        )
                    })}
                </Box>
            ) : (
                <Helper>{t("messagingEvent.fallback.empty")}</Helper>
            )}
            <MessagingErrors
                errors={errorsAt(errors, config, {area: EMessagingErrorArea.FALLBACK})}
            />
        </>
    )
}

export interface IMessagingElection {
    id: string
    name: string
}

export interface MessagingElectionChannelsProps {
    config: IEventMessagingConfig
    elections: IMessagingElection[]
    errors: IMessagingConfigError[]
    canEdit: boolean
    onChange: TOnChange
}

/** Channels by Post: each election offers the ticked channels of the event. */
export const MessagingElectionChannels: React.FC<MessagingElectionChannelsProps> = ({
    config,
    elections,
    errors,
    canEdit,
    onChange,
}) => {
    const {t} = useTranslation()
    const [search, setSearch] = useState("")
    const active = activeChannels(config)
    const matching = elections.filter((election) =>
        election.name.toLowerCase().includes(search.trim().toLowerCase())
    )
    const restricted = elections.filter(
        (election) => electionChannelsOf(config, election.id).length < active.length
    ).length
    const known = new Set(elections.map((election) => election.id))
    const unknown = errors.filter(
        (error) => "election_id" in error && (!active.length || !known.has(error.election_id))
    )
    if (!active.length) {
        return (
            <>
                <Helper>{t("messagingEvent.posts.noChannels")}</Helper>
                <MessagingErrors errors={unknown} />
            </>
        )
    }
    return (
        <>
            <Helper>
                {t("messagingEvent.posts.help")}{" "}
                {restricted
                    ? t("messagingEvent.posts.restricted", {
                          count: restricted,
                          total: active.length,
                      })
                    : t("messagingEvent.posts.allChannels", {total: active.length})}
            </Helper>
            <TextField
                size="small"
                label={t("messagingEvent.posts.search")}
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                sx={{maxWidth: 320}}
            />
            <Table size="small" aria-label={t("messagingEvent.sections.posts")}>
                <TableHead>
                    <TableRow>
                        <TableCell>{t("messagingEvent.column.post")}</TableCell>
                        {active.map((channel) => (
                            <TableCell key={channel} align="center">
                                {t(`messaging.channel.${channel}`)}
                            </TableCell>
                        ))}
                    </TableRow>
                </TableHead>
                <TableBody>
                    {matching.slice(0, POSTS_SHOWN).map((election) => {
                        const offered = electionChannelsOf(config, election.id)
                        const name = election.name
                        return (
                            <TableRow key={election.id}>
                                <TableCell sx={{whiteSpace: "nowrap"}}>
                                    {name}
                                    <MessagingErrors
                                        errors={errorsAt(errors, config, {
                                            area: EMessagingErrorArea.ELECTION,
                                            electionId: election.id,
                                        })}
                                        electionName={() => name}
                                    />
                                </TableCell>
                                {active.map((channel) => (
                                    <TableCell key={channel} align="center" padding="checkbox">
                                        <Checkbox
                                            size="small"
                                            checked={offered.includes(channel)}
                                            disabled={!canEdit}
                                            onChange={() =>
                                                onChange(
                                                    toggleElectionChannel(
                                                        config,
                                                        election.id,
                                                        channel
                                                    )
                                                )
                                            }
                                            slotProps={{
                                                input: {
                                                    "aria-label": t("messagingEvent.posts.cell", {
                                                        post: name,
                                                        channel: t(`messaging.channel.${channel}`),
                                                    }),
                                                },
                                            }}
                                        />
                                    </TableCell>
                                ))}
                            </TableRow>
                        )
                    })}
                </TableBody>
            </Table>
            {matching.length > POSTS_SHOWN ? (
                <Typography variant="caption" sx={{color: "text.secondary"}}>
                    {t("messagingEvent.posts.showing", {
                        shown: POSTS_SHOWN,
                        total: matching.length,
                    })}
                </Typography>
            ) : null}
            <MessagingErrors errors={unknown} />
        </>
    )
}

export interface MessagingReplyTextProps {
    config: IEventMessagingConfig
    languages: string[]
    canEdit: boolean
    onChange: TOnChange
}

/** Automatic reply to incoming messages, per language. */
export const MessagingReplyText: React.FC<MessagingReplyTextProps> = ({
    config,
    languages,
    canEdit,
    onChange,
}) => {
    const {t} = useTranslation()
    return (
        <>
            <Helper>{t("messagingEvent.reply.help")}</Helper>
            {languages.map((language) => (
                <TextField
                    key={language}
                    multiline
                    minRows={2}
                    fullWidth
                    disabled={!canEdit}
                    label={t("messagingEvent.reply.label", {language})}
                    value={config.reply_text[language] ?? ""}
                    onChange={(event) =>
                        onChange(setReplyText(config, language, event.target.value))
                    }
                />
            ))}
        </>
    )
}

export interface MessagingDeliveryStatusProps {
    config: IEventMessagingConfig
    accounts: IMessagingAccount[]
    stats?: IDeliveryStatsResponse | null
    loading?: boolean
}

/** Outbound messages per channel and state. Without receipts, delivery is unavailable, not 0. */
export const MessagingDeliveryStatus: React.FC<MessagingDeliveryStatusProps> = ({
    config,
    accounts,
    stats,
    loading,
}) => {
    const {t} = useTranslation()
    const perChannel = useMemo(() => parseDeliveryStats(stats), [stats])
    const channels = config.channels.map((entry) => entry.channel)
    if (!channels.length) {
        return <Helper>{t("messagingEvent.delivery.empty")}</Helper>
    }
    return (
        <>
            <Helper>{t("messagingEvent.delivery.help")}</Helper>
            <Table size="small" aria-label={t("messagingEvent.sections.delivery")}>
                <TableHead>
                    <TableRow>
                        <TableCell>{t("messagingEvent.column.channel")}</TableCell>
                        {MESSAGE_ATTEMPT_STATES.map((state) => (
                            <TableCell
                                key={state}
                                align="right"
                                title={t(`messaging.stateHelp.${state}`)}
                            >
                                {t(`messaging.state.${state}`)}
                            </TableCell>
                        ))}
                    </TableRow>
                </TableHead>
                <TableBody>
                    {channels.map((channel) => {
                        const account = accountOf(config, accounts, channel)
                        const feedback = account
                            ? (accountCapabilities(account)?.delivery_feedback ??
                              EDeliveryFeedback.UNAVAILABLE)
                            : EDeliveryFeedback.UNAVAILABLE
                        const summary = deliverySummary(perChannel[channel], feedback)
                        const value = (state: EMessageAttemptState): string => {
                            if (loading) return "–"
                            switch (state) {
                                case EMessageAttemptState.QUEUED:
                                    return summary.queued.toLocaleString()
                                case EMessageAttemptState.ACCEPTED:
                                    return summary.accepted.toLocaleString()
                                case EMessageAttemptState.DELIVERED:
                                    return summary.delivered === null
                                        ? t("messaging.deliveryUnavailable")
                                        : summary.delivered.toLocaleString()
                                case EMessageAttemptState.FAILED:
                                    return summary.failed.toLocaleString()
                                case EMessageAttemptState.UNKNOWN:
                                    return summary.unknown.toLocaleString()
                            }
                        }
                        return (
                            <TableRow key={channel} data-channel={channel}>
                                <TableCell>
                                    <ChannelLabel
                                        channel={channel}
                                        label={t(`messaging.channel.${channel}`)}
                                    />
                                </TableCell>
                                {MESSAGE_ATTEMPT_STATES.map((state) => (
                                    <TableCell key={state} align="right">
                                        {value(state)}
                                    </TableCell>
                                ))}
                            </TableRow>
                        )
                    })}
                </TableBody>
            </Table>
        </>
    )
}

const sameErrors = (a: IMessagingConfigError, b: IMessagingConfigError) =>
    JSON.stringify(a) === JSON.stringify(b)

/** Election event > Messaging: channels, accounts and purposes of the event. */
export const EditElectionEventMessaging: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const notify = useNotify()
    const refresh = useRefresh()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canEdit = authContext.isAuthorized(true, tenantId, IPermissions.MESSAGING_CONFIG_WRITE)
    const canReadMessages = authContext.isAuthorized(true, tenantId, IPermissions.NOTIFICATION_READ)
    const {accounts, loading: accountsLoading} = useMessagingAccounts()
    const {data: elections, isLoading: electionsLoading} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            filter: {election_event_id: record?.id, tenant_id: tenantId},
            pagination: {page: 1, perPage: 1000},
            sort: {field: "created_at", order: "ASC"},
        },
        {enabled: !!record?.id}
    )
    const {data: stats, loading: statsLoading} = useQuery<IDeliveryStatsResponse>(
        GET_MESSAGE_DELIVERY_STATS,
        {
            variables: {tenantId, electionEventId: record?.id},
            skip: !record?.id || !tenantId,
            context: canReadMessages
                ? {headers: {"x-hasura-role": IPermissions.NOTIFICATION_READ}}
                : undefined,
        }
    )
    const [updateConfig, {loading: saving}] = useMutation<IUpdateEventMessagingConfigResult>(
        UPDATE_EVENT_MESSAGING_CONFIG,
        {context: {headers: {"x-hasura-role": IPermissions.MESSAGING_CONFIG_WRITE}}}
    )

    const stored = record?.annotations?.[MESSAGING_CONFIG_ANNOTATION]
    const initial = useMemo(() => parseEventMessagingConfig(stored), [stored])
    const [config, setConfig] = useState<IEventMessagingConfig>(initial)
    const [dirty, setDirty] = useState(false)
    const [serverErrors, setServerErrors] = useState<IMessagingConfigError[]>([])
    const [saveTried, setSaveTried] = useState(false)
    useEffect(() => {
        setConfig(initial)
        setDirty(false)
        setSaveTried(false)
    }, [initial])
    const {data: templates} = useGetList<Sequent_Backend_Template>(
        "sequent_backend_template",
        {
            filter: {tenant_id: tenantId},
            pagination: {page: 1, perPage: 1000},
            sort: {field: "alias", order: "ASC"},
        },
        {enabled: !!tenantId}
    )
    const templateAliases = useMemo(
        () =>
            Array.from(
                new Set(
                    (templates ?? [])
                        .map((template) => template.alias)
                        .filter((alias): alias is string => !!alias)
                )
            ),
        [templates]
    )

    const languages: string[] = record?.presentation?.language_conf?.enabled_language_codes?.length
        ? record.presentation.language_conf.enabled_language_codes
        : ["en"]
    const aliasRenderer = useAliasRenderer()
    const electionList = useMemo(
        () =>
            (elections ?? []).map(
                (election): IMessagingElection => ({id: election.id, name: aliasRenderer(election)})
            ),
        [elections, aliasRenderer]
    )
    const clientErrors = useMemo(
        () =>
            accountsLoading || electionsLoading
                ? []
                : validateEventMessagingConfig(
                      preparedConfig(config),
                      tenantId ?? "",
                      accounts.map(accountSummary),
                      electionList.map((election) => election.id)
                  ),
        [config, tenantId, accounts, electionList, accountsLoading, electionsLoading]
    )
    const errors = [
        ...serverErrors,
        ...clientErrors.filter((error) => !serverErrors.some((other) => sameErrors(error, other))),
    ]
    const general = errorsAt(errors, config, {area: EMessagingErrorArea.GENERAL})

    const onChange: TOnChange = (next) => {
        setConfig(next)
        setDirty(true)
        setServerErrors([])
    }

    const incomplete = saveTried ? incompleteBindings(config) : []

    const save = async () => {
        if (!record?.id) return
        setSaveTried(true)
        if (incompleteBindings(config).length) {
            notify(t("messagingEvent.saveRejected"), {type: "error"})
            return
        }
        try {
            const {data, errors: graphqlErrors} = await updateConfig({
                variables: {electionEventId: record.id, config: preparedConfig(config)},
            })
            if (graphqlErrors?.length || !data?.update_event_messaging_config) {
                notify(t("messagingEvent.saveError"), {type: "error"})
                return
            }
            const rejected = parseMessagingConfigErrors(data.update_event_messaging_config.errors)
            if (rejected.length) {
                setServerErrors(rejected)
                notify(t("messagingEvent.saveRejected"), {type: "error"})
                return
            }
            setDirty(false)
            notify(t("messagingEvent.saved"), {type: "success"})
            refresh()
        } catch (error) {
            console.error(error)
            notify(t("messagingEvent.saveError"), {type: "error"})
        }
    }

    if (!record) return null

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2, padding: 2, paddingBottom: 4}}>
            <Helper>{t("messagingEvent.intro")}</Helper>
            {!canEdit ? <Alert severity="info">{t("messagingEvent.readOnly")}</Alert> : null}
            <MessagingErrors errors={general} />
            <Section title={t("messagingEvent.sections.channels")} id="messaging-channels">
                <MessagingChannels
                    config={config}
                    accounts={accounts}
                    errors={errors}
                    canEdit={canEdit}
                    onChange={onChange}
                />
            </Section>
            <Section title={t("messagingEvent.sections.templates")} id="messaging-templates">
                <MessagingTemplateBindings
                    config={config}
                    accounts={accounts}
                    languages={languages}
                    templateAliases={templateAliases}
                    errors={errors}
                    incomplete={incomplete}
                    canEdit={canEdit}
                    onChange={onChange}
                />
            </Section>
            <Section title={t("messagingEvent.sections.fallback")} id="messaging-fallback">
                <MessagingFallbackOrder
                    config={config}
                    errors={errors}
                    canEdit={canEdit}
                    onChange={onChange}
                />
            </Section>
            <Section
                title={t("messagingEvent.sections.postsCount", {count: electionList.length})}
                id="messaging-posts"
            >
                <MessagingElectionChannels
                    config={config}
                    elections={electionList}
                    errors={errors}
                    canEdit={canEdit}
                    onChange={onChange}
                />
            </Section>
            <Section title={t("messagingEvent.sections.reply")} id="messaging-reply">
                <MessagingReplyText
                    config={config}
                    languages={languages}
                    canEdit={canEdit}
                    onChange={onChange}
                />
            </Section>
            <Section title={t("messagingEvent.sections.delivery")} id="messaging-delivery">
                <MessagingDeliveryStatus
                    config={config}
                    accounts={accounts}
                    stats={stats}
                    loading={statsLoading}
                />
            </Section>
            {canEdit ? (
                <Box
                    sx={{
                        display: "flex",
                        flexDirection: "column",
                        gap: 1,
                        alignItems: "flex-start",
                    }}
                >
                    {dirty ? <Alert severity="info">{t("messagingEvent.savingNote")}</Alert> : null}
                    <Button variant="contained" disabled={!dirty || saving} onClick={save}>
                        {t("messagingEvent.save")}
                    </Button>
                </Box>
            ) : null}
        </Box>
    )
}

export default EditElectionEventMessaging
