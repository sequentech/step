import React, { ComponentType, useEffect, useRef, useState } from "react"

import styled from "@emotion/styled"

import { Box } from "@mui/material"
import { useNotify } from 'react-admin'
import { useMutation } from "@apollo/client"
import { useTranslation } from "react-i18next"

import { EPublishType } from './EPublishType'
import { EPublishStatus } from "./EPublishStatus"
import { PUBLISH_BALLOT } from "@/queries/PublishBallot"
import { GENERATE_BALLOT_PUBLICATION } from "@/queries/GenerateBallotPublication"
import { GET_BALLOT_PUBLICATION_CHANGE } from '@/queries/GetBallotPublicationChanges'

import { 
    PublishBallotMutation, 
    GenerateBallotPublicationMutation, 
    GetBallotPublicationChangesOutput,
} from "@/gql/graphql"

import { PublishList } from './PublishList'
import { PublishGenerate } from './PublishGenerate'

export type TPublish = {
    electionId?: string;
    electionEventId: string;
    type: EPublishType.Election | EPublishType.Event;
}

const PublishMemo: React.MemoExoticComponent<ComponentType<TPublish>> = React.memo(({ 
    electionEventId, electionId, type
}: TPublish): React.JSX.Element => {
    const notify = useNotify()
    const {t} = useTranslation()
    const [showDiff, setShowDiff] = useState<boolean>(false)
    const [generateData, setGenerateData] = useState<null|any>()
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

        setShowDiff(false)

        notify(t('publish.notifications.published'), {
            type: 'success'
        })

        setStatus(EPublishStatus.Published)
    };

    const onGenerate = async () => {
        console.log('PUBLISH :: ON GENERATE')
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
        
        setGenerateData(data);
    }

    useEffect(() => {
        if (electionEventId && ballotPublicationId) {
            getPublishChanges()
        }
    }, [ballotPublicationId])

    useEffect(() => {
        if (generateData) {
            setShowDiff(true)
        }
    }, [generateData]);

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            {!showDiff ? (
                <PublishList 
                    status={status} 
                    electionId={electionId}
                    onGenerate={onGenerate}
                    electionEventId={electionEventId} 
                />
            ) : (
                <PublishGenerate 
                    status={status} 
                    data={generateData}
                    onPublish={onPublish}
                    electionId={electionId}
                    onGenerate={onGenerate}
                    onChangeStatus={onChangeStatus}
                    electionEventId={electionEventId}
                />
            )}
        </Box>
    )
});

PublishMemo.displayName = 'Publish';

export const Publish = PublishMemo;
