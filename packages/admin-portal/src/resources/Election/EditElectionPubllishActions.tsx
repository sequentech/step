import React from 'react'

import styled from "@emotion/styled"

import { Button } from "react-admin"
import { Add, PlayCircle, PauseCircle, StopCircle, PublishedWithChanges } from "@mui/icons-material"
import { useTranslation } from 'react-i18next'

const EditElectionPublishActionsStyled = {
    Container: styled.div`
        display: flex;
        margin-bottom: 16px;
        justify-content: flex-end;
    `
}

export const EditElectionPublishActions: React.FC<any> = () => {
    const { t } = useTranslation()

    return (
        <EditElectionPublishActionsStyled.Container>
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
        </EditElectionPublishActionsStyled.Container>
    )
}