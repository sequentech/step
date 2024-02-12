import React, {ComponentType, useContext, useEffect, useRef, useState} from "react"

import {Box} from "@mui/material"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {useGetOne, useNotify, useRecordContext, Identifier} from "react-admin"

import {EPublishType} from "./EPublishType"
import {PUBLISH_BALLOT} from "@/queries/PublishBallot"
import {EPublishStatus, PUBLISH_STATUS_CONVERT} from "./EPublishStatus"
import {GENERATE_BALLOT_PUBLICATION} from "@/queries/GenerateBallotPublication"
import {GET_BALLOT_PUBLICATION_CHANGE} from "@/queries/GetBallotPublicationChanges"

import {
    PublishBallotMutation,
    Sequent_Backend_Election,
    UpdateEventVotingStatusOutput,
    Sequent_Backend_Election_Event,
    UpdateElectionVotingStatusOutput,
    GenerateBallotPublicationMutation,
    GetBallotPublicationChangesOutput,
    Sequent_Backend_Ballot_Publication,
} from "@/gql/graphql"

import {PublishList} from "./PublishList"
import {PublishGenerate} from "./PublishGenerate"
import {IElectionEventStatus} from "@sequentech/ui-essentials"
import {UPDATE_EVENT_VOTING_STATUS} from "@/queries/UpdateEventVotingStatus"
import {UPDATE_ELECTION_VOTING_STATUS} from "@/queries/UpdateElectionVotingStatus"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ApolloContext} from "@/providers/ApolloContextProvider"

export type TPublish = {
    electionId?: string
    electionEventId: string
    type: EPublishType.Election | EPublishType.Event
}

enum ViewMode {
    Edit,
    View,
    List,
}

