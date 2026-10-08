// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {styled} from "@mui/material/styles"
import {CircularProgress, Typography, Menu, MenuItem} from "@mui/material"
import {
    Publish,
    RotateLeft,
    PlayCircle,
    PauseCircle,
    StopCircle,
    PlaylistAddCheck,
} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {
    Button,
    FilterButton,
    SelectColumnsButton,
    useGetList,
    useRecordContext,
    Identifier,
} from "react-admin"

import {EPublishActionsType, EPublishType} from "./EPublishType"
import {PublishStatus, ElectionEventStatus, nextStatus} from "./EPublishStatus"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import SvgIcon from "@mui/material/SvgIcon"
import {EPublishActions} from "@/types/publishActions"

import {VotingStatusChannel} from "@/gql/graphql"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {
    EBallotBoxSealPolicy,
    EGracePeriodPolicy,
    EInitializeReportPolicy,
    EVotingPeriodEnd,
    EVotingStatus,
    IElectionPresentation,
    IElectionStatus,
    IChannelButtonInfo,
} from "@sequentech/ui-core"
import {usePublishPermissions} from "./usePublishPermissions"
import PublishExport from "./PublishExport"
import {useEventPresentation} from "@/hooks/useZonedFormat"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {intlLanguage} from "@/hooks/useZonedTime"
import {
    electionSealChannels,
    type ISealChannel,
    type IStopSealOutcome,
    eventStartChannels,
    keptClosedChannels,
    neverOpened,
    onlineRan,
    sealProgress,
    stopSealOutcome,
} from "@/services/sealOnStop"
import {useSealReadRole} from "@/hooks/useSealReadRole"
import {useQuery} from "@apollo/client"
import {GET_BALLOT_BOX_SEALS} from "@/queries/GetBallotBoxSeals"
import type {GetBallotBoxSealsQuery, GetBallotBoxSealsQueryVariables} from "@/types/ballotBoxSeal"

type SvgIconComponent = typeof SvgIcon

const PublishActionsStyled = {
    Container: styled("div")`
        display: flex;
        margin-bottom: 8px;
        justify-content: flex-end;
        width: 100%;
    `,
}

export const StyledStatusButton = styled(Button)`
    &.MuiButtonBase-root {
        line-height: 1 !important;
    }

    :disabled {
        color: #ccc;
        cursor: not-allowed;
        background-color: #eee;
    }
`

export const StyledMenuItem = styled(MenuItem)`
    &.MuiMenuItem-root {
        display: flex;
        gap: 8px;
    }
`

export type PublishActionsProps = {
    ballotPublicationId?: string | Identifier | null
    data?: any
    status: PublishStatus
    publishType: EPublishType.Election | EPublishType.Event
    electionStatus: IElectionStatus | null
    electionPresentation: IElectionPresentation | null
    kioskModeEnabled: IChannelButtonInfo
    onlineModeEnabled: IChannelButtonInfo
    earlyVotingEnabled: IChannelButtonInfo
    telephoneVotingEnabled: IChannelButtonInfo
    changingStatus: boolean
    onPublish?: () => void
    onGenerate: () => void
    onChangeStatus?: (status: ElectionEventStatus, votingChannel?: VotingStatusChannel[]) => void
    /** Initializes voting at the Post (its initialization report); election level only. */
    onInitialize?: () => void
    initializing?: boolean
    perCountryInitialization?: boolean
    initializationReportPolicy?: EInitializeReportPolicy
    type: EPublishActionsType.List | EPublishActionsType.Generate
}

