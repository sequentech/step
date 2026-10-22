// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useState, useContext, useEffect, useCallback} from "react"
import {useTranslation} from "react-i18next"
import {Add, Visibility, Preview} from "@mui/icons-material"
import {Dialog} from "@sequentech/ui-essentials"
import {Box, Typography, Button, DialogContent, DialogActions} from "@mui/material"
import {EPublishActions} from "@/types/publishActions"

import {
    List,
    TextField,
    Identifier,
    TextInput,
    BooleanInput,
    BooleanField,
    DatagridConfigurable,
    WrapperField,
} from "react-admin"

import {ElectionEventStatus, PublishStatus} from "./EPublishStatus"
import {PublishActions} from "./PublishActions"
import {EPublishActionsType, EPublishType} from "./EPublishType"
import {HeaderTitle} from "@/components/HeaderTitle"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ResetFilters} from "@/components/ResetFilters"
import {AuthContext} from "@/providers/AuthContextProvider"
import {VotingStatusChannel} from "@/gql/graphql"
import {IElectionPresentation, IElectionStatus, IChannelButtonInfo} from "@sequentech/ui-core"
import {usePublishPermissions} from "./usePublishPermissions"
import {ThreeStateDatagridHeader} from "@/components/ThreeStateDatagridHeader"
import {AdminDateField} from "@/components/AdminDateTime"

const OMIT_FIELDS: string[] = []

const filters: Array<ReactElement> = [
    <TextInput source="id" key={0} />,
    <BooleanInput source="is_generated" key={1} />,
]

type TPublishList = {
    status: PublishStatus
    electionStatus: IElectionStatus | null
    electionPresentation: IElectionPresentation | null
    electionId?: number | string
    electionEventId: number | string | undefined
    canRead: boolean
    canWrite: boolean
    kioskModeEnabled: IChannelButtonInfo
    onlineModeEnabled: IChannelButtonInfo
    earlyVotingEnabled: IChannelButtonInfo
    telephoneVotingEnabled: IChannelButtonInfo
    changingStatus: boolean
    publishType: EPublishType.Election | EPublishType.Event
    onGenerate: () => void
    onChangeStatus: (status: ElectionEventStatus, votingChannel?: VotingStatusChannel[]) => void
    /** Initializes voting at the Post; election level only. */
    onInitialize?: () => void
    initializing?: boolean
    perCountryInitialization?: boolean
    initializationReportPolicy?: import("@sequentech/ui-core").EInitializeReportPolicy
    setBallotPublicationId: (id: string | Identifier) => void
    onPreview: (id: string | Identifier) => void
}

export const PublishList: React.FC<TPublishList> = ({
    status,
    publishType,
    electionStatus,
    electionPresentation,
    electionId,
    electionEventId,
    kioskModeEnabled,
    onlineModeEnabled,
    earlyVotingEnabled,
    telephoneVotingEnabled,
    changingStatus,
    onGenerate = () => null,
    onChangeStatus = () => null,
    onInitialize,
    initializing = false,
    perCountryInitialization = false,
    initializationReportPolicy,
    setBallotPublicationId = () => null,
    onPreview = () => null,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const {isGoldUser, reauthWithGold} = authContext

    const {canReadPublish, canPublishCreate, showPublishPreview, showPublishView} =
        usePublishPermissions()

    /**
     * Specific Handler for "Publish Changes" Button: Incorporates
     * re-authentication logic for actions that require Gold-level permissions.
     */
    const handlePublish = async () => {
        try {
            if (!isGoldUser()) {
                const baseUrl = new URL(window.location.href)
                // The event and election tabs select Publish by id; its position depends on permissions.
                baseUrl.searchParams.set("tabId", "publish")
                sessionStorage.setItem(EPublishActions.PENDING_PUBLISH_ACTION, "true")
                await reauthWithGold(baseUrl.toString())
            } else {
                onGenerate()
            }
        } catch (error) {
            console.error("Re-authentication failed:", error)
        }
    }

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("publish.empty.header")}
            </Typography>
            {canPublishCreate && canReadPublish && (
                <>
                    <Button onClick={handlePublish} className="publish-add-button">
                        <Add aria-hidden="true" />
                        {t("publish.empty.action")}
                    </Button>
                    <Typography variant="body1" paragraph>
                        {t("common.resources.noResult.askCreate")}
                    </Typography>
                </>
            )}
        </ResourceListStyles.EmptyBox>
    )

    const actions: Action[] = [
        {
            icon: <Visibility className="publish-visibility-icon" />,
            action: setBallotPublicationId,
            showAction: () => showPublishView,
        },
        {
            icon: <Preview className="publish-preview-icon" />,
            action: onPreview,
            showAction: () => showPublishPreview,
        },
    ]

    if (!canReadPublish) {
        return <Empty />
    }

    return (
        <Box>
            <List
                actions={
                    <PublishActions
                        publishType={publishType}
                        status={status}
                        electionStatus={electionStatus}
                        electionPresentation={electionPresentation}
                        changingStatus={changingStatus}
                        kioskModeEnabled={kioskModeEnabled}
                        onlineModeEnabled={onlineModeEnabled}
                        earlyVotingEnabled={earlyVotingEnabled}
                        telephoneVotingEnabled={telephoneVotingEnabled}
                        onGenerate={onGenerate}
                        onChangeStatus={onChangeStatus}
                        onInitialize={onInitialize}
                        initializing={initializing}
                        perCountryInitialization={perCountryInitialization}
                        initializationReportPolicy={initializationReportPolicy}
                        type={EPublishActionsType.List}
                    />
                }
                resource="sequent_backend_ballot_publication"
                filter={
                    electionId
                        ? {
                              election_event_id: electionEventId,
                              election_id: electionId,
                          }
                        : {
                              election_event_id: electionEventId,
                          }
                }
                sort={{field: "created_at", order: "DESC"}}
                filters={filters}
                sx={{flexGrow: 2}}
                empty={<Empty />}
                disableSyncWithLocation
            >
                <ResetFilters />
                <HeaderTitle title={"publish.header.history"} subtitle="" />
                <DatagridConfigurable
                    header={ThreeStateDatagridHeader}
                    omit={OMIT_FIELDS}
                    bulkActionButtons={false}
                >
                    <TextField source="id" />
                    <BooleanField source="is_generated" />
                    <AdminDateField source="published_at" seconds />
                    <AdminDateField source="created_at" seconds />
                    <WrapperField label={String(t("common.label.actions"))}>
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DatagridConfigurable>
            </List>
        </Box>
    )
}
