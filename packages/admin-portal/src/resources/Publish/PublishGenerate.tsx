import React, { useEffect, useState } from "react"

import styled from "@emotion/styled"

import { Box } from "@mui/material"
import { useTranslation } from "react-i18next"
import { PublishActions } from "./PublishActions"

import { DiffView } from '@/components/DiffView'
import { EPublishActionsType } from './EPublishType'

const PublishStyled = {
    Container: styled.div`
        display: flex;
        flex-direction: column;
        gap: 32px;
    `,
    AccordionHeaderTitle: styled.span`
        font-family: Roboto;
        font-size: 24px;
        font-weight: 700;
        line-height: 32px;
        letter-spacing: 0px;
        text-align: left;
    `,
    Loading: styled.div`
        display: flex;
        height: 60vh;
        justify-content: center;
        align-items: center;
    `,
}

export type TPublishGenerate = {
    data: any
    status: number
    electionId?: string
    electionEventId: string
    onPublish: () => void
    onGenerate: () => void
}

export const PublishGenerate: React.FC<TPublishGenerate> = ({ 
    data,
    status,
    onPublish = () => null,
    onGenerate = () => null,
}): React.JSX.Element => {
    const {t} = useTranslation()
    const [currentState, setCurrentState] = useState<null|any>(null)
    const [previousState, setPreviouseState] = useState<null|any>(null)

    useEffect(() => {
        if (data) {
            setCurrentState(data?.previous || {})
            setPreviouseState(data?.current || {})
        }
    }, [data])
    
    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <PublishStyled.Container>

                <PublishStyled.AccordionHeaderTitle>
                    {t("publish.header.change")}
                </PublishStyled.AccordionHeaderTitle>


                <DiffView
                    currentTitle={t("publish.label.current")}
                    diffTitle={t("publish.label.diff")}
                    current={currentState}
                    modify={previousState}
                />

                <PublishActions 
                    status={status}
                    onPublish={onPublish} 
                    onGenerate={onGenerate}
                    type={EPublishActionsType.Generate}
                />
            
            </PublishStyled.Container>
        </Box>
    )
};
