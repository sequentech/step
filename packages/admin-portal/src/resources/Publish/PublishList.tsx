import React, {useEffect} from "react"

import {useTranslation} from "react-i18next"
import {Visibility} from "@mui/icons-material"
import {IconButton} from "@sequentech/ui-essentials"
import {Box, Typography, Button} from "@mui/material"
import {faPlus} from "@fortawesome/free-solid-svg-icons"

import {
    useList,
    TextField,
    useNotify,
    useGetList,
    BooleanField,
    ListContextProvider,
    DatagridConfigurable,
    RaRecord,
    FunctionField,
    Identifier,
    WrapperField,
} from "react-admin"

import {PublishActions} from "./PublishActions"
import {EPublishStatus} from "./EPublishStatus"
import {EPublishActionsType} from "./EPublishType"
import {HeaderTitle} from "@/components/HeaderTitle"
import {Sequent_Backend_Ballot_Publication} from "@/gql/graphql"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Action, ActionsColumn} from "@/components/ActionButons"

const OMIT_FIELDS: string[] = []

type TPublishList = {
    status: number
    onGenerate: () => void
    electionId?: number | string
    electionEventId: number | string | undefined
    onChangeStatus: (status: string) => void
    setBallotPublicationId: (id: string | Identifier) => void
}

export const PublishList: React.FC<TPublishList> = ({
    status,
    electionId,
    electionEventId,
    onGenerate = () => null,
    onChangeStatus = () => null,
    setBallotPublicationId = () => null,
}) => {
    const notify = useNotify()
    const {t} = useTranslation()

    const {data, error} = useGetList<Sequent_Backend_Ballot_Publication>(
        "sequent_backend_ballot_publication",
        {
            filter: electionId
                ? {
                      election_event_id: electionEventId,
                      election_id: electionId,
                  }
                : {
                      election_event_id: electionEventId,
                  },
        }
    )

    const ballotContext = useList({
        data,
        filterCallback: (record) =>
            (!!electionId || !record.election_id) &&
            (electionId
                ? record.election_ids?.some((id: string) => id === electionId) ?? false
                : true),
    })

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("publish.empty.header")}
            </Typography>
            <>
                <Typography variant="body1" paragraph>
                    {t("common.resources.noResult.askCreate")}
                </Typography>

                <Button onClick={onGenerate}>
                    <IconButton icon={faPlus} fontSize="24px" />
                    {t("publish.empty.action")}
                </Button>
            </>
        </ResourceListStyles.EmptyBox>
    )

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: setBallotPublicationId,
        },
    ]

    useEffect(() => {
        if (error) {
            notify(t("publish.dialog.error"), {
                type: "error",
            })
        }
    }, [error])

    return (
        <Box>
            {ballotContext.total ? (
                <>
                    <PublishActions
                        status={status}
                        onGenerate={onGenerate}
                        onChangeStatus={onChangeStatus}
                        type={EPublishActionsType.List}
                    />

                    <ListContextProvider value={ballotContext}>
                        <HeaderTitle title={"publish.header.history"} subtitle="" />

                        <DatagridConfigurable omit={OMIT_FIELDS}>
                            <TextField source="published_at" />
                            <TextField source="created_at" />
                            <BooleanField source="is_generated" />

                            <WrapperField source="actions" label="Actions">
                                <ActionsColumn actions={actions} />
                            </WrapperField>
                        </DatagridConfigurable>
                    </ListContextProvider>
                </>
            ) : (
                <Empty />
            )}
        </Box>
    )
}
