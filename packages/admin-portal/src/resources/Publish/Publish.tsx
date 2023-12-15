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
import { PublishList } from './PublishList'

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

export type TPublish = {
    electionId?: string;
    electionEventId: string;
    type: EPublishType.Election | EPublishType.Event;
}

const PublishMemo: React.MemoExoticComponent<ComponentType<TPublish>> = React.memo(({ 
    electionEventId, electionId, type
}: TPublish): React.JSX.Element => {
    let current: any

    const ref = useRef(null)
    const notify = useNotify()
    const {t} = useTranslation()
    const [currentState, setCurrentState] = useState<null|any>(null)
    const [previousState, setPreviouseState] = useState<null|any>(null)
    const [expan, setExpan] = useState<string>('election-publish-diff')
    const [status, setStatus] = useState<number>(EPublishStatus.Void)
    const [ballotPublicationId, setBallotPublicationId] = useState<null|string>(null)

    const [publishBallot] = useMutation<PublishBallotMutation>(PUBLISH_BALLOT)
    const [getBallotPublicationChanges] = useMutation<GetBallotPublicationChangesOutput>(
        GET_BALLOT_PUBLICATION_CHANGE
    )
    const [generateBallotPublication] = useMutation<GenerateBallotPublicationMutation>(
        GENERATE_BALLOT_PUBLICATION
    )

    const onPublish = async () => {
        if (!ballotPublicationId) {
            return
        }

        setStatus(EPublishStatus.PublishedLoading)

        const {data} = await publishBallot({
            variables: {
                electionEventId,
                ballotPublicationId,
            },
        })

        if (data?.publish_ballot?.ballot_publication_id) {
            setBallotPublicationId(data?.publish_ballot?.ballot_publication_id)
        }

        notify(t('publish.notifications.published'), {
            type: 'success'
        })

        setStatus(EPublishStatus.Published)
    };

    const onGenerate = async () => {
        setStatus(EPublishStatus.GeneratedLoading)

        const { data } = await generateBallotPublication({
            variables: {
                electionId,
                electionEventId,
            },
        })

        setStatus(EPublishStatus.Void)

        if (data?.generate_ballot_publication?.ballot_publication_id) {
            setBallotPublicationId(data?.generate_ballot_publication?.ballot_publication_id)
        }
    };

    const onChangeStatus = (status: string) => {
        if (type === EPublishType.Election) {
            onChangeElectionStatus(status)
        } else if  (type === EPublishType.Event) {
            onChangeEventStatus(status)
        }
    }

    const onChangeElectionStatus = (status: string) => {

    }

    const onChangeEventStatus = (status: string) => {

    }

    const getPublishChanges = async () => {
        const { data: { get_ballot_publication_changes: data } } = await getBallotPublicationChanges({
            variables: {
                electionEventId,
                ballotPublicationId,
            }
        }) as any
        
        setCurrentState(data?.previous || {});
        setPreviouseState(data?.current || {});
    }

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <PublishList 
                status={status} 
                electionEventId={electionEventId} 
                electionId={electionId}
                onPublish={onPublish}
                onChangeStatus={onChangeStatus}
                onGenerate={onGenerate}
            />
        </Box>
    )
});

PublishMemo.displayName = 'Publish';

export const Publish = PublishMemo;
