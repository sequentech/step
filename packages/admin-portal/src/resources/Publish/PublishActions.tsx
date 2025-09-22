// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"

import styled from "@emotion/styled"
import {styled as muiStyled} from "@mui/material/styles"

import {CircularProgress, Typography} from "@mui/material"
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
} from "@sequentech/ui-core"
import {usePublishPermissions} from "./usePublishPermissions"
import PublishExport from "./PublishExport"

type SvgIconComponent = typeof SvgIcon

const PublishActionsStyled = {
    Container: styled.div`
        display: flex;
        margin-bottom: 8px;
        justify-content: flex-end;
        width: 100%;
    `,
}

export const StyledStatusButton = muiStyled(Button)`
    &.MuiButtonBase-root {
        line-height: 1 !important;
    }

    :disabled {
        color: #ccc;
        cursor: not-allowed;
        background-color: #eee;
    }
`

export type PublishActionsProps = {
    ballotPublicationId?: string | Identifier | null
    data?: any
    status: PublishStatus
    publishType: EPublishType.Election | EPublishType.Event
    electionStatus: IElectionStatus | null
    electionPresentation: IElectionPresentation | null
    kioskModeEnabled: boolean
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

    const {
        canPublishRegenerate,
        canPublishStartVoting,
        canPublishPauseVoting,
        canPublishStopVoting,
        canPublishChanges,
        showPublishColumns,
        showPublishFilters,
    } = usePublishPermissions()

    const StatusIcon = ({
        changingStatus,
        Icon,
    }: {
        changingStatus: boolean
        Icon: SvgIconComponent
    }) => {
        return changingStatus ? <CircularProgress size={16} /> : <Icon width={24} />
    }

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
            label={t(label)}
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
    let reauthCallback = (baseUrl: URL, action: EPublishActions) => {
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
    }

    /**
     * Specific Handler for "Start Voting" Button:
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
                    reauthCallback(baseUrl, action)
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
     * Specific Handler for "Stop Kiosk Voting" Button:
     * Incorporates re-authentication logic for actions that require Gold-level permissions.
     */
    const handleStopKioskVoting = () => {
        const actionText = t(`publish.action.stopKioskVotingPeriod`)
        const dialogMessage = isGoldUser()
            ? t("publish.dialog.kioskStopInfo", {action: actionText})
            : t("publish.dialog.confirmation", {action: actionText})
        openDialog(dialogMessage)

        setCurrentCallback(() => async () => {
            try {
                if (!isGoldUser()) {
                    const baseUrl = new URL(window.location.href)
                    reauthCallback(baseUrl, EPublishActions.PENDING_STOP_KIOSK_ACTION)
                    await reauthWithGold(baseUrl.toString())
                } else {
                    onChangeStatus(ElectionEventStatus.Closed, [VotingStatusChannel.Kiosk])
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
                isGold && onChangeStatus(ElectionEventStatus.Open)
                sessionStorage.removeItem(EPublishActions.PENDING_START_VOTING)
            }

            const pendingPause = sessionStorage.getItem(EPublishActions.PENDING_PAUSE_VOTING)
            if (pendingPause) {
                isGold && onChangeStatus(ElectionEventStatus.Paused)
                sessionStorage.removeItem(EPublishActions.PENDING_PAUSE_VOTING)
            }

            const pendingStop = sessionStorage.getItem(EPublishActions.PENDING_STOP_VOTING)
            if (pendingStop) {
                isGold && onChangeStatus(ElectionEventStatus.Closed, [VotingStatusChannel.Online])
                sessionStorage.removeItem(EPublishActions.PENDING_STOP_VOTING)
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

    const kioskVotingStarted = () => {
        return (
            kioskModeEnabled &&
            [EVotingStatus.OPEN, EVotingStatus.PAUSED].includes(
                electionStatus?.kiosk_voting_status ?? EVotingStatus.NOT_STARTED
            )
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
                                <StatusButton
                                    onClick={() =>
                                        handleChangeVotingPeriod(
                                            EPublishActions.PENDING_START_VOTING,
                                            ElectionEventStatus.Open
                                        )
                                    }
                                    label={t("publish.action.startVotingPeriod")}
                                    st={PublishStatus.Started}
                                    Icon={PlayCircle}
                                    disabledStatus={[
                                        PublishStatus.Stopped,
                                        PublishStatus.Started,
                                        PublishStatus.GeneratedLoading,
                                    ]}
                                    disabled={
                                        record?.presentation?.initialization_report_policy ===
                                            EInitializeReportPolicy.REQUIRED &&
                                        !record?.initialization_report_generated
                                    }
                                />
                            )}

                            {canChangeStatus && canPublishPauseVoting && (
                                <StatusButton
                                    onClick={() =>
                                        handleChangeVotingPeriod(
                                            EPublishActions.PENDING_PAUSE_VOTING,
                                            ElectionEventStatus.Paused
                                        )
                                    }
                                    label={t("publish.action.pauseVotingPeriod")}
                                    st={PublishStatus.Paused}
                                    Icon={PauseCircle}
                                    disabledStatus={[
                                        PublishStatus.Void,
                                        PublishStatus.Paused,
                                        PublishStatus.Stopped,
                                        PublishStatus.Generated,
                                        PublishStatus.GeneratedLoading,
                                    ]}
                                />
                            )}

                            {canChangeStatus && canPublishStopVoting && (
                                <StatusButton
                                    onClick={() =>
                                        handleChangeVotingPeriod(
                                            EPublishActions.PENDING_STOP_VOTING,
                                            ElectionEventStatus.Closed,
                                            [VotingStatusChannel.Online]
                                        )
                                    }
                                    label={t("publish.action.stopVotingPeriod")}
                                    st={PublishStatus.Stopped}
                                    Icon={StopCircle}
                                    disabledStatus={[
                                        PublishStatus.Void,
                                        PublishStatus.Stopped,
                                        PublishStatus.Generated,
                                        PublishStatus.GeneratedLoading,
                                    ]}
                                    disabled={isVotingPeriodEndDisallowed}
                                />
                            )}

                            {canChangeStatus && kioskModeEnabled && (
                                <StyledStatusButton
                                    onClick={handleStopKioskVoting}
                                    className={"kioskMode"}
                                    label={t("publish.action.stopKioskVotingPeriod")}
                                    disabled={
                                        changingStatus ||
                                        !kioskVotingStarted() ||
                                        isVotingPeriodEndDisallowed
                                    }
                                >
                                    <StatusIcon changingStatus={changingStatus} Icon={StopCircle} />
                                </StyledStatusButton>
                            )}

                            {canWrite && canPublishChanges && (
                                <StatusButton
                                    Icon={Publish}
                                    onClick={() => handlePublish(false)}
                                    st={PublishStatus.Generated}
                                    label={t("publish.action.publish")}
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
                                        label={t("publish.action.generate")}
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
                title={t("publish.dialog.title")}
                ok={t("publish.dialog.ok")}
                cancel={t("publish.dialog.ko")}
                variant="info"
            >
                <Typography variant="body1">{dialogText}</Typography>
            </Dialog>
        </>
    )
}
