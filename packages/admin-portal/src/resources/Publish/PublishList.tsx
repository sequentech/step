import React, {ReactElement} from "react"

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
} from "react-admin"

import {PublishActions} from "./PublishActions"
import {EPublishActionsType} from "./EPublishType"
import {HeaderTitle} from "@/components/HeaderTitle"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Action, ActionsColumn} from "@/components/ActionButons"

const OMIT_FIELDS: string[] = []

const filters: Array<ReactElement> = [
    <TextInput source="id" key={0} />,
    <BooleanInput source="is_generated" key={1} />,
    <TextInput source="published_at" key={2} />,
    <TextInput source="created_at" key={3} />,
]

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
    const {t} = useTranslation()
    console.log(`has electionId=${electionId}`)

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

    return (
        <Box>
            <List
                actions={
                    <PublishActions
                        status={status}
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
        </Box>
    )
}
