// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useGetList, useNotify, useRecordContext} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {useLocation, useNavigate} from "react-router-dom"
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
    Link,
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
import {ChannelLabel} from "@sequentech/ui-essentials"
import {
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Template,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {useMessagingAccounts} from "@/hooks/useMessagingAccounts"
import {
    useEventMessagingConfig,
    useEventMessagingConfigAuthor,
} from "@/hooks/useEventMessagingConfig"
import {MIGRATED_INVALID_BY} from "@/queries/GetEventMessagingConfig"
import {roleContext, useMessagingReadRole} from "@/hooks/useMessagingReadRole"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {
    GET_MESSAGING_EVENT_STATS,
    IGetMessagingEventStatsResult,
} from "@/queries/GetMessagingEventStats"
import {UPDATE_EVENT_MESSAGING_CONFIG} from "@/queries/UpdateEventMessagingConfig"
import {IVoterCounts} from "@/queries/GetMessagingVoterCounts"
import {useMessagingVoterCounts} from "@/hooks/useMessagingVoterCounts"
import {useFormatCount} from "@/services/messagingFormat"
import {MessagingVoterCountStatus} from "@/components/MessagingVoterCountStatus"
import {IPermissions} from "@/types/keycloak"
import {
    EDeliveryFeedback,
    EMessageAttemptState,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    IChannelMessageStats,
    IEventMessagingConfig,
    IMessagingAccount,
    IMessagingConfigError,
    ITemplateBinding,
    KEYCLOAK_NOTICE_MESSAGE_KEYS,
    KEYCLOAK_OTP_MESSAGE_KEYS,
    MESSAGE_CHANNELS,
    MESSAGE_PURPOSES,
} from "@/types/messaging"
import {
    accountCapabilities,
    accountDeliveryFeedback,
    accountIdentity,
    accountSummary,
    deliverySummary,
    purposeReadiness,
    validateEventMessagingConfig,
} from "@/services/messaging"
import {
    EBindingApproval,
    EMessagingErrorArea,
    IBindingPosition,
    IMessagingErrorLocation,
    activeChannels,
    addTemplateBinding,
    bindingApproval,
    electionChannelsOf,
    errorsAt,
    incompleteBindings,
    parseMessagingConfigErrors,
    preparedConfig,
    removeTemplateBinding,
    noChannelStats,
    setChannelAccount,
    statsByChannel,
    templateChannels,
    toggleElectionChannel,
    togglePurpose,
    updateTemplateBinding,
} from "./eventMessagingConfig"
import {logsFilterSearch} from "@/components/logs/messagingLogFilters"
import {reasonText} from "@/services/messagingReasons"

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
    <Typography variant="body2" sx={{color: "text.secondary"}}>
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
                                {entry?.out_of_window === EOutOfWindowPolicy.UTILITY_MESSAGES ? (
                                    <Typography
                                        variant="caption"
                                        sx={{display: "block", color: "text.secondary", mt: 1}}
                                    >
                                        {t("messagingEvent.outOfWindow.utility", {
                                            channel: channelName,
                                        })}
                                    </Typography>
                                ) : null}
                                <MessagingErrors
                                    errors={at({area: EMessagingErrorArea.CHANNEL, channel})}
                                />
                            </TableCell>
                            {MESSAGE_PURPOSES.map((purpose) => {
                                const enabled = entry?.purposes.includes(purpose) ?? false
                                const blockers = account
                                    ? purposeReadiness(
                                          account.provider,
                                          capabilities,
                                          account.provider_approval,
                                          account.status,
                                          purpose,
                                          null,
                                          accountIdentity(account.sender)
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
                                        {!account && options.length ? (
                                            <Typography
                                                variant="caption"
                                                sx={{display: "block", color: "text.secondary"}}
                                            >
                                                {t("messagingEvent.chooseAccount")}
                                            </Typography>
                                        ) : null}
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

const APPROVAL_COLOR: Record<EBindingApproval, "success" | "default"> = {
    [EBindingApproval.APPROVED]: "success",
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

export interface MessagingDeliveryStatusProps {
    config: IEventMessagingConfig
    accounts: IMessagingAccount[]
    stats?: IChannelMessageStats[] | null
    loading?: boolean
    /** The counts could not be read: the cells say so instead of showing 0. */
    unavailable?: boolean
    /** The event's voters per channel, as last counted; `undefined` while loading. */
    voterCounts?: IVoterCounts | null
    /** The voter counts could not be read: their cells show "–". */
    votersUnavailable?: boolean
    /** Asks for the voters to be counted again; absent where it can't be asked. */
    onRecount?: () => void
    recounting?: boolean
    /**
     * Opens Logs filtered to the channel's failed messages (`null`: those of
     * voters with no contact channel); absent without access to Logs.
     */
    onShowFailed?: (channel: EMessageChannel | null) => void
}

/**
 * Per channel: voters whose channel it is (as last counted, with when), messages
 * sent in the last 24 hours, and messages by the state of their latest
 * attempt. The rows are the configured channels, and every other channel some
 * voter or message is on (said not to be offered by the event: its notices are
 * refused), then the voters with no contact channel. Without receipts,
 * delivery is unavailable, never 0. Unknown for more than 24 hours is shown
 * apart as unresolved: delivery couldn't be confirmed, which isn't a failure.
 */
export const MessagingDeliveryStatus: React.FC<MessagingDeliveryStatusProps> = ({
    config,
    accounts,
    stats,
    loading,
    unavailable,
    voterCounts,
    votersUnavailable,
    onRecount,
    recounting,
    onShowFailed,
}) => {
    const {t} = useTranslation()
    const formatCount = useFormatCount()
    const perChannel = useMemo(() => statsByChannel(stats), [stats])
    const noChannel = useMemo(() => noChannelStats(stats), [stats])
    const votersOn = useMemo(() => {
        const byChannel = new Map<EMessageChannel | null, number>()
        for (const entry of voterCounts?.channels ?? []) {
            byChannel.set(entry.channel ?? null, entry.voters)
        }
        return byChannel
    }, [voterCounts])
    const configured = config.channels.map((entry) => entry.channel)
    const channels = MESSAGE_CHANNELS.filter(
        (channel) =>
            configured.includes(channel) || !!perChannel[channel] || !!votersOn.get(channel)
    )
    const noChannelVoters = votersOn.get(null) ?? 0
    const showNoChannel = !!noChannel || noChannelVoters > 0
    const count = (value: number | null): string => {
        if (loading || unavailable || value === null) return "–"
        return formatCount(value)
    }
    // Never counted, still loading or unreadable: "–", never 0.
    const voters = (channel: EMessageChannel | null): string =>
        !voterCounts || votersUnavailable || !voterCounts.status.computed_at
            ? "–"
            : formatCount(votersOn.get(channel) ?? 0)
    const countStatus = (
        <MessagingVoterCountStatus
            status={voterCounts?.status}
            onRecount={onRecount}
            recounting={recounting}
        />
    )
    if (!channels.length && !showNoChannel) {
        return (
            <>
                <Helper>{t("messagingEvent.delivery.empty")}</Helper>
                {countStatus}
            </>
        )
    }
    return (
        <>
            <Helper>{t("messagingEvent.delivery.help")}</Helper>
            {countStatus}
            {votersUnavailable ? (
                <Typography variant="caption" role="status" sx={{color: "text.secondary"}}>
                    {t("messaging.voterCount.loadError")}
                </Typography>
            ) : null}
            {unavailable ? (
                <Typography variant="caption" role="status" sx={{color: "text.secondary"}}>
                    {t("messagingEvent.delivery.loadError")}
                </Typography>
            ) : null}
            <Table size="small" aria-label={t("messagingEvent.sections.delivery")}>
                <TableHead>
                    <TableRow>
                        <TableCell>{t("messagingEvent.column.channel")}</TableCell>
                        <TableCell align="right">{t("messagingEvent.column.voters")}</TableCell>
                        <TableCell align="right" title={t("messagingEvent.column.sent24hHelp")}>
                            {t("messagingEvent.column.sent24h")}
                        </TableCell>
                        {[
                            EMessageAttemptState.QUEUED,
                            EMessageAttemptState.ACCEPTED,
                            EMessageAttemptState.DELIVERED,
                            EMessageAttemptState.FAILED,
                            EMessageAttemptState.UNKNOWN,
                        ].map((state) => (
                            <TableCell
                                key={state}
                                align="right"
                                title={t(`messaging.stateHelp.${state}`)}
                            >
                                {t(`messaging.state.${state}`)}
                            </TableCell>
                        ))}
                        <TableCell align="right" title={t("messaging.stateHelp.UNRESOLVED")}>
                            {t("messaging.state.UNRESOLVED")}
                        </TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {channels.map((channel) => {
                        const offered = configured.includes(channel)
                        const account = accountOf(config, accounts, channel)
                        // A channel the event doesn't configure sends nothing: its
                        // messages are the refused ones, counted as they are.
                        const {feedback, reason} = offered
                            ? accountDeliveryFeedback(account)
                            : {feedback: EDeliveryFeedback.PROVIDER_RECEIPTS, reason: undefined}
                        const summary = deliverySummary(perChannel[channel], feedback)
                        const channelName = t(`messaging.channel.${channel}`)
                        const failed = count(summary.failed)
                        return (
                            <TableRow key={channel} data-channel={channel}>
                                <TableCell>
                                    <ChannelLabel channel={channel} label={channelName} />
                                    {offered ? null : (
                                        <Typography
                                            variant="caption"
                                            component="div"
                                            sx={{color: "text.secondary"}}
                                        >
                                            {t("messagingEvent.delivery.notConfigured")}
                                        </Typography>
                                    )}
                                </TableCell>
                                <TableCell align="right">{voters(channel)}</TableCell>
                                <TableCell align="right">{count(summary.sentLast24h)}</TableCell>
                                <TableCell align="right">{count(summary.queued)}</TableCell>
                                <TableCell align="right">{count(summary.accepted)}</TableCell>
                                <TableCell
                                    align="right"
                                    title={
                                        summary.delivered === null
                                            ? reason
                                                ? reasonText(t, reason)
                                                : t("messagingEvent.delivery.noReceipts")
                                            : undefined
                                    }
                                >
                                    {summary.delivered === null
                                        ? t("messaging.deliveryUnavailable")
                                        : count(summary.delivered)}
                                </TableCell>
                                <TableCell align="right">
                                    {onShowFailed && !loading && !unavailable ? (
                                        <Link
                                            component="button"
                                            type="button"
                                            variant="body2"
                                            onClick={() => onShowFailed(channel)}
                                            aria-label={t("messagingEvent.delivery.showFailed", {
                                                channel: channelName,
                                            })}
                                        >
                                            {failed}
                                        </Link>
                                    ) : (
                                        failed
                                    )}
                                </TableCell>
                                <TableCell align="right">{count(summary.unknown)}</TableCell>
                                <TableCell align="right">{count(summary.unresolved)}</TableCell>
                            </TableRow>
                        )
                    })}
                    {showNoChannel ? (
                        // Voters with no contact channel: their failures are never
                        // counted under a channel, so they show on their own row.
                        <TableRow data-channel="NONE">
                            <TableCell title={t("messagingEvent.delivery.noChannelHelp")}>
                                {t("messaging.reason.NO_CONTACT_CHANNEL")}
                            </TableCell>
                            <TableCell align="right">{voters(null)}</TableCell>
                            <TableCell align="right">–</TableCell>
                            <TableCell align="right">–</TableCell>
                            <TableCell align="right">–</TableCell>
                            <TableCell align="right">–</TableCell>
                            <TableCell align="right">
                                {onShowFailed && !loading && !unavailable ? (
                                    <Link
                                        component="button"
                                        type="button"
                                        variant="body2"
                                        onClick={() => onShowFailed(null)}
                                        aria-label={t(
                                            "messagingEvent.delivery.showFailedNoChannel"
                                        )}
                                    >
                                        {count(noChannel?.failed ?? 0)}
                                    </Link>
                                ) : (
                                    count(noChannel?.failed ?? 0)
                                )}
                            </TableCell>
                            <TableCell align="right">–</TableCell>
                            <TableCell align="right">–</TableCell>
                        </TableRow>
                    ) : null}
                </TableBody>
            </Table>
        </>
    )
}

/**
 * A configuration copied from the old storage that didn't pass validation is
 * marked by the backend: it says so, and that saving again (after fixing what
 * the tab shows) stores it as a checked configuration.
 */
export const MigratedConfigNotice: React.FC<{updatedBy?: string | null}> = ({updatedBy}) => {
    const {t} = useTranslation()
    if (updatedBy !== MIGRATED_INVALID_BY) return null
    return (
        <Alert severity="warning">
            <strong>{t("messagingEvent.needsSavingAgain")}</strong>{" "}
            {t("messagingEvent.needsSavingAgainHelp")}
        </Alert>
    )
}

const sameErrors = (a: IMessagingConfigError, b: IMessagingConfigError) =>
    JSON.stringify(a) === JSON.stringify(b)

/** Election event > Messaging: channels, accounts and purposes of the event. */
export const EditElectionEventMessaging: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const notify = useNotify()
    const navigate = useNavigate()
    const location = useLocation()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const readRole = useMessagingReadRole()
    // Saving without having read the configuration would overwrite it blindly.
    const canEdit =
        !!readRole && authContext.isAuthorized(true, tenantId, IPermissions.MESSAGING_CONFIG_WRITE)
    // The Failed count links to the event's Logs tab, filtered to that channel's failures.
    const canReadLogs =
        authContext.isAuthorized(true, tenantId, IPermissions.LOGS_READ) &&
        authContext.isAuthorized(true, tenantId, IPermissions.ELECTION_EVENT_LOGS_TAB)
    const {accounts, loading: accountsLoading} = useMessagingAccounts({callbackStatus: true})
    const {data: elections, isLoading: electionsLoading} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            filter: {election_event_id: record?.id, tenant_id: tenantId},
            pagination: {page: 1, perPage: 1000},
            sort: {field: "created_at", order: "ASC"},
        },
        {enabled: !!record?.id}
    )
    const {
        data: stats,
        loading: statsLoading,
        error: statsError,
    } = useQuery<IGetMessagingEventStatsResult>(GET_MESSAGING_EVENT_STATS, {
        variables: {electionEventId: record?.id},
        skip: !record?.id || !readRole,
        context: roleContext(readRole),
    })
    const {
        counts: voterCounts,
        unavailable: votersUnavailable,
        recount,
        recounting,
        refetch: refetchVoterCounts,
    } = useMessagingVoterCounts(record?.id)
    const {
        config: initial,
        loading: configLoading,
        refetch: refetchConfig,
    } = useEventMessagingConfig(record?.id)
    const {updatedBy, refetch: refetchAuthor} = useEventMessagingConfigAuthor(record?.id)
    const [updateConfig, {loading: saving}] = useMutation<IUpdateEventMessagingConfigResult>(
        UPDATE_EVENT_MESSAGING_CONFIG,
        {context: {headers: {"x-hasura-role": IPermissions.MESSAGING_CONFIG_WRITE}}}
    )

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
            accountsLoading || electionsLoading || configLoading
                ? []
                : validateEventMessagingConfig(
                      preparedConfig(config),
                      tenantId ?? "",
                      accounts.map(accountSummary),
                      electionList.map((election) => election.id)
                  ),
        [config, tenantId, accounts, electionList, accountsLoading, electionsLoading, configLoading]
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
            await refetchConfig()
            refetchAuthor().catch(() => undefined)
            // The save asked for a recount: show it on its way.
            refetchVoterCounts().catch(() => undefined)
        } catch (error) {
            console.error(error)
            notify(t("messagingEvent.saveError"), {type: "error"})
        }
    }

    const showFailed = (channel: EMessageChannel | null) =>
        navigate({pathname: location.pathname, search: logsFilterSearch(channel)})

    if (!record) return null

    if (!readRole) {
        return (
            <Box sx={{padding: 2}}>
                <Alert severity="info">{t("messagingEvent.noReadPermission")}</Alert>
            </Box>
        )
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2, padding: 2, paddingBottom: 4}}>
            <Helper>{t("messagingEvent.intro")}</Helper>
            {!canEdit ? <Alert severity="info">{t("messagingEvent.readOnly")}</Alert> : null}
            <MigratedConfigNotice updatedBy={updatedBy} />
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
            <Section title={t("messagingEvent.sections.delivery")} id="messaging-delivery">
                <MessagingDeliveryStatus
                    config={config}
                    accounts={accounts}
                    stats={stats?.messaging_event_stats?.channels}
                    loading={statsLoading}
                    unavailable={!!statsError}
                    voterCounts={voterCounts}
                    votersUnavailable={votersUnavailable}
                    onRecount={recount}
                    recounting={recounting}
                    onShowFailed={canReadLogs ? showFailed : undefined}
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
