import React from 'react'

import styled from "@emotion/styled"

import { Button } from "react-admin"
import { useTranslation } from 'react-i18next'
import { Publish, RotateLeft, PlayCircle, PauseCircle, StopCircle } from "@mui/icons-material"

const PublishActionsStyled = {
    Container: styled.div`
        display: flex;
        margin-bottom: 16px;
        justify-content: flex-end;
    `
}

export const PublishActions: React.FC<any> = () => {
    const { t } = useTranslation()

    return (
        <PublishActionsStyled.Container>
            <div className="list-actions">
                <Button onClick={() =>null} label={t('publish.action.start')}>
                    <PlayCircle width={24} />
                </Button>
                <Button onClick={() =>null} label={t('publish.action.pause')}>
                    <PauseCircle width={24} />
                </Button>
                <Button onClick={() =>null} label={t('publish.action.stop')}>
                    <StopCircle width={24} />
                </Button>
                <Button onClick={() =>null} label={t('publish.action.publish')}>
                    <Publish width={24} />
                </Button>
                <Button onClick={() =>null} label={t('publish.action.generate')}>
                    <RotateLeft width={24} />
                </Button>
            </div>
        </PublishActionsStyled.Container>
    )
}