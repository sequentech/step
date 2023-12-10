import React, {useEffect, useState} from "react"

import styled from "@emotion/styled"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {Box, Accordion, AccordionDetails, AccordionSummary, CircularProgress} from "@mui/material"

import Summary from "./election-publish.json"
import OldSummary from "./election-publish-old.json"

import {DiffView} from "@/components/DiffView"
import {PublishActions} from "./PublishActions"
import {useTranslation} from "react-i18next"
import {useMutation} from "@apollo/client"
import {GENERATE_BALLOT_PUBLICATION} from "@/queries/GenerateBallotPublication"
import {PUBLISH_BALLOT} from "@/queries/PublishBallot"
import {GenerateBallotPublicationMutation, PublishBallotMutation} from "@/gql/graphql"

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

export interface PublishProps {
    electionEventId: string
    electionId?: string
}

export const Publish: React.FC<PublishProps> = ({electionEventId, electionId}) => {
    const {t} = useTranslation()
    const [expan, setExpan] = useState<string>("election-publish-diff")
    const [ballotPublicationId, setBallotPublicationId] = useState<string | null>(null)
    const [isPublished, setIsPublished] = useState<boolean>(false)
    const [generateBallotPublication] = useMutation<GenerateBallotPublicationMutation>(
        GENERATE_BALLOT_PUBLICATION
    )
    const [publishBallot] = useMutation<PublishBallotMutation>(PUBLISH_BALLOT)

    const generateNewPublication = async () => {
        const {data} = await generateBallotPublication({
            variables: {
                electionEventId,
                electionId,
            },
        })

        if (data?.generate_ballot_publication?.ballot_publication_id) {
            setIsPublished(false)
            setBallotPublicationId(data.generate_ballot_publication?.ballot_publication_id)
        }
    }

    useEffect(() => {
        generateNewPublication()
    }, [])

    const onPublish = async () => {
        if (!ballotPublicationId) {
            return
        }
        const {data} = await publishBallot({
            variables: {
                electionEventId,
                ballotPublicationId,
            },
        })

        if (data?.publish_ballot?.ballot_publication_id) {
            setIsPublished(true)
        }
    }

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <PublishActions onPublish={generateNewPublication} onGenerate={onPublish} />

            <PublishStyled.Container>
                <Accordion
                    sx={{width: "100%"}}
                    expanded={expan == "election-publish-diff"}
                    onChange={() => setExpan("election-publish-diff")}
                >
                    <AccordionSummary expandIcon={<ExpandMoreIcon id="election-publish-diff" />}>
                        <PublishStyled.AccordionHeaderTitle>
                            {t("publish.header.change")}
                        </PublishStyled.AccordionHeaderTitle>
                    </AccordionSummary>
                    <AccordionDetails>
                        <DiffView
                            currentTitle={t("publish.label.current")}
                            diffTitle={t("publish.label.diff")}
                            current={OldSummary}
                            modify={Summary}
                        />
                    </AccordionDetails>
                </Accordion>

                <Accordion
                    sx={{width: "100%"}}
                    expanded={expan === "election-publish-history"}
                    onChange={() => setExpan("election-publish-history")}
                >
                    <AccordionSummary expandIcon={<ExpandMoreIcon id="election-publish-history" />}>
                        <PublishStyled.AccordionHeaderTitle>
                            {t("publish.header.history")}
                        </PublishStyled.AccordionHeaderTitle>
                    </AccordionSummary>
                    <AccordionDetails>
                        <span>Add correct resource</span>
                    </AccordionDetails>
                </Accordion>
            </PublishStyled.Container>
        </Box>
    )
}
