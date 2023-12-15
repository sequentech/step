import React, { ComponentType, useEffect, useRef, useState } from "react"

import styled from "@emotion/styled"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import { useMutation } from "@apollo/client"
import { useTranslation } from "react-i18next"
import { useGetList, useNotify } from 'react-admin'
import { Box, Accordion, AccordionDetails, AccordionSummary } from "@mui/material"

import Summary from "./election-publish.json"
import OldSummary from "./election-publish-old.json"

import { EPublishType } from './EPublishType'
import { DiffView } from "@/components/DiffView"
import { PublishActions } from "./PublishActions"
import { EPublishStatus } from "./EPublishStatus"
import { PUBLISH_BALLOT } from "@/queries/PublishBallot"
import { GENERATE_BALLOT_PUBLICATION } from "@/queries/GenerateBallotPublication"
import { GET_BALLOT_PUBLICATION_CHANGE } from '@/queries/GetBallotPublicationChanges'

import { 
    PublishBallotMutation, 
    GenerateBallotPublicationMutation, 
    GetBallotPublicationChangesOutput, 
    Sequent_Backend_Ballot_Publication,
} from "@/gql/graphql"
import { EPublishActionsType } from './EPublishActionsType'

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
    data: any;
    status: number;
    electionId?: string;
    electionEventId: string;
    onPublish: () => void;
    onGenerate: () => void;
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
            setCurrentState(data?.previous)
            setPreviouseState(data?.current)
        }
    }, [data])
    
    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <PublishStyled.Container>
                <PublishActions 
                    status={status}
                    onPublish={onPublish} 
                    onGenerate={onGenerate}
                    type={EPublishActionsType.Generate}
                />

                <PublishStyled.AccordionHeaderTitle>
                    {t("publish.header.change")}
                </PublishStyled.AccordionHeaderTitle>


                <DiffView
                    currentTitle={t("publish.label.current")}
                    diffTitle={t("publish.label.diff")}
                    current={currentState}
                    modify={previousState}
                />
            
            </PublishStyled.Container>
        </Box>
    )
};