export const PublishActions: React.FC<PublishActionsProps> = ({
    ballotPublicationId,
    type,
    status,
    publishType,
    kioskModeEnabled,
    onlineModeEnabled,
    earlyVotingEnabled,
    telephoneVotingEnabled,
    electionStatus,
    electionPresentation,
    changingStatus,
    onGenerate,
    onPublish = () => null,
    onChangeStatus = () => null,
    onInitialize,
    initializing = false,
    perCountryInitialization = false,
    initializationReportPolicy,
    data,
}) => {
    const {t, i18n} = useTranslation()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const {isGoldUser, reauthWithGold} = authContext
    const canWrite = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_WRITE)
    const record = useRecordContext<Sequent_Backend_Election>()
    const isVotingPeriodEndDisallowed =
        electionPresentation?.voting_period_end == EVotingPeriodEnd.DISALLOWED
    const canChangeStatus = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_STATE_WRITE
    )
    const requiredInitialization =
        (initializationReportPolicy ?? record?.presentation?.initialization_report_policy) ===
        EInitializeReportPolicy.REQUIRED
    // The trusted published requirement also controls ceremony availability.
    const canInitialize =
        publishType === EPublishType.Election &&
        !!onInitialize &&
        requiredInitialization &&
        authContext.isAuthorized(true, tenantId, IPermissions.ADMIN_CEREMONY)
    // const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const eventPresentation = useEventPresentation()
    const aliasRenderer = useAliasRenderer()
    const [showDialog, setShowDialog] = useState(false)
    const [dialogText, setDialogText] = useState("")
    const [currentCallback, setCurrentCallback] = useState<any>(null)
    const [startAnchorEl, setStartAnchorEl] = useState<null | HTMLElement>(null)
    const startMenuOpen = Boolean(startAnchorEl)
    const [pauseAnchorEl, setPauseAnchorEl] = useState<null | HTMLElement>(null)
    const pauseMenuOpen = Boolean(pauseAnchorEl)
    const [stopAnchorEl, setStopAnchorEl] = useState<null | HTMLElement>(null)
    const stopMenuOpen = Boolean(stopAnchorEl)

    const {
        canPublishRegenerate,
        canPublishStartVoting,
        canPublishPauseVoting,
        canPublishStopVoting,
        canPublishChanges,
        showPublishColumns,
        showPublishFilters,
    } = usePublishPermissions()

    const IconOrProgress = ({st, Icon}: {st: PublishStatus; Icon: SvgIconComponent}) => {
        return nextStatus(st) === status && status !== PublishStatus.Void ? (
            <CircularProgress size={16} />
        ) : (
            <Icon width={24} />
        )
    }

    const StatusButton = ({
        st,
        label,
        onClick,
        Icon,
        disabledStatus,
        disabled = false,
        className,
    }: {
        st: PublishStatus
        label: string
        onClick: () => void
        Icon: SvgIconComponent
        disabledStatus: Array<PublishStatus>
        disabled?: boolean
        className?: string
    }) => (
        <Button
            onClick={onClick}
            className={className}
            label={String(t(label))}
            style={
                changingStatus || disabledStatus?.includes(status)
                    ? {
                          overflow: "hidden",
                          color: "#ccc",
                          cursor: "not-allowed",
                          backgroundColor: "#eee",
                          padding: "6px 16px",
                      }
                    : {
                          overflow: "hidden",
                          padding: "6px 16px",
                      }
            }
            disabled={disabled || disabledStatus?.includes(status) || st === status + 0.1}
        >
            <IconOrProgress st={st} Icon={Icon} />
        </Button>
    )

    const openDialog = (dialogText: string) => {
        setDialogText(dialogText)
        setShowDialog(true)
    }

    // Handler for navigating after a re-authentication action.
    let reauthCallback = (
        baseUrl: URL,
        action: EPublishActions,
        voting_channels?: VotingStatusChannel[]
    ) => {
        // The event and election tabs select Publish by id; its position depends on permissions.
        baseUrl.searchParams.set("tabId", "publish")
        sessionStorage.setItem(action, "true")
        if (
            voting_channels &&
            (action === EPublishActions.PENDING_START_VOTING ||
                action === EPublishActions.PENDING_PAUSE_VOTING ||
                action === EPublishActions.PENDING_STOP_VOTING)
        ) {
            try {
                sessionStorage.setItem(`${action}_CHANNELS`, JSON.stringify(voting_channels))
            } catch (e) {
                console.warn("Could not persist selected channels for re-auth", e)
            }
        }
    }

    const sealsAtClose =
        eventPresentation?.ballot_box_seal_policy === EBallotBoxSealPolicy.SEAL_AT_CLOSE

    // The event's elections, for what an event-wide Start or Stop does to each.
    const {data: eventElections} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            filter: {election_event_id: record?.id, tenant_id: tenantId},
            pagination: {page: 1, perPage: 9999},
        },
        {enabled: sealsAtClose && publishType === EPublishType.Event && !!record?.id}
    )

    // Whether this election has seals: then it never opens again (S3).
    const sealRole = useSealReadRole()
    const {data: electionSeals} = useQuery<GetBallotBoxSealsQuery, GetBallotBoxSealsQueryVariables>(
        GET_BALLOT_BOX_SEALS,
        {
            variables: {
                electionEventId: String(record?.election_event_id ?? ""),
                electionIds: [String(record?.id ?? "")],
            },
            skip:
                !sealsAtClose || publishType !== EPublishType.Election || !record?.id || !sealRole,
            context: sealRole ? {headers: {"x-hasura-role": sealRole}} : undefined,
        }
    )
    const electionHasSeals = !!electionSeals?.sequent_backend_ballot_box_seal?.length

    // The seals of the event's elections, for what an event-wide Start keeps closed.
    const {data: eventSeals} = useQuery<GetBallotBoxSealsQuery, GetBallotBoxSealsQueryVariables>(
        GET_BALLOT_BOX_SEALS,
        {
            variables: {
                electionEventId: String(record?.id ?? ""),
                electionIds: (eventElections ?? []).map((election) => String(election.id)),
            },
            skip:
                !sealsAtClose ||
                publishType !== EPublishType.Event ||
                !eventElections?.length ||
                !sealRole,
            context: sealRole ? {headers: {"x-hasura-role": sealRole}} : undefined,
        }
    )
    const sealedElectionIds = new Set(
        (eventSeals?.sequent_backend_ballot_box_seal ?? []).map((seal) => String(seal.election_id))
    )

    /** The channels as Stop Voting sees them, with when each first opened (election level). */
    const sealChannels = (): ISealChannel[] => {
        const dates = (key: keyof IElectionStatus) =>
            (electionStatus?.[key] as {first_started_at?: string | null} | undefined)
                ?.first_started_at
        return [
            {
                channel: VotingStatusChannel.Online,
                info: onlineModeEnabled,
                firstStartedAt: dates("voting_period_dates"),
            },
            {
                channel: VotingStatusChannel.Kiosk,
                info: kioskModeEnabled,
                firstStartedAt: dates("kiosk_voting_period_dates"),
            },
            {
                channel: VotingStatusChannel.EarlyVoting,
                info: earlyVotingEnabled,
                firstStartedAt: dates("early_voting_period_dates"),
            },
            {
                channel: VotingStatusChannel.Telephone,
                info: telephoneVotingEnabled,
                firstStartedAt: dates("telephone_voting_period_dates"),
            },
        ].map(({channel, info, firstStartedAt}) => ({
            channel,
            enabled: !!info?.is_channel_enabled,
            status: info?.status ?? EVotingStatus.NOT_STARTED,
            firstStartedAt,
        }))
    }

    const listFormat = (items: string[]) =>
        new Intl.ListFormat(intlLanguage(i18n?.language ?? "en"), {
            style: "long",
            type: "conjunction",
        }).format(items)
    const channelNames = (channels: VotingStatusChannel[]) =>
        listFormat(channels.map((channel) => t(`publish.dialog.channel.${channel}`)))

    /**
     * The online grace period of an election in minutes, or 0. It applies only
     * when online voting ran (`deadline.rs`: ONLINE enabled and started).
     */
    const graceMinutes = (presentation: unknown, ran: boolean): number => {
        const value = (presentation ?? {}) as IElectionPresentation
        const secs =
            ran &&
            (value.grace_period_policy ?? EGracePeriodPolicy.NO_GRACE_PERIOD) !==
                EGracePeriodPolicy.NO_GRACE_PERIOD
                ? (value.grace_period_secs ?? 0)
                : 0
        return Math.ceil(secs / 60)
    }

    /** The sentence said before a Stop's seal text: channels it closes before they ever opened. */
    const stopPrefix = (stopping?: VotingStatusChannel[]): string => {
        const closingUnopened = sealChannels()
            .filter(
                (channel) =>
                    channel.enabled &&
                    channel.status === EVotingStatus.NOT_STARTED &&
                    !channel.firstStartedAt &&
                    (!stopping || stopping.includes(channel.channel))
            )
            .map(({channel}) => channel)
        return closingUnopened.length
            ? t("publish.dialog.stopNeverOpened", {
                  count: closingUnopened.length,
                  channels: channelNames(closingUnopened),
              })
            : ""
    }

    /**
     * Why a Stop doesn't seal the ballot boxes yet: the enabled channels still
     * open, and each channel the Post doesn't enable that is open or ran.
     */
    const sealHoldingText = (outcome: IStopSealOutcome): string =>
        [
            outcome.holding.length || !outcome.notEnabled.length
                ? t("publish.dialog.sealHolding", {
                      count: outcome.holding.length,
                      channels: channelNames(outcome.holding),
                  })
                : "",
            ...outcome.notEnabled.map((channel) =>
                t("publish.dialog.sealNotEnabled", {
                    channel: t(`publish.dialog.channel.${channel}`),
                })
            ),
        ]
            .filter(Boolean)
            .join(" ")

    /**
     * What Stop Voting says when the event seals its ballot boxes at close
     * (D1): the seal text only when this Stop finishes voting; otherwise the
     * usual text with the channels that still hold the seal. Null when the
     * event doesn't seal them.
     */
    const stopSealText = (stopping?: VotingStatusChannel[]): string | null => {
        if (!sealsAtClose) return null
        if (publishType === EPublishType.Event && eventElections) {
            return stopSealEventText(stopping)
        }
        const outcome = stopSealOutcome(sealChannels(), stopping)
        const prefix = stopPrefix(stopping)
        const text = outcome.seals
            ? stopSealBaseText()
            : t("publish.dialog.stopSealNotYet", {holding: sealHoldingText(outcome)})
        return [prefix, text].filter(Boolean).join(" ")
    }

    /** The seal text of a Stop that finishes voting at this election, with its grace period. */
    const stopSealBaseText = (): string => {
        if (publishType === EPublishType.Event) return t("publish.dialog.stopSealEvent")
        const name = aliasRenderer(record)
        const minutes = graceMinutes(electionPresentation, onlineRan(sealChannels()))
        return minutes > 0
            ? t("publish.dialog.stopSealGrace", {name, count: minutes})
            : t("publish.dialog.stopSeal", {name})
    }

    /**
     * An event-wide Stop, election by election with their own channels and
     * grace periods: which are sealed (now or after their grace period) and
     * which keep a channel that holds the seal.
     */
    const stopSealEventText = (stopping?: VotingStatusChannel[]): string => {
        const affected = (eventElections ?? [])
            .map((election) => {
                const channels = electionSealChannels(election)
                return {
                    election,
                    before: sealProgress(channels),
                    outcome: stopSealOutcome(channels, stopping),
                    grace: graceMinutes(election.presentation, onlineRan(channels)),
                    unopened: neverOpened(channels),
                }
            })
            // Elections whose voting had already finished don't change.
            .filter(({before}) => !before.finished)
        // An event-wide Stop closes the channel on every election, so a Post
        // that never opened is closed for good and gets empty sealed boxes.
        const unopened = affected.filter(({outcome, unopened}) => outcome.seals && unopened)
        const sealing = affected.filter(({outcome, unopened}) => outcome.seals && !unopened)
        // Posts kept by an enabled channel; those kept only by channels they
        // don't enable are named with those channels.
        const holding = affected.filter(
            ({outcome}) =>
                !outcome.seals && (outcome.holding.length > 0 || !outcome.notEnabled.length)
        )
        const notEnabled = affected.flatMap(({election, outcome}) =>
            outcome.seals
                ? []
                : outcome.notEnabled.map((channel) =>
                      t("publish.dialog.sealNotEnabledPost", {
                          post: aliasRenderer(election),
                          channel: t(`publish.dialog.channel.${channel}`),
                      })
                  )
        )
        const grace = Math.max(0, ...sealing.map((item) => item.grace))
        const names = (items: typeof affected) =>
            listFormat(items.map(({election}) => aliasRenderer(election)))
        const unopenedPart = unopened.length
            ? `${t("publish.dialog.stopNeverOpenedPosts", {
                  count: unopened.length,
                  names: names(unopened),
              })} `
            : ""
        if (!holding.length && !notEnabled.length) {
            return `${unopenedPart}${
                grace > 0
                    ? t("publish.dialog.stopSealEventGrace", {count: grace})
                    : t("publish.dialog.stopSealEvent")
            }`
        }
        const sealedPart = !sealing.length
            ? ""
            : grace > 0
              ? t("publish.dialog.sealedGracePart", {names: names(sealing), count: grace})
              : t("publish.dialog.sealedNowPart", {names: names(sealing)})
        const holdingPart = [
            holding.length
                ? t("publish.dialog.holdingEventPart", {
                      count: holding.length,
                      names: names(holding),
                  })
                : "",
            ...notEnabled,
        ]
            .filter(Boolean)
            .join(" ")
        return `${unopenedPart}${t("publish.dialog.stopSealEventSome", {
            sealed: sealedPart,
            holding: holdingPart,
        })}`
            .replace(/\s+/g, " ")
            .trim()
    }

    /**
     * With the seal at close, an event Start names, Post by Post, what the
     * server keeps closed (`election_event_status.rs`): per channel it
     * applies, a Post where that channel is CLOSED; and every channel of a
     * Post with ballot box seals.
     */
    const startSealNote = (starting?: VotingStatusChannel[]): string | null => {
        if (!sealsAtClose || publishType !== EPublishType.Event) return null
        if (!eventElections || (eventElections.length && !eventSeals)) {
            return t("publish.dialog.startSealNote")
        }
        const applied = eventStartChannels(sealChannels(), starting)
        const items = eventElections.flatMap((election) => {
            const post = aliasRenderer(election)
            if (sealedElectionIds.has(String(election.id))) {
                return [t("publish.dialog.startKeptSealed", {post})]
            }
            const kept = keptClosedChannels(electionSealChannels(election), applied)
            return kept.length
                ? [
                      t("publish.dialog.startKeptChannels", {
                          post,
                          count: kept.length,
                          channels: channelNames(kept),
                      }),
                  ]
                : []
        })
        return items.length
            ? t("publish.dialog.startSealNoteList", {items: items.join("; ")})
            : null
    }

    /**
     * Specific Handler for Start/Pause/Stop Buttons:
     * Incorporates re-authentication logic for actions that require Gold-level permissions.
     */
    const handleChangeVotingPeriod = (
        action: EPublishActions,
        status: ElectionEventStatus,
        voting_channels?: VotingStatusChannel[]
    ) => {
        const actionText =
            action === EPublishActions.PENDING_START_VOTING
                ? t(`publish.action.startVotingPeriod`)
                : action === EPublishActions.PENDING_STOP_VOTING
                  ? t(`publish.action.stopVotingPeriod`)
                  : t(`publish.action.pauseVotingPeriod`)

        // With the seal at close, the dialog says what this action does to the
        // ballot boxes, also before re-authenticating: whether a Stop seals
        // them, and that an event Start leaves closed elections closed.
        const sealText =
            action === EPublishActions.PENDING_STOP_VOTING
                ? stopSealText(voting_channels)
                : action === EPublishActions.PENDING_START_VOTING
                  ? startSealNote(voting_channels)
                  : null
        const dialogMessage = isGoldUser()
            ? action === EPublishActions.PENDING_START_VOTING
                ? [t("publish.dialog.startInfo"), sealText].filter(Boolean).join(" ")
                : action === EPublishActions.PENDING_STOP_VOTING
                  ? (sealText ?? t("publish.dialog.stopInfo"))
                  : t("publish.dialog.pauseInfo")
            : [sealText, t("publish.dialog.confirmation", {action: actionText})]
                  .filter(Boolean)
                  .join(" ")
        openDialog(dialogMessage)

        setCurrentCallback(() => async () => {
            try {
                if (!isGoldUser()) {
                    const baseUrl = new URL(window.location.href)
                    reauthCallback(baseUrl, action, voting_channels)
                    await reauthWithGold(baseUrl.toString())
                } else {
                    onChangeStatus(status, voting_channels)
                }
            } catch (error) {
                console.error("Re-authentication failed:", error)
            }
        })
    }

    /** Initializing voting asks for confirmation, then creates the initialization report. */
    const handleInitialize = () => {
        openDialog(t("publish.dialog.initializationInfo"))
        setCurrentCallback(() => async () => onInitialize?.())
    }

    const isInitializeDisabled = (): boolean =>
        changingStatus ||
        initializing ||
        (!!record?.initialization_report_generated && !perCountryInitialization) ||
        (electionStatus?.voting_status ?? EVotingStatus.NOT_STARTED) !== EVotingStatus.NOT_STARTED

    /**
     * Specific Handler for "Publish Changes" Button: Incorporates
     * re-authentication logic for actions that require Gold-level permissions.
     */
    const handlePublish = (is_generate: boolean) => {
        const actionText = t(`publish.action.publish`)
        const dialogMessage = isGoldUser()
            ? is_generate
                ? t("publish.dialog.info")
                : t("publish.dialog.publishInfo", {action: actionText})
            : t("publish.dialog.confirmation", {action: actionText})
        openDialog(dialogMessage)

        setCurrentCallback(() => async () => {
            try {
                if (!isGoldUser()) {
                    const baseUrl = new URL(window.location.href)
                    reauthCallback(baseUrl, EPublishActions.PENDING_PUBLISH_ACTION)
                    await reauthWithGold(baseUrl.toString())
                } else {
                    onGenerate()
                }
            } catch (error) {
                console.error("Re-authentication failed:", error)
                setDialogText(t("publish.dialog.errorReauth"))
                setShowDialog(true)
            }
        })
    }

    /**
     * Checks for any pending actions after the component mounts. If a pending
     * action is found, it executes the action and removes the flag. Except to
     * publish action, which is handled in the useEffect of the parent
     * component.
     */
    useEffect(() => {
        const executePendingActions = async () => {
            if (!record) {
                return
            }

            let isGold = isGoldUser()

            const pendingStart = sessionStorage.getItem(EPublishActions.PENDING_START_VOTING)
            if (pendingStart) {
                let selectedChannels: VotingStatusChannel[] | undefined = undefined
                try {
                    const channelsStr = sessionStorage.getItem(
                        `${EPublishActions.PENDING_START_VOTING}_CHANNELS`
                    )
                    if (channelsStr) {
                        selectedChannels = JSON.parse(channelsStr) as VotingStatusChannel[]
                    }
                } catch (e) {
                    console.warn("Could not restore selected channels after re-auth", e)
                }
                isGold && onChangeStatus(ElectionEventStatus.Open, selectedChannels)
                sessionStorage.removeItem(EPublishActions.PENDING_START_VOTING)
                sessionStorage.removeItem(`${EPublishActions.PENDING_START_VOTING}_CHANNELS`)
            }

            const pendingPause = sessionStorage.getItem(EPublishActions.PENDING_PAUSE_VOTING)
            if (pendingPause) {
                let selectedPauseChannels: VotingStatusChannel[] | undefined = undefined
                try {
                    const channelsStr = sessionStorage.getItem(
                        `${EPublishActions.PENDING_PAUSE_VOTING}_CHANNELS`
                    )
                    if (channelsStr) {
                        selectedPauseChannels = JSON.parse(channelsStr) as VotingStatusChannel[]
                    }
                } catch (e) {
                    console.warn("Could not restore selected pause channels after re-auth", e)
                }
                isGold && onChangeStatus(ElectionEventStatus.Paused, selectedPauseChannels)
                sessionStorage.removeItem(EPublishActions.PENDING_PAUSE_VOTING)
                sessionStorage.removeItem(`${EPublishActions.PENDING_PAUSE_VOTING}_CHANNELS`)
            }

            const pendingStop = sessionStorage.getItem(EPublishActions.PENDING_STOP_VOTING)
            if (pendingStop) {
                let selectedStopChannels: VotingStatusChannel[] | undefined = undefined
                try {
                    const channelsStr = sessionStorage.getItem(
                        `${EPublishActions.PENDING_STOP_VOTING}_CHANNELS`
                    )
                    if (channelsStr) {
                        selectedStopChannels = JSON.parse(channelsStr) as VotingStatusChannel[]
                    }
                } catch (e) {
                    console.warn("Could not restore selected stop channels after re-auth", e)
                }
                isGold && onChangeStatus(ElectionEventStatus.Closed, selectedStopChannels)
                sessionStorage.removeItem(EPublishActions.PENDING_STOP_VOTING)
                sessionStorage.removeItem(`${EPublishActions.PENDING_STOP_VOTING}_CHANNELS`)
            }

            const pendingStopKiosk = sessionStorage.getItem(
                EPublishActions.PENDING_STOP_KIOSK_ACTION
            )
            if (pendingStopKiosk) {
                isGold && onChangeStatus(ElectionEventStatus.Closed, [VotingStatusChannel.Kiosk])
                sessionStorage.removeItem(EPublishActions.PENDING_STOP_KIOSK_ACTION)
            }
        }

        executePendingActions()
    }, [isGoldUser, onChangeStatus, onGenerate, record])

    // Per-channel menu item disable logic
    const isStartChannelDisabled = (info?: IChannelButtonInfo): boolean => {
        const channelEnabled = info?.is_channel_enabled ?? false
        const st = info?.status
        return (
            !channelEnabled ||
            st === EVotingStatus.OPEN ||
            st === EVotingStatus.CLOSED ||
            // With Seal at close, an election with seals never opens again.
            electionHasSeals
        )
    }

    const isPauseChannelDisabled = (info?: IChannelButtonInfo): boolean => {
        const channelEnabled = info?.is_channel_enabled ?? false
        const st = info?.status
        return (
            !channelEnabled ||
            st === EVotingStatus.NOT_STARTED ||
            st === EVotingStatus.PAUSED ||
            st === EVotingStatus.CLOSED
        )
    }

    const isStopChannelDisabled = (info?: IChannelButtonInfo): boolean => {
        const channelEnabled = info?.is_channel_enabled ?? false
        const st = info?.status
        // With Seal at close, a channel that never started can be closed at
        // an election, so it no longer holds the seal (D1).
        const closesUnopened = sealsAtClose && publishType === EPublishType.Election
        return (
            !channelEnabled ||
            st === EVotingStatus.CLOSED ||
            (st === EVotingStatus.NOT_STARTED && !closesUnopened)
        )
    }

    const initializationReportNotGenerated = (): boolean => {
        return requiredInitialization && !record?.initialization_report_generated
    }

    // Encapsulated original disabled logic for each main action button
    const isStartButtonDisabled = (): boolean => {
        const allChannelsDisabled =
            isStartChannelDisabled(kioskModeEnabled) &&
            isStartChannelDisabled(onlineModeEnabled) &&
            isStartChannelDisabled(earlyVotingEnabled) &&
            isStartChannelDisabled(telephoneVotingEnabled)

        return (
            changingStatus ||
            [PublishStatus.GeneratedLoading].includes(status) ||
            allChannelsDisabled
        )
    }

    const isPauseButtonDisabled = (): boolean => {
        const allChannelsDisabled =
            isPauseChannelDisabled(kioskModeEnabled) &&
            isPauseChannelDisabled(onlineModeEnabled) &&
            isPauseChannelDisabled(earlyVotingEnabled) &&
            isPauseChannelDisabled(telephoneVotingEnabled)

        return (
            changingStatus ||
            [
                // PublishStatus.Void,
                PublishStatus.Generated,
                PublishStatus.GeneratedLoading,
            ].includes(status) ||
            allChannelsDisabled
        )
    }

    const isStopButtonDisabled = (): boolean => {
        const allChannelsDisabled =
            isStopChannelDisabled(kioskModeEnabled) &&
            isStopChannelDisabled(onlineModeEnabled) &&
            isStopChannelDisabled(earlyVotingEnabled) &&
            isStopChannelDisabled(telephoneVotingEnabled)

        return (
            changingStatus ||
            [
                // PublishStatus.Void,
                PublishStatus.Generated,
                PublishStatus.GeneratedLoading,
            ].includes(status) ||
            allChannelsDisabled
        )
    }

    return (
        <>
            {electionHasSeals ? (
                <Typography
                    variant="body2"
                    color="text.secondary"
                    sx={{textAlign: "right", mb: 1}}
                    className="publish-seal-start-reason"
                >
                    {t("publish.sealRefusals.startDisabled")}
                </Typography>
            ) : null}
            <PublishActionsStyled.Container>
                <div
                    className="list-actions"
                    style={{
                        display: "flex",
                        gap: 0,
                        alignItems: "center",
                        justifyContent: "flex-end",
                    }}
                >
                    {type === EPublishActionsType.List ? (
                        <>
                            {showPublishColumns ? <SelectColumnsButton /> : null}
                            {showPublishFilters ? <FilterButton /> : null}
                            {canInitialize && (
                                <StyledStatusButton
                                    onClick={handleInitialize}
                                    className={"initializeVoting"}
                                    label={String(t("publish.action.generateInitializationReport"))}
                                    disabled={isInitializeDisabled()}
                                >
                                    {initializing ? (
                                        <CircularProgress size={16} />
                                    ) : (
                                        <PlaylistAddCheck width={24} />
                                    )}
                                </StyledStatusButton>
                            )}
                            {canChangeStatus && canPublishStartVoting && (
                                <>
                                    <StyledStatusButton
                                        onClick={(e: React.MouseEvent<HTMLButtonElement>) =>
                                            setStartAnchorEl(e.currentTarget)
                                        }
                                        className={"startVotingMenu"}
                                        label={String(t("publish.action.startVotingPeriod"))}
                                        disabled={isStartButtonDisabled()}
                                    >
                                        <IconOrProgress
                                            st={PublishStatus.Started}
                                            Icon={PlayCircle}
                                        />
                                    </StyledStatusButton>
                                    <Menu
                                        anchorEl={startAnchorEl}
                                        open={startMenuOpen}
                                        onClose={() => setStartAnchorEl(null)}
                                        anchorOrigin={{vertical: "bottom", horizontal: "left"}}
                                        transformOrigin={{vertical: "top", horizontal: "left"}}
                                    >
                                        <StyledMenuItem
                                            disabled={
                                                isStartChannelDisabled(onlineModeEnabled) ||
                                                initializationReportNotGenerated()
                                            }
                                            onClick={() => {
                                                setStartAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_START_VOTING,
                                                    ElectionEventStatus.Open,
                                                    [VotingStatusChannel.Online]
                                                )
                                            }}
                                        >
                                            {t("publish.action.startOnlineVoting")}
                                        </StyledMenuItem>
                                        <StyledMenuItem
                                            disabled={isStartChannelDisabled(kioskModeEnabled)}
                                            onClick={() => {
                                                setStartAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_START_VOTING,
                                                    ElectionEventStatus.Open,
                                                    [VotingStatusChannel.Kiosk]
                                                )
                                            }}
                                        >
                                            {t("publish.action.startKioskVoting")}
                                        </StyledMenuItem>
                                        <StyledMenuItem
                                            disabled={isStartChannelDisabled(earlyVotingEnabled)}
                                            onClick={() => {
                                                setStartAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_START_VOTING,
                                                    ElectionEventStatus.Open,
                                                    [VotingStatusChannel.EarlyVoting]
                                                )
                                            }}
                                        >
                                            {t("publish.action.startEarlyVoting")}
                                        </StyledMenuItem>
                                        <StyledMenuItem
                                            disabled={isStartChannelDisabled(
                                                telephoneVotingEnabled
                                            )}
                                            onClick={() => {
                                                setStartAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_START_VOTING,
                                                    ElectionEventStatus.Open,
                                                    [VotingStatusChannel.Telephone]
                                                )
                                            }}
                                        >
                                            {t("publish.action.startTelephoneVoting")}
                                        </StyledMenuItem>
                                    </Menu>
                                </>
                            )}

                            {canChangeStatus && canPublishPauseVoting && (
                                <>
                                    <StyledStatusButton
                                        onClick={(e: React.MouseEvent<HTMLButtonElement>) =>
                                            setPauseAnchorEl(e.currentTarget)
                                        }
                                        className={"pauseVotingMenu"}
                                        label={String(t("publish.action.pauseVotingPeriod"))}
                                        disabled={isPauseButtonDisabled()}
                                    >
                                        <IconOrProgress
                                            st={PublishStatus.Paused}
                                            Icon={PauseCircle}
                                        />
                                    </StyledStatusButton>
                                    <Menu
                                        anchorEl={pauseAnchorEl}
                                        open={pauseMenuOpen}
                                        onClose={() => setPauseAnchorEl(null)}
                                        anchorOrigin={{vertical: "bottom", horizontal: "left"}}
                                        transformOrigin={{vertical: "top", horizontal: "left"}}
                                    >
                                        <StyledMenuItem
                                            disabled={isPauseChannelDisabled(onlineModeEnabled)}
                                            onClick={() => {
                                                setPauseAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_PAUSE_VOTING,
                                                    ElectionEventStatus.Paused,
                                                    [VotingStatusChannel.Online]
                                                )
                                            }}
                                        >
                                            {t("publish.action.pauseOnlineVoting")}
                                        </StyledMenuItem>
                                        <StyledMenuItem
                                            disabled={isPauseChannelDisabled(kioskModeEnabled)}
                                            onClick={() => {
                                                setPauseAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_PAUSE_VOTING,
                                                    ElectionEventStatus.Paused,
                                                    [VotingStatusChannel.Kiosk]
                                                )
                                            }}
                                        >
                                            {t("publish.action.pauseKioskVoting")}
                                        </StyledMenuItem>
                                        <StyledMenuItem
                                            disabled={isPauseChannelDisabled(earlyVotingEnabled)}
                                            onClick={() => {
                                                setPauseAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_PAUSE_VOTING,
                                                    ElectionEventStatus.Paused,
                                                    [VotingStatusChannel.EarlyVoting]
                                                )
                                            }}
                                        >
                                            {t("publish.action.pauseEarlyVoting")}
                                        </StyledMenuItem>
                                        <StyledMenuItem
                                            disabled={isPauseChannelDisabled(
                                                telephoneVotingEnabled
                                            )}
                                            onClick={() => {
                                                setPauseAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_PAUSE_VOTING,
                                                    ElectionEventStatus.Paused,
                                                    [VotingStatusChannel.Telephone]
                                                )
                                            }}
                                        >
                                            {t("publish.action.pauseTelephoneVoting")}
                                        </StyledMenuItem>
                                    </Menu>
                                </>
                            )}

                            {canChangeStatus && canPublishStopVoting && (
                                <>
                                    <StyledStatusButton
                                        onClick={(e: React.MouseEvent<HTMLButtonElement>) =>
                                            setStopAnchorEl(e.currentTarget)
                                        }
                                        className={"stopVotingMenu"}
                                        label={String(t("publish.action.stopVotingPeriod"))}
                                        disabled={isStopButtonDisabled()}
                                    >
                                        <IconOrProgress
                                            st={PublishStatus.Stopped}
                                            Icon={StopCircle}
                                        />
                                    </StyledStatusButton>
                                    <Menu
                                        anchorEl={stopAnchorEl}
                                        open={stopMenuOpen}
                                        onClose={() => setStopAnchorEl(null)}
                                        anchorOrigin={{vertical: "bottom", horizontal: "left"}}
                                        transformOrigin={{vertical: "top", horizontal: "left"}}
                                    >
                                        <StyledMenuItem
                                            disabled={
                                                isStopChannelDisabled(onlineModeEnabled) ||
                                                isVotingPeriodEndDisallowed
                                            }
                                            onClick={() => {
                                                setStopAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_STOP_VOTING,
                                                    ElectionEventStatus.Closed,
                                                    [VotingStatusChannel.Online]
                                                )
                                            }}
                                        >
                                            {t("publish.action.stopOnlineVoting")}
                                        </StyledMenuItem>
                                        <StyledMenuItem
                                            disabled={isStopChannelDisabled(kioskModeEnabled)}
                                            onClick={() => {
                                                setStopAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_STOP_VOTING,
                                                    ElectionEventStatus.Closed,
                                                    [VotingStatusChannel.Kiosk]
                                                )
                                            }}
                                        >
                                            {t("publish.action.stopKioskVotingPeriod")}
                                        </StyledMenuItem>
                                        <StyledMenuItem
                                            disabled={isStopChannelDisabled(earlyVotingEnabled)}
                                            onClick={() => {
                                                setStopAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_STOP_VOTING,
                                                    ElectionEventStatus.Closed,
                                                    [VotingStatusChannel.EarlyVoting]
                                                )
                                            }}
                                        >
                                            {t("publish.action.stopEarlyVoting")}
                                        </StyledMenuItem>
                                        <StyledMenuItem
                                            disabled={isStopChannelDisabled(telephoneVotingEnabled)}
                                            onClick={() => {
                                                setStopAnchorEl(null)
                                                handleChangeVotingPeriod(
                                                    EPublishActions.PENDING_STOP_VOTING,
                                                    ElectionEventStatus.Closed,
                                                    [VotingStatusChannel.Telephone]
                                                )
                                            }}
                                        >
                                            {t("publish.action.stopTelephoneVoting")}
                                        </StyledMenuItem>
                                    </Menu>
                                </>
                            )}

                            {canWrite && canPublishChanges && (
                                <StatusButton
                                    Icon={Publish}
                                    onClick={() => handlePublish(false)}
                                    st={PublishStatus.Generated}
                                    label={String(t("publish.action.publish"))}
                                    disabledStatus={[]}
                                    disabled={changingStatus}
                                />
                            )}
                        </>
                    ) : (
                        <>
                            {canWrite && canPublishRegenerate && (
                                <div className="list-actions" style={{paddingBottom: "4px"}}>
                                    <StatusButton
                                        Icon={RotateLeft}
                                        disabledStatus={[]}
                                        st={PublishStatus.Generated}
                                        label={String(t("publish.action.generate"))}
                                        onClick={() => handlePublish(true)}
                                    />
                                </div>
                            )}

                            {canWrite && (
                                <PublishExport ballotPublicationId={ballotPublicationId} />
                            )}
                        </>
                    )}
                </div>
            </PublishActionsStyled.Container>

            <Dialog
                handleClose={(flag) => {
                    if (flag && currentCallback) {
                        currentCallback() // Execute the saved callback
                    }
                    setShowDialog(false) // Close the dialog
                    setCurrentCallback(null) // Reset the callback
                }}
                open={showDialog}
                title={String(t("publish.dialog.title"))}
                ok={String(t("publish.dialog.ok"))}
                cancel={String(t("publish.dialog.ko"))}
                variant="info"
            >
                <Typography variant="body1">{dialogText}</Typography>
            </Dialog>
        </>
    )
}
