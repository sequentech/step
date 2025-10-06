// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ComponentType, useCallback, useContext, useEffect, useState} from "react"
import {Box} from "@mui/material"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {useGetOne, useNotify, useRecordContext, Identifier, useRefresh} from "react-admin"

import {EPublishType} from "./EPublishType"
import {PUBLISH_BALLOT} from "@/queries/PublishBallot"
import {
    PublishStatus,
    ElectionEventStatus,
    MAP_ELECTION_EVENT_STATUS_PUBLISH,
    nextStatus,
} from "./EPublishStatus"
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
    VotingStatusChannel,
    Sequent_Backend_Tenant,
} from "@/gql/graphql"

import {PublishList} from "./PublishList"
import {PublishGenerate} from "./PublishGenerate"
import {UPDATE_EVENT_VOTING_STATUS} from "@/queries/UpdateEventVotingStatus"
import {UPDATE_ELECTION_VOTING_STATUS} from "@/queries/UpdateElectionVotingStatus"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    EVotingStatus,
    IElectionEventStatus,
    IElectionPresentation,
    IElectionStatus,
    IVotingChannelsConfig,
    IChannelButtonInfo,
} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {convertToNumber} from "@/lib/helpers"
import {EditPreview} from "./EditPreview"
import FormDialog from "@/components/FormDialog"
import {EPublishActions} from "@/types/publishActions"

enum ViewMode {
    Edit,
    View,
    List,
}

type TPublish = {
    electionId?: string
    electionEventId: string
    type: EPublishType.Election | EPublishType.Event
    showList?: string
}

