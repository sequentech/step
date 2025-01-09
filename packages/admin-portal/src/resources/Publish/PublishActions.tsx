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

import {useMutation} from "@apollo/client"
import {VotingStatusChannel} from "@/gql/graphql"

import {Sequent_Backend_Election} from "@/gql/graphql"
import {EInitializeReportPolicy, EVotingStatus, IElectionStatus} from "@sequentech/ui-core"
import {UPDATE_ELECTION_INITIALIZATION_REPORT} from "@/queries/UpdateElectionInitializationReport"
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
    kioskModeEnabled: boolean
    changingStatus: boolean
    onPublish?: () => void
    onGenerate: () => void
    onChangeStatus?: (status: ElectionEventStatus, votingChannel?: VotingStatusChannel) => void
    type: EPublishActionsType.List | EPublishActionsType.Generate
}

export const PublishActions: React.FC<PublishActionsProps> = ({
    ballotPublicationId,
    publishType,
    type,
    status,
    kioskModeEnabled,
    electionStatus,
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

    const [UpdateElectionInitializationReport] = useMutation(UPDATE_ELECTION_INITIALIZATION_REPORT)

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
                          color: "#ccc",
                          cursor: "not-allowed",
                          backgroundColor: "#eee",
                      }
                    : {}
            }
            disabled={disabled || disabledStatus?.includes(status) || st === status + 0.1}
        >
            <IconOrProgress st={st} Icon={Icon} />
        </Button>
    )

    /**
     * General Handler for Events:
     * Shows a confirmation dialog without involving re-authentication.
     * Used by buttons that don't require Gold-level permissions.
     */
    const handleEvent = (callback: (status?: number) => void, dialogText: string) => {
        setDialogText(dialogText)
        setShowDialog(true)
        setCurrentCallback(() => callback)
    }

    /**
     * Specific Handler for "Start Voting" Button:
     * Incorporates re-authentication logic for actions that require Gold-level permissions.
     */
    const handleStartVotingPeriod = () => {
        const actionText = t(`publish.action.startVotingPeriod`)
        const dialogMessage = isGoldUser()
            ? t("publish.dialog.startInfo", {action: actionText})
            : t("publish.dialog.confirmation", {action: actionText})

        setDialogText(dialogMessage)
        setShowDialog(true)
        setCurrentCallback(() => async () => {
            try {
                if (!isGoldUser()) {
                    const baseUrl = new URL(window.location.href)
                    if (publishType === EPublishType.Event) {
                        baseUrl.searchParams.set("tabIndex", "8")
                    } else {
                        baseUrl.searchParams.set("tabIndex", "4")
                    }
                    sessionStorage.setItem(EPublishActions.PENDING_START_VOTING, "true")
                    await reauthWithGold(baseUrl.toString())
                } else {
                    onChangeStatus(ElectionEventStatus.Open)
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

        setDialogText(dialogMessage)
        setShowDialog(true)
        setCurrentCallback(() => async () => {
            try {
                if (!isGoldUser()) {
                    const baseUrl = new URL(window.location.href)
                    if (publishType === EPublishType.Event) {
                        baseUrl.searchParams.set("tabIndex", "8")
                    } else {
                        baseUrl.searchParams.set("tabIndex", "4")
                    }
                    sessionStorage.setItem(EPublishActions.PENDING_STOP_KIOSK_ACTION, "true")
                    await reauthWithGold(baseUrl.toString())
                } else {
                    handleOnChange(ElectionEventStatus.Closed, VotingStatusChannel.Kiosk)
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
    const handlePublish = () => {
        const dialogMessage = isGoldUser()
            ? t("publish.dialog.publishInfo", {action: t("publish.action.publish")})
            : t("publish.dialog.confirmation", {action: t("publish.action.publish")})
        setDialogText(dialogMessage)
        setShowDialog(true)

        setCurrentCallback(() => async () => {
            try {
                if (!isGoldUser()) {
                    const baseUrl = new URL(window.location.href)
                    if (publishType === EPublishType.Event) {
                        baseUrl.searchParams.set("tabIndex", "8")
                    } else {
                        baseUrl.searchParams.set("tabIndex", "4")
                    }
                    sessionStorage.setItem(EPublishActions.PENDING_PUBLISH_ACTION, "true")

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
     * Checks for any pending actions after the component mounts.
     * If a pending action is found, it executes the action and removes the flag.
     */
    useEffect(() => {
        const executePendingActions = async () => {
            if (!record) {
                return
            }

            const pendingStart = sessionStorage.getItem(EPublishActions.PENDING_START_VOTING)
            if (pendingStart) {
                sessionStorage.removeItem(EPublishActions.PENDING_START_VOTING)
                onChangeStatus(ElectionEventStatus.Open)
            }

            const pendingPublish = sessionStorage.getItem(EPublishActions.PENDING_PUBLISH_ACTION)
            if (pendingPublish) {
                sessionStorage.removeItem(EPublishActions.PENDING_PUBLISH_ACTION)
                onGenerate()
            }

            const pendingStopKiosk = sessionStorage.getItem(
                EPublishActions.PENDING_STOP_KIOSK_ACTION
            )
            if (pendingStopKiosk) {
                sessionStorage.removeItem(EPublishActions.PENDING_STOP_KIOSK_ACTION)
                onChangeStatus(ElectionEventStatus.Closed, VotingStatusChannel.Kiosk)
            }
        }

        executePendingActions()
    }, [onChangeStatus, onGenerate, record])

    const handleOnChange =
        (status: ElectionEventStatus, votingChannel?: VotingStatusChannel) => () =>
            onChangeStatus(status, votingChannel)

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
                                    onClick={handleStartVotingPeriod}
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
                                        handleEvent(
                                            handleOnChange(ElectionEventStatus.Paused),
                                            t("publish.dialog.pauseInfo")
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
                                        handleEvent(
                                            handleOnChange(ElectionEventStatus.Closed),
                                            t("publish.dialog.stopInfo")
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
                                />
                            )}

                            {canChangeStatus && kioskModeEnabled && (
                                <StyledStatusButton
                                    onClick={handleStopKioskVoting}
                                    className={"kioskMode"}
                                    label={t("publish.action.stopKioskVotingPeriod")}
                                    disabled={changingStatus || !kioskVotingStarted()}
                                >
                                    <StatusIcon changingStatus={changingStatus} Icon={StopCircle} />
                                </StyledStatusButton>
                            )}

                            {canWrite && canPublishChanges && (
                                <StatusButton
                                    Icon={Publish}
                                    onClick={handlePublish}
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
                                        onClick={() =>
                                            handleEvent(onGenerate, t("publish.dialog.info"))
                                        }
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
