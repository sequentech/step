// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useState, useContext, useEffect, useCallback} from "react"
import {useTranslation} from "react-i18next"
import {Visibility, Preview} from "@mui/icons-material"
import {IconButton, Dialog} from "@sequentech/ui-essentials"
import {Box, Typography, Button, DialogContent, DialogActions} from "@mui/material"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {EPublishActions} from "@/types/publishActions"

import {
    List,
    TextField,
    Identifier,
    TextInput,
    BooleanInput,
    BooleanField,
    DatagridConfigurable,
} from "react-admin"

import {ElectionEventStatus, PublishStatus} from "./EPublishStatus"
import {PublishActions} from "./PublishActions"
import {EPublishActionsType} from "./EPublishType"
import {HeaderTitle} from "@/components/HeaderTitle"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ResetFilters} from "@/components/ResetFilters"
import {AuthContext} from "@/providers/AuthContextProvider"
import {VotingStatusChannel} from "@/gql/graphql"
import {IElectionStatus} from "@sequentech/ui-core"

const OMIT_FIELDS: string[] = []

const filters: Array<ReactElement> = [
    <TextInput source="id" key={0} />,
    <BooleanInput source="is_generated" key={1} />,
]

type TPublishList = {
    status: PublishStatus
    electionStatus: IElectionStatus | null
    electionId?: number | string
    electionEventId: number | string | undefined
    canRead: boolean
    canWrite: boolean
    kioskModeEnabled: boolean
    changingStatus: boolean
    onGenerate: () => void
    onChangeStatus: (status: ElectionEventStatus, votingChannel?: VotingStatusChannel) => void
    setBallotPublicationId: (id: string | Identifier) => void
    onPreview: (id: string | Identifier) => void
}

export const PublishList: React.FC<TPublishList> = ({
    status,
    electionStatus,
    electionId,
    electionEventId,
    canRead,
    canWrite,
    kioskModeEnabled,
    changingStatus,
    onGenerate = () => null,
    onChangeStatus = () => null,
    setBallotPublicationId = () => null,
    onPreview = () => null,
}) => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const {isGoldUser, reauthWithGold} = authContext

    const handleGenerateClick = async () => {
        if (isGoldUser()) {
            onGenerate()
        } else {
            try {
                const baseUrl = new URL(window.location.href)
                baseUrl.searchParams.set("tabIndex", "7")

                sessionStorage.setItem(EPublishActions.PENDING_PUBLISH_ACTION, "true")
                await reauthWithGold(baseUrl.toString())

                console.log("Re-authentication successful. Proceeding to generate.")
                onGenerate()
            } catch (error) {
                console.error("Re-authentication failed:", error)
            }
        }
    }

    /**
     * Checks for any pending actions after the component mounts.
     * If a pending action is found, it executes the action and removes the flag.
     */
    useEffect(() => {
        const executePendingActions = async () => {
            if (!electionEventId) {
                return
            }

            const pendingPublish = sessionStorage.getItem(EPublishActions.PENDING_PUBLISH_ACTION)
            if (pendingPublish) {
                sessionStorage.removeItem(EPublishActions.PENDING_PUBLISH_ACTION)
                onGenerate()
            }
        }

        executePendingActions()
    }, [onGenerate, electionEventId])

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("publish.empty.header")}
            </Typography>
            {canWrite && (
                <>
                    <Button onClick={handleGenerateClick} className="publish-add-button">
                        <IconButton icon={faPlus} fontSize="24px" />
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
        },
        {
            icon: <Preview className="publish-preview-icon" />,
            action: onPreview,
        },
    ]

    if (!canRead) {
        return <Empty />
    }

    return (
        <Box>
            <List
                actions={
                    <PublishActions
                        status={status}
                        electionStatus={electionStatus}
                        changingStatus={changingStatus}
                        kioskModeEnabled={kioskModeEnabled}
                        onGenerate={onGenerate}
                        onChangeStatus={onChangeStatus}
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
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <TextField source="id" />
                    <BooleanField source="is_generated" />
                    <TextField source="published_at" />
                    <TextField source="created_at" />
                    <ActionsColumn actions={actions} />
                </DatagridConfigurable>
            </List>
        </Box>
    )
}
