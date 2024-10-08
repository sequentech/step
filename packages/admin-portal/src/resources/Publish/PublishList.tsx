// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useEffect} from "react"

import {useTranslation} from "react-i18next"
import {Visibility} from "@mui/icons-material"
import {IconButton} from "@sequentech/ui-essentials"
import {Box, Typography, Button} from "@mui/material"
import {faPlus} from "@fortawesome/free-solid-svg-icons"

import {
    List,
    TextField,
    Identifier,
    TextInput,
    BooleanInput,
    BooleanField,
    DatagridConfigurable,
    useListController,
} from "react-admin"

import {ElectionEventStatus, PublishStatus} from "./EPublishStatus"
import {PublishActions} from "./PublishActions"
import {EPublishActionsType} from "./EPublishType"
import {HeaderTitle} from "@/components/HeaderTitle"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {useNavigate, useLocation} from "react-router-dom"

const OMIT_FIELDS: string[] = []

const filters: Array<ReactElement> = [
    <TextInput source="id" key={0} />,
    <BooleanInput source="is_generated" key={1} />,
]

type TPublishList = {
    status: PublishStatus
    electionId?: number | string
    electionEventId: number | string | undefined
    canRead: boolean
    canWrite: boolean
    changingStatus: boolean
    onGenerate: () => void
    onChangeStatus: (status: ElectionEventStatus) => void
    setBallotPublicationId: (id: string | Identifier) => void
}

export const PublishList: React.FC<TPublishList> = ({
    status,
    electionId,
    electionEventId,
    canRead,
    canWrite,
    changingStatus,
    onGenerate = () => null,
    onChangeStatus = () => null,
    setBallotPublicationId = () => null,
}) => {
    const {t} = useTranslation()

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("publish.empty.header")}
            </Typography>
            {canWrite && (
                <>
                    <Button onClick={onGenerate} className="publish-add-button">
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
    ]
    // Avoid error when coming from filterd list in other tabs
    const listContext = useListController({
        resource: "sequent_backend_ballot_publication",
        filter: electionId
            ? {
                  election_event_id: electionEventId,
                  election_id: electionId,
              }
            : {
                  election_event_id: electionEventId,
              },
    })

    const navigate = useNavigate()
    const location = useLocation()

    useEffect(() => {
        // navigate to self but without search params
        navigate(
            {
                pathname: location.pathname,
                search: "",
            },
            {replace: true}
        )

        // Reset filters when the component mounts
        if (listContext && listContext.setFilters) {
            listContext.setFilters(
                electionId
                    ? {
                          election_event_id: electionEventId,
                          election_id: electionId,
                      }
                    : {
                          election_event_id: electionEventId,
                      },
                {}
            )
        }
    }, [electionEventId, electionId])

    // check if data array is empty
    const {data, isLoading} = listContext

    if (!canRead) {
        return <Empty />
    }

    return (
        <Box>
            {(!isLoading && (!data || data.length)) === 0 ? (
                <Empty />
            ) : (
                <List
                    actions={
                        <PublishActions
                            status={status}
                            changingStatus={changingStatus}
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
                    sort={{
                        field: "created_at",
                        order: "DESC",
                    }}
                    filters={filters}
                    sx={{flexGrow: 2}}
                    empty={<Empty />}
                >
                    <HeaderTitle title={"publish.header.history"} subtitle="" />

                    <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                        <TextField source="id" />
                        <BooleanField source="is_generated" />
                        <TextField source="published_at" />
                        <TextField source="created_at" />

                        <ActionsColumn actions={actions} />
                    </DatagridConfigurable>
                </List>
            )}
        </Box>
    )
}
