// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {styled} from "@mui/material/styles"
import {CircularProgress, Typography, Menu, MenuItem} from "@mui/material"
import {Publish, RotateLeft, PlayCircle, PauseCircle, StopCircle} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {Button, FilterButton, SelectColumnsButton, useRecordContext, Identifier} from "react-admin"

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
    EInitializeReportPolicy,
    EVotingPeriodEnd,
    EVotingStatus,
    IElectionPresentation,
    IElectionStatus,
    IChannelButtonInfo,
} from "@sequentech/ui-core"
import {usePublishPermissions} from "./usePublishPermissions"
import PublishExport from "./PublishExport"

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
    changingStatus: boolean
    onPublish?: () => void
    onGenerate: () => void
    onChangeStatus?: (status: ElectionEventStatus, votingChannel?: VotingStatusChannel[]) => void
    type: EPublishActionsType.List | EPublishActionsType.Generate
}

export const PublishActions: React.FC<PublishActionsProps> = ({
    ballotPublicationId,
    publishType,
    type,
    status,
    kioskModeEnabled,
    onlineModeEnabled,
    earlyVotingEnabled,
    electionStatus,
    electionPresentation,
    changingStatus,
    onGenerate,
    onPublish = () => null,
    onChangeStatus = () => null,
    data,
}) => {
    const {t} = useTranslation()
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
    // const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
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
        if (publishType === EPublishType.Event) {
            const electionEventPublishTabIndex = localStorage.getItem(
                "electionEventPublishTabIndex"
            )
            baseUrl.searchParams.set("tabIndex", electionEventPublishTabIndex ?? "8")
        } else {
            const electionPublishTabIndex = localStorage.getItem("electionPublishTabIndex")
            baseUrl.searchParams.set("tabIndex", electionPublishTabIndex ?? "4")
        }
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

        const dialogMessage = isGoldUser()
            ? action === EPublishActions.PENDING_START_VOTING
                ? t("publish.dialog.startInfo")
                : action === EPublishActions.PENDING_STOP_VOTING
                  ? t("publish.dialog.stopInfo")
                  : t("publish.dialog.pauseInfo")
            : t("publish.dialog.confirmation", {action: actionText})
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
        return !channelEnabled || st === EVotingStatus.OPEN || st === EVotingStatus.CLOSED
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
        return !channelEnabled || st === EVotingStatus.CLOSED || st === EVotingStatus.NOT_STARTED
    }

    const initializationReportNotGenerated = (): boolean => {
        return (
            record?.presentation?.initialization_report_policy ===
                EInitializeReportPolicy.REQUIRED && !record?.initialization_report_generated
        )
    }

    // Encapsulated original disabled logic for each main action button
    const isStartButtonDisabled = (): boolean => {
        const allChannelsDisabled =
            isStartChannelDisabled(kioskModeEnabled) &&
            isStartChannelDisabled(onlineModeEnabled) &&
            isStartChannelDisabled(earlyVotingEnabled)

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
            isPauseChannelDisabled(earlyVotingEnabled)

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
            isStopChannelDisabled(earlyVotingEnabled)

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
                                    </Menu>
                                </>
                            )}

                            {canWrite && canPublishChanges && (
                                <StatusButton
                                    Icon={Publish}
                                    onClick={() => handlePublish(false)}
                                    st={PublishStatus.Generated}
                                    label={String(t("publish.action.publish"))}
                                    disabledStatus={[PublishStatus.Stopped]}
                                />
                            )}
                        </>
                    ) : (
                        <>
                            {canWrite && canPublishRegenerate && (
                                <div className="list-actions" style={{paddingTop: "4px"}}>
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
