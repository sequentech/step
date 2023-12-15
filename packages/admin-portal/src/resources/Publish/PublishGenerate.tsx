import React, { useEffect, useState } from "react"

import styled from "@emotion/styled"

import { Box } from "@mui/material"
import { Button } from "react-admin"
import { useTranslation } from "react-i18next"
import { ArrowBackIosNew } from '@mui/icons-material';

import { DiffView } from '@/components/DiffView'
import { PublishActions } from "./PublishActions"
import { EPublishActionsType } from './EPublishType'

const PublishGenerateStyled = {
    Container: styled.div`
        display: flex;
        flex-direction: column;
        gap: 32px;
        margin-top: -12px;
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
    Bottom: styled.div`
        display: flex;
        padding: 8px 16px;
        width: 100%;
        background-color: #f5f5f5;
        justify-content: space-between;
    `,
}

export type TPublishGenerate = {
    data: any
    status: number
    electionId?: string
    onBack: () => void
    onPublish: () => void
    onGenerate: () => void
    electionEventId: string
}

export const PublishGenerate: React.FC<TPublishGenerate> = ({ 
    data,
    status,
    onBack = () => null,
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

            <PublishActions 
                status={status}
                onPublish={onPublish} 
                onGenerate={onGenerate}
                type={EPublishActionsType.Generate}
            />

            <PublishGenerateStyled.Container>

                <PublishGenerateStyled.AccordionHeaderTitle>
                    {t("publish.header.change")}
                </PublishGenerateStyled.AccordionHeaderTitle>

                <DiffView
                    currentTitle={t("publish.label.current")}
                    diffTitle={t("publish.label.diff")}
                    current={currentState}
                    modify={previousState}
                />

                <PublishGenerateStyled.Bottom>
                    <Button
                        onClick={onBack}
                        label={t('publish.action.back')}
                        style={{ 
                            backgroundColor: '#eee', 
                            color: '#0f054c',
                        }}
                    >
                        <ArrowBackIosNew />
                    </Button>
                    
                    <Button
                        onClick={onPublish}
                        label={t('publish.action.publish')}
                        style={{ 
                            color: '#fff',
                        }}
                    />
                </PublishGenerateStyled.Bottom>
            
            </PublishGenerateStyled.Container>
        </Box>
    )
};