const PublishMemo: React.MemoExoticComponent<ComponentType<TPublish>> = React.memo(
    ({electionEventId, electionId, type}: TPublish): React.JSX.Element => {
        const notify = useNotify()
        const {t} = useTranslation()
        const [tenantId] = useTenantStore()
        const [viewMode, setViewMode] = useState<ViewMode>(ViewMode.List)
        const [status, setStatus] = useState<string>(EPublishStatus.Void)
        const [ballotPublicationId, setBallotPublicationId] = useState<string | Identifier | null>(
            null
        )
        const authContext = useContext(AuthContext)
        const canWrite = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_WRITE)
        const canRead = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_READ)

        const record = useRecordContext<Sequent_Backend_Election_Event | Sequent_Backend_Election>()
        const [generateData, setGenerateData] = useState<GetBallotPublicationChangesOutput | null>(
            null
        )

        const [publishBallot] = useMutation<PublishBallotMutation>(PUBLISH_BALLOT)

        const [getBallotPublicationChanges] = useMutation<GetBallotPublicationChangesOutput>(
            GET_BALLOT_PUBLICATION_CHANGE
        )
        const [generateBallotPublication] = useMutation<GenerateBallotPublicationMutation>(
            GENERATE_BALLOT_PUBLICATION
        )

        const [updateStatusEvent] = useMutation<UpdateEventVotingStatusOutput>(
            UPDATE_EVENT_VOTING_STATUS
        )

        const [updateStatusElection] = useMutation<UpdateElectionVotingStatusOutput>(
            UPDATE_ELECTION_VOTING_STATUS
        )
        const {data: ballotPublication, refetch} = useGetOne<Sequent_Backend_Ballot_Publication>(
            "sequent_backend_ballot_publication",
            {
                id: ballotPublicationId,
            }
        )

        const onPublish = async () => {
            try {
                if (!ballotPublicationId) {
                    await onGenerate()
                    return
                }

                handleSetStatus(EPublishStatus.PublishedLoading)

                const {data} = await publishBallot({
                    variables: {
                        electionEventId,
                        ballotPublicationId,
                    },
                })

                if (data?.publish_ballot?.ballot_publication_id) {
                    setBallotPublicationId(data?.publish_ballot?.ballot_publication_id)
                }

                refetch()
                setViewMode(ViewMode.List)

                notify(t("publish.notifications.published"), {
                    type: "success",
                })

                handleSetStatus(EPublishStatus.Void)
            } catch (e) {
                notify(t("publish.dialog.error_publish"), {
                    type: "error",
                })
            }
        }

        const onGenerate = async () => {
            try {
                setViewMode(ViewMode.Edit)
                handleSetStatus(EPublishStatus.GeneratedLoading)

                const {data} = await generateBallotPublication({
                    variables: {
                        electionId,
                        electionEventId,
                    },
                })

                handleSetStatus(EPublishStatus.GeneratedLoading)

                if (data?.generate_ballot_publication?.ballot_publication_id) {
                    setBallotPublicationId(data?.generate_ballot_publication?.ballot_publication_id)
                }
            } catch (e) {
                notify(t("publish.dialog.error"), {
                    type: "error",
                })
            }
        }

        const onChangeStatus = (status: string) => {
            handleSetStatus(PUBLISH_STATUS_CONVERT[status] + 0.1)

            if (type === EPublishType.Election) {
                onChangeElectionStatus(status)
            } else if (type === EPublishType.Event) {
                onChangeEventStatus(status)
            }
        }

        const onChangeElectionStatus = async (status: string) => {
            try {
                await updateStatusElection({
                    variables: {
                        status,
                        electionId,
                        electionEventId,
                    },
                })

                handleSetStatus(PUBLISH_STATUS_CONVERT[status])

                notify(t("publish.notifications.change_status"), {
                    type: "success",
                })
            } catch (e) {
                notify(t("publish.dialog.error_status"), {
                    type: "error",
                })
            }
        }

        const onChangeEventStatus = async (status: string) => {
            try {
                await updateStatusEvent({
                    variables: {
                        status,
                        electionEventId,
                    },
                })

                handleSetStatus(PUBLISH_STATUS_CONVERT[status])

                notify(t("publish.notifications.change_status"), {
                    type: "success",
                })
            } catch (e) {
                notify(t("publish.dialog.error_status"), {
                    type: "error",
                })
            }
        }

        const getPublishChanges = async () => {
            const {
                data: {get_ballot_publication_changes: data},
            } = (await getBallotPublicationChanges({
                variables: {
                    electionEventId,
                    ballotPublicationId,
                },
            })) as any

            setGenerateData(data)
        }

        const handleSetStatus = (flag: string) => {
            if (status !== EPublishStatus.Stopped) {
                setStatus(flag)
            }
        }

        useEffect(() => {
            if (electionEventId && ballotPublicationId && ballotPublication?.is_generated) {
                getPublishChanges()
            }
        }, [ballotPublicationId, ballotPublication?.is_generated])

        useEffect(() => {
            console.log("PUBLISH :: BALLOT PUBLICATION ID", ballotPublicationId)
            if (ballotPublicationId) {
                setTimeout(() => {
                    refetch()
                }, 3000)
            }
        }, [ballotPublicationId])

        useEffect(() => {
            if (generateData) {
                handleSetStatus(EPublishStatus.Generated)

                if (!viewMode) {
                    notify(t("publish.notifications.generated"), {
                        type: "success",
                    })
                }
            }
        }, [generateData])

        useEffect(() => {
            const status = record?.status as IElectionEventStatus | undefined

            handleSetStatus(
                status?.voting_status
                    ? PUBLISH_STATUS_CONVERT?.[status?.voting_status]
                    : EPublishStatus.Void
            )
        }, [record])

        return (
            <Box sx={{flexGrow: 2, flexShrink: 0}}>
                {viewMode == ViewMode.List && (
                    <PublishList
                        status={status}
                        canRead={canRead}
                        canWrite={canWrite}
                        electionId={electionId}
                        onGenerate={onGenerate}
                        onChangeStatus={onChangeStatus}
                        electionEventId={electionEventId}
                        setBallotPublicationId={(id: Identifier) => {
                            setViewMode(ViewMode.View)
                            setBallotPublicationId(id)
                        }}
                    />
                )}
                {(viewMode == ViewMode.Edit || viewMode == ViewMode.View) && (
                    <PublishGenerate
                        status={status}
                        readOnly={viewMode == ViewMode.View}
                        data={generateData}
                        onPublish={onPublish}
                        electionId={electionId}
                        onGenerate={onGenerate}
                        onBack={() => {
                            refetch()
                            setViewMode(ViewMode.List)
                            setBallotPublicationId(null)
                            handleSetStatus(EPublishStatus.Generated)
                        }}
                        electionEventId={electionEventId}
                    />
                )}
            </Box>
        )
    }
)

PublishMemo.displayName = "Publish"

export const Publish = PublishMemo
