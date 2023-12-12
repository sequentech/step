import React, {ComponentType, useEffect, useRef, useState} from "react"

import styled from "@emotion/styled"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"

import {useGetList, useNotify} from "react-admin"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Box, Accordion, AccordionDetails, AccordionSummary} from "@mui/material"

import Summary from "./election-publish.json"
import OldSummary from "./election-publish-old.json"

import {DiffView} from "@/components/DiffView"
import {PublishActions} from "./PublishActions"
import {EPublishStatus} from "./EPublishStatus"
import {PUBLISH_BALLOT} from "@/queries/PublishBallot"
import {GENERATE_BALLOT_PUBLICATION} from "@/queries/GenerateBallotPublication"
import {GET_BALLOT_PUBLICATION_CHANGE} from "@/queries/GetBallotPublicationChanges"
import {
    GenerateBallotPublicationMutation,
    GetBallotPublicationChangesOutput,
    PublishBallotMutation,
    Sequent_Backend_Ballot_Publication,
} from "@/gql/graphql"
import globalSettings from "@/global-settings"

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
    electionId?: string
    electionEventId: string
}

const PublishMemo: React.MemoExoticComponent<ComponentType<TPublish>> = React.memo(
    ({electionEventId, electionId}: TPublish): React.JSX.Element => {
        let current: any

        const {t} = useTranslation()
        const notify = useNotify()
        const ref = useRef(null)
        const [expan, setExpan] = useState<string>("election-publish-diff")
        const [status, setStatus] = useState<null | number>(EPublishStatus.Void)
        const [ballotPublicationId, setBallotPublicationId] = useState<null | string>(null)
        const [currentData, setCurrentData] = useState<any>(null)
        const [previousData, setPreviousData] = useState<any>([])

        const {data, isLoading, error, refetch} = useGetList<Sequent_Backend_Ballot_Publication>(
            "sequent_backend_ballot_publication",
            {
                pagination: {
                    page: 1,
                    perPage: 10,
                },
                sort: {
                    field: "created_at",
                    order: "DESC",
                },
                filter: {
                    election_event_id: electionEventId,
                },
            }
        )

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
            const {data} = await publishBallot({
                variables: {
                    electionEventId,
                    ballotPublicationId: ballotPublicationId,
                },
            })
            notify(t("publish.notifications.published"), {type: "success"})
            refetch()
        }

        const onGenerate = async () => {
            setStatus(EPublishStatus.Generated)

            const {data} = await generateBallotPublication({
                variables: {
                    electionId,
                    electionEventId,
                },
            })

            setStatus(EPublishStatus.Void)
            notify(t("publish.notifications.generated"), {type: "success"})

            if (data?.generate_ballot_publication?.ballot_publication_id) {
                setBallotPublicationId(data?.generate_ballot_publication?.ballot_publication_id)
            }
            setTimeout(() => {
                refetch()
            }, 4000)
        }

        const getPublishChanges = async () => {
            const {data} = await getBallotPublicationChanges({
                variables: {
                    electionEventId,
                    ballotPublicationId,
                },
            })

            console.log("PUBLISH :: Get publish changes data =>", data)

            if (!data) {
                return
            }
            console.log("FFF Set changes")
            setCurrentData(data.current)
            if (data.previous) {
                setPreviousData(data.previous)
            }
        }

        useEffect(() => {
            if (!ballotPublicationId) {
                return
            }
            let foundPublication = data?.find((p) => p.id === ballotPublicationId)
            if (!foundPublication) {
                return
            }
            getPublishChanges()
        }, [ballotPublicationId, data])

        useEffect(() => {
            if (!current) {
                current = ref.current

                onGenerate()
            }
        }, [ref])

        return (
            <Box ref={ref} sx={{flexGrow: 2, flexShrink: 0}}>
                <PublishActions status={status} onPublish={onPublish} onGenerate={onGenerate} />

                <PublishStyled.Container>
                    <Accordion
                        sx={{width: "100%"}}
                        expanded={expan == "election-publish-diff"}
                        onChange={() => setExpan("election-publish-diff")}
                    >
                        <AccordionSummary
                            expandIcon={<ExpandMoreIcon id="election-publish-diff" />}
                        >
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
                        <AccordionSummary
                            expandIcon={<ExpandMoreIcon id="election-publish-history" />}
                        >
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
)

PublishMemo.displayName = "Publish"

export const Publish = PublishMemo
