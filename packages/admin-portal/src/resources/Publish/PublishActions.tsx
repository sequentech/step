import React, {useState} from "react"

import styled from "@emotion/styled"

import {Button} from "react-admin"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {CircularProgress, Typography} from "@mui/material"
import {Publish, RotateLeft, PlayCircle, PauseCircle, StopCircle} from "@mui/icons-material"

import {EPublishActionsType} from "./EPublishType"
import {EPublishStatus, EPublishStatushChanges} from "./EPublishStatus"

const PublishActionsStyled = {
    Container: styled.div`
        display: flex;
        margin-bottom: 8px;
        justify-content: flex-end;
    `,
}

export type PublishActionsProps = {
    status: null | number
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
                st === status || disabledStatus?.includes(status)
                    ? {
                          backgroundColor: "#eee",
                          color: "#ccc",
                          cursor: "not-allowed",
                      }
                    : {}
            }
            disabled={st === status || disabledStatus?.includes(status)}
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
                            <ButtonDisabledOrNot
                                onClick={() =>
                                    handleEvent(handleOnChange(EPublishStatushChanges.Open))
                                }
                                label={t("publish.action.start")}
                                st={EPublishStatus.Started}
                                Icon={PlayCircle}
                                disabledStatus={[EPublishStatus.Stopped]}
                            />

                            <ButtonDisabledOrNot
                                onClick={() =>
                                    handleEvent(handleOnChange(EPublishStatushChanges.Paused))
                                }
                                label={t("publish.action.pause")}
                                st={EPublishStatus.Paused}
                                Icon={PauseCircle}
                                disabledStatus={[
                                    EPublishStatus.Void,
                                    EPublishStatus.Stopped,
                                    EPublishStatus.Generated,
                                ]}
                            />

                            <ButtonDisabledOrNot
                                onClick={() =>
                                    handleEvent(handleOnChange(EPublishStatushChanges.Closed))
                                }
                                label={t("publish.action.stop")}
                                st={EPublishStatus.Stopped}
                                Icon={StopCircle}
                                disabledStatus={[EPublishStatus.Void, , EPublishStatus.Generated]}
                            />

                            <ButtonDisabledOrNot
                                onClick={onGenerate}
                                label={t("publish.action.publish")}
                                Icon={Publish}
                                disabledStatus={[EPublishStatus.Stopped]}
                            />
                        </>
                    ) : (
                        <>
                            <ButtonDisabledOrNot
                                onClick={onPublish}
                                label={t("publish.action.publish")}
                                Icon={Publish}
                                disabledStatus={[EPublishStatus.Stopped]}
                            />
                            <ButtonDisabledOrNot
                                onClick={() => handleEvent(onGenerate)}
                                label={t("publish.action.generate")}
                                st={EPublishStatus.Generated}
                                Icon={RotateLeft}
                            />
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
