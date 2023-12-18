import React, {useEffect} from "react"

import {useTranslation} from "react-i18next"
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
} from "react-admin"

import {PublishActions} from "./PublishActions"
import {EPublishStatus} from "./EPublishStatus"
import {EPublishActionsType} from "./EPublishType"
import {HeaderTitle} from "@/components/HeaderTitle"
import {Sequent_Backend_Ballot_Publication} from "@/gql/graphql"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"

const OMIT_FIELDS: string[] = []

type TPublishList = {
    status: number
    onGenerate: () => void
    electionId?: number | string
    electionEventId: number | string | undefined
    onChangeStatus: (status: string) => void
    setBallotPublicationId: (id: string) => void
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

    const {data, error, isLoading} = useGetList<Sequent_Backend_Ballot_Publication>(
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

    useEffect(() => {
        if (error) {
            notify(t("publish.dialog.error"), {
                type: "error",
            })
        }
    }, [error])

    useEffect(() => {
        console.log("PUBLISH :: DATA", data)
        console.log("PUBLISH :: BALLOT CONTEXT", ballotContext)
    }, [data])

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

                        <DatagridConfigurable
                            omit={OMIT_FIELDS}
                            rowClick={(id: string | number) => {
                                setBallotPublicationId(String(id)) // Asegúrate de convertir a string si es necesario

                                return false
                            }}
                        >
                            <TextField source="published_at" />
                            <BooleanField source="is_generated" />
                        </DatagridConfigurable>
                    </ListContextProvider>
                </>
            ) : (
                <Empty />
            )}
        </Box>
    )
}
