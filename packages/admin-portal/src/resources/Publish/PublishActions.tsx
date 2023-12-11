import React from "react"

import styled from "@emotion/styled"

import {Button} from "react-admin"
import {useTranslation} from "react-i18next"
import {CircularProgress} from "@mui/material"
import {Publish, RotateLeft, PlayCircle, PauseCircle, StopCircle} from "@mui/icons-material"

import {EPublishStatus} from "./EPublishStatus"

const PublishActionsStyled = {
    Container: styled.div`
        display: flex;
        margin-bottom: 16px;
        justify-content: flex-end;
    `,
}

export type PublishActionsProps = {
    status: null | number
    onPublish: () => void
    onGenerate: () => void
}

export const PublishActions: React.FC<PublishActionsProps> = ({status, onPublish, onGenerate}) => {
    const {t} = useTranslation()

    const IconOrProgress = ({st, Icon}: any) => {
        return status === st && status !== EPublishStatus.Void ? (
            <CircularProgress size={16} />
        ) : (
            <Icon width={24} />
        )
    }

    return (
        <PublishActionsStyled.Container>
            <div className="list-actions">
                <Button onClick={() => null} label={t("publish.action.start")}>
                    <IconOrProgress st={EPublishStatus.Started} Icon={PlayCircle} />
                </Button>
                <Button onClick={() => null} label={t("publish.action.pause")}>
                    <IconOrProgress st={EPublishStatus.Paused} Icon={PauseCircle} />
                </Button>
                <Button onClick={() => null} label={t("publish.action.stop")}>
                    <IconOrProgress st={EPublishStatus.Stopped} Icon={StopCircle} />
                </Button>
                <Button onClick={onPublish} label={t("publish.action.publish")} >
                    <IconOrProgress st={EPublishStatus.Published} Icon={Publish} />
                </Button>
                <Button onClick={onGenerate} label={t("publish.action.generate")}>
                    <IconOrProgress st={EPublishStatus.Generated} Icon={RotateLeft} />
                </Button>
            </div>
        </PublishActionsStyled.Container>
    )
}
