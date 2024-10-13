// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useState} from "react"

import styled from "@emotion/styled"

import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {CircularProgress, Typography} from "@mui/material"
import {Publish, RotateLeft, PlayCircle, PauseCircle, StopCircle} from "@mui/icons-material"
import {Button, FilterButton, SelectColumnsButton, useRecordContext} from "react-admin"

import {EPublishActionsType} from "./EPublishType"
import {PublishStatus, ElectionEventStatus, nextStatus} from "./EPublishStatus"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import SvgIcon from "@mui/material/SvgIcon"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {EInitializeReportPolicy} from "@sequentech/ui-core"

type SvgIconComponent = typeof SvgIcon

const PublishActionsStyled = {
    Container: styled.div`
        display: flex;
        margin-bottom: 8px;
        justify-content: flex-end;
        width: 100%;
    `,
}

export type PublishActionsProps = {
    status: PublishStatus
    changingStatus: boolean
    onPublish?: () => void
    onGenerate: () => void
    onChangeStatus?: (status: ElectionEventStatus) => void
    type: EPublishActionsType.List | EPublishActionsType.Generate
}

export const PublishActions: React.FC<PublishActionsProps> = ({
    type,
    status,
    changingStatus,
    onGenerate,
    onPublish = () => null,
    onChangeStatus = () => null,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const canWrite = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_WRITE)
    const record = useRecordContext<Sequent_Backend_Election>()
    const canChangeStatus = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_STATE_WRITE
    )

    const [showDialog, setShowDialog] = useState(false)
    const [dialogText, setDialogText] = useState("")
    const [currentCallback, setCurrentCallback] = useState<any>(null)

    const IconOrProgress = ({st, Icon}: {st: PublishStatus; Icon: SvgIconComponent}) => {
        return nextStatus(st) === status && status !== PublishStatus.Void ? (
            <CircularProgress size={16} />
        ) : (
            <Icon width={24} />
        )
    }

    const ButtonDisabledOrNot = ({
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

    const handleEvent = (callback: (status?: number) => void, dialogText: string) => {
        setDialogText(dialogText)
        setShowDialog(true)
        setCurrentCallback(() => callback)
    }

    const handleOnChange = (status: ElectionEventStatus) => () => onChangeStatus(status)

    return (
        <>
            <PublishActionsStyled.Container>
                <div className="list-actions">
                    {type === EPublishActionsType.List ? (
                        <>
                            <SelectColumnsButton />
                            <FilterButton />
                            {canChangeStatus && (
                                <ButtonDisabledOrNot
                                    onClick={() =>
                                        handleEvent(
                                            handleOnChange(ElectionEventStatus.Open),
                                            t("publish.dialog.startInfo")
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
                                        record?.presentation?.initialize_report_policy ===
                                            EInitializeReportPolicy.REQUIRED &&
                                        !record?.initializion_report_generated
                                    }
                                />
                            )}

                            {canChangeStatus && (
                                <ButtonDisabledOrNot
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

                            {canChangeStatus && (
                                <ButtonDisabledOrNot
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

                            {canWrite && (
                                <ButtonDisabledOrNot
                                    Icon={Publish}
                                    onClick={onGenerate}
                                    st={PublishStatus.Generated}
                                    label={t("publish.action.publish")}
                                    disabledStatus={[PublishStatus.Stopped]}
                                />
                            )}
                        </>
                    ) : (
                        <>
                            {canWrite && (
                                <ButtonDisabledOrNot
                                    Icon={RotateLeft}
                                    disabledStatus={[]}
                                    st={PublishStatus.Generated}
                                    label={t("publish.action.generate")}
                                    onClick={() =>
                                        handleEvent(onGenerate, t("publish.dialog.info"))
                                    }
                                />
                            )}
                        </>
                    )}
                </div>
            </PublishActionsStyled.Container>

            <Dialog
                handleClose={(flag) => {
                    if (flag) {
                        currentCallback()
                    }

                    setShowDialog(false)
                    setCurrentCallback(null)
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
