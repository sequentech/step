import React from "react"

import styled from "@emotion/styled"

import {Button} from "react-admin"
import {useTranslation} from "react-i18next"
import {CircularProgress} from "@mui/material"
import {Publish, RotateLeft, PlayCircle, PauseCircle, StopCircle} from "@mui/icons-material"

import {EPublishStatus} from "./EPublishStatus"
import { EPublishActionsType } from './EPublishActionsType'

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
    type: EPublishActionsType.List | EPublishActionsType.Generate
}

export const PublishActions: React.FC<PublishActionsProps> = ({ status, onPublish, onGenerate, type }) => {
    const {t} = useTranslation()

    const IconOrProgress = ({ st, Icon }: any) => {
        return status === (st+0.1) && status !== EPublishStatus.Void ? (
            <CircularProgress size={16} />
        ) : (
            <Icon width={24} />
        )
    }

    const ButtonDisabledOrNot = ({ st, label, onClick, Icon }: any) => (
        <Button 
            onClick={onClick} 
            label={t(label)}
            style={st === status ? { 
                backgroundColor: '#eee', 
                color: '#ccc',
                cursor: 'not-allowed'
            } : {}} 
            disabled={st === status}
        >
            <IconOrProgress st={st} Icon={Icon} />
        </Button>
    )

    return (
        <PublishActionsStyled.Container>
            <div className="list-actions">
                {
                    type === EPublishActionsType.List ? (
                        <>
                            <ButtonDisabledOrNot onClick={() => null} label={t("publish.action.start")} st={EPublishStatus.Started} Icon={PlayCircle} />
            
                            <ButtonDisabledOrNot onClick={() => null} label={t("publish.action.pause")} st={EPublishStatus.Paused} Icon={PauseCircle} />
            
                            <ButtonDisabledOrNot onClick={() => null} label={t("publish.action.stop")} st={EPublishStatus.Stopped} Icon={StopCircle} />

                            <ButtonDisabledOrNot onClick={onPublish} label={t("publish.action.go_to_publish")} st={EPublishStatus.Published} Icon={Publish} />
                        </>
                    ) : (
                        <>
                            <ButtonDisabledOrNot onClick={onPublish} label={t("publish.action.publish")} st={EPublishStatus.Published} Icon={Publish} />
            
                            <ButtonDisabledOrNot onClick={onGenerate} label={t("publish.action.generate")}  st={EPublishStatus.Generated} Icon={RotateLeft} />
                        </>
                    )
                }
            </div>
        </PublishActionsStyled.Container>
    )
}
