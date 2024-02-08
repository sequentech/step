import React, {useContext, useState} from "react"

import styled from "@emotion/styled"

import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {CircularProgress, Typography} from "@mui/material"
import {Publish, RotateLeft, PlayCircle, PauseCircle, StopCircle} from "@mui/icons-material"
import {Button, FilterButton, SelectColumnsButton} from "react-admin"

import {EPublishActionsType} from "./EPublishType"
import {EPublishStatus, EPublishStatushChanges} from "./EPublishStatus"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

const PublishActionsStyled = {
    Container: styled.div`
        display: flex;
        margin-bottom: 8px;
        justify-content: flex-end;
        width: 100%;
    `,
}

export type PublishActionsProps = {
    status: number
    onPublish?: () => void
    onGenerate: () => void
    onChangeStatus?: (status: string) => void
    type: EPublishActionsType.List | EPublishActionsType.Generate
}

export const PublishActions: React.FC<PublishActionsProps> = ({
    type,
    status,
    onGenerate,
    onPublish = () => null,
    onChangeStatus = () => null,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const canWrite = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_WRITE)
    const canRead = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_READ)
    const canChangeStatus = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.ELECTION_STATE_WRITE
    )

    const [showDialog, setShowDialog] = useState(false)
    const [currentCallback, setCurrentCallback] = useState<any>(null)

    const IconOrProgress = ({st, Icon}: any) => {
        return status === st + 0.1 && status !== EPublishStatus.Void ? (
            <CircularProgress size={16} />
        ) : (
            <Icon width={24} />
        )
    }

    const ButtonDisabledOrNot = ({st, label, onClick, Icon, disabledStatus}: any) => (
        <Button
            onClick={onClick}
            label={t(label)}
            style={
                disabledStatus?.includes(status)
                    ? {
                          color: "#ccc",
                          cursor: "not-allowed",
                          backgroundColor: "#eee",
                      }
                    : {}
            }
            disabled={disabledStatus?.includes(status) || st === status + 0.1}
        >
            <IconOrProgress st={st} Icon={Icon} />
        </Button>
    )

    const handleEvent = (callback: (status?: number) => void) => {
        setShowDialog(true)
        setCurrentCallback(() => callback)
    }

    const handleOnChange = (status: string) => () => onChangeStatus(status)

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
                                        handleEvent(handleOnChange(EPublishStatushChanges.Open))
                                    }
                                    label={t("publish.action.start")}
                                    className="publish-action-start-button"
                                    st={EPublishStatus.Started}
                                    Icon={PlayCircle}
                                    disabledStatus={[
                                        EPublishStatus.Stopped,
                                        EPublishStatus.Started,
                                        EPublishStatus.GeneratedLoading,
                                    ]}
                                />
                            )}

                            {canChangeStatus && (
                                <ButtonDisabledOrNot
                                    onClick={() =>
                                        handleEvent(handleOnChange(EPublishStatushChanges.Paused))
                                    }
                                    label={t("publish.action.pause")}
                                    className="publish-action-pause-button"
                                    st={EPublishStatus.Paused}
                                    Icon={PauseCircle}
                                    disabledStatus={[
                                        EPublishStatus.Void,
                                        EPublishStatus.Paused,
                                        EPublishStatus.Stopped,
                                        EPublishStatus.Generated,
                                        EPublishStatus.GeneratedLoading,
                                    ]}
                                />
                            )}

                            {canChangeStatus && (
                                <ButtonDisabledOrNot
                                    onClick={() =>
                                        handleEvent(handleOnChange(EPublishStatushChanges.Closed))
                                    }
                                    label={t("publish.action.stop")}
                                    className="publish-action-stop-button"
                                    st={EPublishStatus.Stopped}
                                    Icon={StopCircle}
                                    disabledStatus={[
                                        EPublishStatus.Void,
                                        EPublishStatus.Stopped,
                                        EPublishStatus.Generated,
                                        EPublishStatus.GeneratedLoading,
                                    ]}
                                />
                            )}

                            {canWrite && (
                                <ButtonDisabledOrNot
                                    Icon={Publish}
                                    onClick={onGenerate}
                                    st={EPublishStatus.Generated}
                                    label={t("publish.action.publish-button")}
                                    className="publish-action-publish"
                                    disabledStatus={[EPublishStatus.Stopped]}
                                />
                            )}
                        </>
                    ) : (
                        <>
                            {canWrite && (
                                <ButtonDisabledOrNot
                                    Icon={RotateLeft}
                                    disabledStatus={[]}
                                    st={EPublishStatus.Generated}
                                    label={t("publish.action.generate-button")}
                                    onClick={() => handleEvent(onGenerate)}
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
                <Typography variant="body1">{t("publish.dialog.info")}</Typography>
            </Dialog>
        </>
    )
}
