import React from 'react'

import styled from "@emotion/styled"

import { Button } from "react-admin"
import { useTranslation } from 'react-i18next'
import { Add, PlayCircle, PauseCircle, StopCircle, PublishedWithChanges } from "@mui/icons-material"

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
                <Button onClick={() =>null} label="START ELECTION">
                    <PlayCircle width={24} />
                </Button>
                <Button onClick={() =>null} label="PAUSE">
                    <PauseCircle width={24} />
                </Button>
                <Button onClick={() =>null} label="STOP ELECTION">
                    <StopCircle width={24} />
                </Button>
                <Button onClick={() =>null} label="PUBLISH CHNAGES">
                    <PublishedWithChanges width={24} />
                </Button>
            </div>
        </PublishActionsStyled.Container>
    )
}