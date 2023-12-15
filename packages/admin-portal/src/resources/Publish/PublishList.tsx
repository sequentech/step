import React, {useEffect, useState} from "react"

import {Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    Button,
    DatagridConfigurable,
    BooleanField,
    List,
    TextField,
    useGetList,
    ListContextProvider,
    useList,
    useNotify,
} from "react-admin"

import {PublishActions} from "./PublishActions"
import {EPublishActionsType} from "./EPublishType"
import {HeaderTitle} from "@/components/HeaderTitle"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {useActionPermissions} from "../ElectionEvent/EditElectionEventKeys"
import {EPublishStatus} from "./EPublishStatus"
import { Sequent_Backend_Ballot_Publication } from "@/gql/graphql"

const OMIT_FIELDS: any = []

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

    const {data, error, isLoading, total} = useGetList<Sequent_Backend_Ballot_Publication>("sequent_backend_ballot_publication", {
        filter: electionId
            ? {
                  election_event_id: electionEventId,
                  election_id: electionId,
              }
            : {
                  election_event_id: electionEventId,
              },
    })

    const ballotContext = useList({
        data,
        filterCallback: (record) =>
            (!!electionId || !record.election_id) &&
            (electionId ? record.election_ids?.some((id: string) => id === electionId) ?? false : true),
    })

    useEffect(() => {
        if (error) {
            notify(t("publish.dialog.error"), {
                type: "error",
            })
        }
    }, [error])

    useEffect(() => {
        console.log("PUBLISH :: DATA", ballotContext)
        if (ballotContext.total === 0 && status === EPublishStatus.Void) {
            onGenerate()
            console.log("PUBLISH :: GENERATE CALLBACK")
        }
    }, [isLoading])

    return (
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
                    rowClick={(id: any) => {
                        setBallotPublicationId(id)
                        return false
                    }}
                >
                    <TextField source="id" />
                    <BooleanField source="is_generated" />
                    <TextField source="published_at" />
                </DatagridConfigurable>
            </ListContextProvider>
        </>
    )
}