const PublishMemo: React.MemoExoticComponent<ComponentType<TPublish>> = React.memo(
    ({electionEventId, electionId, type, showList}: TPublish): React.JSX.Element => {
        const MAX_DIFF_LINES = convertToNumber(process.env.MAX_DIFF_LINES) ?? 500
        const notify = useNotify()
        const {t} = useTranslation()
        const [tenantId] = useTenantStore()
        const [viewMode, setViewMode] = useState<ViewMode>(ViewMode.List)
        const [changingStatus, setChangingStatus] = useState<boolean>(false)
        const [publishStatus, setPublishStatus] = useState<PublishStatus>(PublishStatus.Void)
        const [open, setOpen] = React.useState(false)
        const [ballotPublicationId, setBallotPublicationId] = useState<string | Identifier | null>(
            null
        )
        const {globalSettings} = useContext(SettingsContext)
        const authContext = useContext(AuthContext)
        const {isGoldUser} = authContext
        const canWrite = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_WRITE)
        const canRead = authContext.isAuthorized(true, tenantId, IPermissions.PUBLISH_READ)

        const record = useRecordContext<Sequent_Backend_Election_Event | Sequent_Backend_Election>()

        // Used to show the election status
        const [electionStatus, setElectionStatus] = useState<IElectionStatus | null>(null)
        const [electionPresentation, setElectionPresentation] =
            useState<IElectionPresentation | null>(null)

        const refresh = useRefresh()

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
        const [updateStatusEvent, {error: updateStatusEventError}] =
            useMutation<UpdateEventVotingStatusOutput>(UPDATE_EVENT_VOTING_STATUS)
        const [updateStatusElection] = useMutation<UpdateElectionVotingStatusOutput>(
            UPDATE_ELECTION_VOTING_STATUS
        )

        const {data: ballotPublication, refetch} = useGetOne<Sequent_Backend_Ballot_Publication>(
            "sequent_backend_ballot_publication",
            {
                id: ballotPublicationId,
            },
            {
                enabled: !!ballotPublicationId,
            }
        )

        const onPublish = async () => {
            try {
                if (!ballotPublicationId) {
                    await onGenerate()
                    return
                }

                handleSetPublishStatus(PublishStatus.PublishedLoading)

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

                handleSetPublishStatus(PublishStatus.Void)
            } catch (e) {
                notify(t("publish.dialog.error_publish"), {
                    type: "error",
                })
                handleSetPublishStatus(PublishStatus.Void)
            }
        }

        const kioskModeEnabled = () => {
            let status =
                (record?.status as IElectionStatus)?.kiosk_voting_status ??
                EVotingStatus.NOT_STARTED
            let is_channel_enabled =
                (record?.voting_channels as IVotingChannelsConfig)?.kiosk ?? false
            return {
                status,
                is_channel_enabled,
            } as IChannelButtonInfo
        }

        const onlineModeEnabled = () => {
            let status =
                (record?.status as IElectionStatus)?.voting_status ?? EVotingStatus.NOT_STARTED
            let is_channel_enabled =
                (record?.voting_channels as IVotingChannelsConfig)?.online ?? false
            return {
                status,
                is_channel_enabled,
            } as IChannelButtonInfo
        }

        const earlyVotingEnabled = () => {
            let status =
                (record?.status as IElectionStatus)?.early_voting_status ??
                EVotingStatus.NOT_STARTED
            let is_channel_enabled =
                (record?.voting_channels as IVotingChannelsConfig)?.early_voting ?? false
            return {
                status,
                is_channel_enabled,
            } as IChannelButtonInfo
        }

        const onGenerate = async () => {
            try {
                setViewMode(ViewMode.Edit)
                handleSetPublishStatus(PublishStatus.GeneratedLoading)

                const {data} = await generateBallotPublication({
                    variables: {
                        electionId,
                        electionEventId,
                    },
                })
                handleSetPublishStatus(PublishStatus.GeneratedLoading)

                if (data?.generate_ballot_publication?.ballot_publication_id) {
                    setBallotPublicationId(data?.generate_ballot_publication?.ballot_publication_id)
                } else {
                    throw "Publication Generation Error"
                }
            } catch (e) {
                notify(t("publish.dialog.error"), {
                    type: "error",
                })
                handleSetPublishStatus(PublishStatus.Void)
                setViewMode(ViewMode.List)
            }
        }

        const onChangeStatus = (
            electionEventStatus: ElectionEventStatus,
            votingChannel?: VotingStatusChannel[]
        ) => {
            let publishStatus = MAP_ELECTION_EVENT_STATUS_PUBLISH[electionEventStatus]
            let newStatus: PublishStatus = nextStatus(publishStatus)
            handleSetPublishStatus(newStatus)

            if (type === EPublishType.Election) {
                onChangeElectionStatus(electionEventStatus, votingChannel)
            } else if (type === EPublishType.Event) {
                onChangeElectionEventStatus(electionEventStatus, votingChannel)
            }
        }

        const onChangeElectionStatus = async (
            votingStatus: ElectionEventStatus,
            votingChannel?: VotingStatusChannel[]
        ) => {
            try {
                setChangingStatus(true)
                await updateStatusElection({
                    variables: {
                        votingStatus,
                        electionId,
                        electionEventId,
                        votingChannel,
                    },
                })
                // No matter the channel, we need to update the general publish status.
                // That´s used to control the loading icon in the buttons for the transitions.
                handleSetPublishStatus(MAP_ELECTION_EVENT_STATUS_PUBLISH[votingStatus])
                setChangingStatus(false)
                refresh()

                notify(t("publish.notifications.change_status"), {
                    type: "success",
                })
            } catch (e) {
                setChangingStatus(false)
                notify(t("publish.dialog.error_status"), {
                    type: "error",
                })
            }
        }

        const onChangeElectionEventStatus = async (
            electionEventStatus: ElectionEventStatus,
            votingChannel?: VotingStatusChannel[]
        ) => {
            try {
                setChangingStatus(true)
                await updateStatusEvent({
                    variables: {
                        electionEventId,
                        votingStatus: electionEventStatus,
                        votingChannel,
                    },
                })
                handleSetPublishStatus(MAP_ELECTION_EVENT_STATUS_PUBLISH[electionEventStatus])
                setChangingStatus(false)
                refresh()

                notify(t("publish.notifications.change_status"), {
                    type: "success",
                })
            } catch (e) {
                setChangingStatus(false)
                notify(t("publish.dialog.error_status"), {
                    type: "error",
                })
            }
        }

        const fetchAllPublishChanges = useCallback(async () => {
            try {
                const {
                    data: {get_ballot_publication_changes: data},
                } = (await getBallotPublicationChanges({
                    variables: {
                        electionEventId,
                        ballotPublicationId,
                    },
                })) as any
                setGenerateData(data)
            } catch (error) {
                setViewMode(ViewMode.List)
                setGenerateData(null)
                handleSetPublishStatus(PublishStatus.Void)
                notify(t("publish.dialog.error"), {
                    type: "error",
                })
            }
        }, [ballotPublicationId, electionEventId, getBallotPublicationChanges])

        const getPublishChanges = useCallback(async () => {
            try {
                const {
                    data: {get_ballot_publication_changes: data},
                } = (await getBallotPublicationChanges({
                    variables: {
                        electionEventId,
                        ballotPublicationId,
                        limit: MAX_DIFF_LINES / 10,
                    },
                })) as any
                setGenerateData(data)
            } catch (error) {
                setViewMode(ViewMode.List)
                setGenerateData(null)
                handleSetPublishStatus(PublishStatus.Void)
                notify(t("publish.dialog.error"), {
                    type: "error",
                })
            }
        }, [ballotPublicationId, electionEventId, getBallotPublicationChanges])

        const handleSetPublishStatus = useCallback(
            (status: PublishStatus) => {
                if (publishStatus !== PublishStatus.Stopped) {
                    setPublishStatus(status)
                }
            },
            [publishStatus]
        )

        const onPreview = (id: string | Identifier) => {
            setBallotPublicationId(id)
            setOpen(true)
        }

        const handleCloseEditDrawer = () => {
            setOpen(false)
        }

        /**
         * Checks for any pending actions after the component mounts.
         * If a pending action is found, it executes the action and removes the flag.
         */
        useEffect(() => {
            const executePendingActions = async () => {
                let isGold = isGoldUser()

                if (isGold) {
                    const pendingPublish = sessionStorage.getItem(
                        EPublishActions.PENDING_PUBLISH_ACTION
                    )
                    if (pendingPublish) {
                        onGenerate()
                    }
                }
            }
            const cleanup = () => {
                sessionStorage.removeItem(EPublishActions.PENDING_PUBLISH_ACTION)
            }

            if (electionEventId || electionId) {
                executePendingActions()
                cleanup()
            }
        }, [onChangeStatus, onGenerate])

        useEffect(() => {
            if (showList) {
                setViewMode(ViewMode.List)
                setBallotPublicationId(null)
            }
        }, [showList])

        useEffect(() => {
            if (electionEventId && ballotPublicationId && ballotPublication?.is_generated) {
                getPublishChanges()
            }
        }, [
            ballotPublicationId,
            ballotPublication?.is_generated,
            electionEventId,
            getPublishChanges,
        ])

        // Used in order to make sure new generated publications are viewed when task completes
        useEffect(() => {
            if (ballotPublication && ballotPublication.is_generated === false) {
                const intervalId = setInterval(() => {
                    refetch()
                }, globalSettings.QUERY_POLL_INTERVAL_MS)

                return () => clearInterval(intervalId)
            }
        }, [ballotPublication, refetch])

        useEffect(() => {
            if (ballotPublicationId) {
                refetch()
            }
        }, [refetch, ballotPublicationId])

        useEffect(() => {
            if (generateData) {
                handleSetPublishStatus(PublishStatus.Generated)

                if (!viewMode) {
                    notify(t("publish.notifications.generated"), {
                        type: "success",
                    })
                }
            }
        }, [t, notify, viewMode, handleSetPublishStatus, generateData])

        useEffect(() => {
            const status = record?.status as IElectionEventStatus | undefined

            handleSetPublishStatus(
                status?.voting_status
                    ? MAP_ELECTION_EVENT_STATUS_PUBLISH?.[status?.voting_status]
                    : PublishStatus.Void
            )
        }, [updateStatusEventError, handleSetPublishStatus, record])

        useEffect(() => {
            const status = (record?.status as IElectionStatus | null) ?? null
            const presentation = (record?.presentation as IElectionPresentation | null) ?? null

            setElectionStatus(status)
            setElectionPresentation(presentation)
        }, [record])

        return (
            <Box sx={{flexGrow: 2, flexShrink: 0}}>
                {viewMode === ViewMode.List && (
                    <PublishList
                        status={publishStatus}
                        electionStatus={electionStatus}
                        electionPresentation={electionPresentation}
                        canRead={canRead}
                        publishType={type}
                        canWrite={canWrite}
                        kioskModeEnabled={kioskModeEnabled()}
                        onlineModeEnabled={onlineModeEnabled()}
                        earlyVotingEnabled={earlyVotingEnabled()}
                        changingStatus={changingStatus}
                        electionId={electionId}
                        onGenerate={onGenerate}
                        onChangeStatus={onChangeStatus}
                        electionEventId={electionEventId}
                        setBallotPublicationId={(id: Identifier) => {
                            setViewMode(ViewMode.View)
                            setBallotPublicationId(id)
                        }}
                        onPreview={onPreview}
                    />
                )}
                {(viewMode === ViewMode.Edit || viewMode === ViewMode.View) && (
                    <PublishGenerate
                        ballotPublicationId={ballotPublicationId}
                        status={publishStatus}
                        changingStatus={changingStatus}
                        readOnly={viewMode === ViewMode.View}
                        data={generateData}
                        publishType={type}
                        onPublish={onPublish}
                        electionId={electionId}
                        onGenerate={onGenerate}
                        onBack={() => {
                            refetch()
                            setViewMode(ViewMode.List)
                            handleSetPublishStatus(PublishStatus.Generated)
                            setGenerateData(null)
                            setBallotPublicationId(null)
                        }}
                        electionEventId={electionEventId}
                        fetchAllPublishChanges={fetchAllPublishChanges}
                        onPreview={onPreview}
                        kioskModeEnabled={kioskModeEnabled()}
                        onlineModeEnabled={onlineModeEnabled()}
                        earlyVotingEnabled={earlyVotingEnabled()}
                    />
                )}
                <FormDialog
                    open={open}
                    onClose={handleCloseEditDrawer}
                    title={t("publish.dialog.title")}
                >
                    <EditPreview
                        publicationId={ballotPublicationId}
                        electionEventId={electionEventId}
                        close={handleCloseEditDrawer}
                        ballotData={generateData}
                    />
                </FormDialog>
            </Box>
        )
    }
)

PublishMemo.displayName = "Publish"

export const Publish = PublishMemo
