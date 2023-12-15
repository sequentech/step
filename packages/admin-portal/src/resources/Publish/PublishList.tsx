import React, { ComponentType, useEffect, useRef, useState } from "react"

import { Button, CreateButton, DatagridConfigurable, Empty, List, TextField } from 'react-admin'

import { PublishActions } from './PublishActions'
import { HeaderTitle } from '@/components/HeaderTitle'
import { EPublishActionsType } from './EPublishActionsType'
import { ResourceListStyles } from '@/components/styles/ResourceListStyles'
import { Typography } from '@mui/material'
import { useActionPermissions } from '../ElectionEvent/EditElectionEventKeys'
import { useTranslation } from 'react-i18next'

const OMIT_FIELDS: any = []

type TPublishList = {
    status: number;
    electionId?: number|string;
    electionEventId: number|string|undefined;
    onPublish: () => void;
    onGenerate: () => void;
    onChangeStatus: (status: string) => void;
}

export const PublishList: React.FC<TPublishList> = ({ 
    status,
    electionId,
    electionEventId, 
    onPublish = () => null,
    onGenerate = () => null,
    onChangeStatus = () => null,
}) => {
    const {t} = useTranslation()
    const {canAdminCeremony} = useActionPermissions()

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t("publish.empty.header")}
            </Typography>
            {canAdminCeremony ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t("common.resources.noResult.askCreate")}
                    </Typography>
                    

                    <Button 
                        onClick={onGenerate} 
                        label={t('')}
                    />
                </>
            ) : null}
        </ResourceListStyles.EmptyBox>
    )
    
    return (
        <List
            resource="sequent_backend_ballot_publication"
            actions={
                <PublishActions 
                    status={status}
                    onPublish={() => null} 
                    onGenerate={() => null}
                    type={EPublishActionsType.List}
                />
            }
            empty={<Empty />}
            sx={{flexGrow: 2}}
            filter={{
                election_event_id: electionEventId,
            }}
        >
            <HeaderTitle title={"electionEventScreen.tally.title"} subtitle="" />

            <DatagridConfigurable omit={OMIT_FIELDS}>
                <TextField source="tenant_id" />
                <TextField source="is_generated" />
                <TextField source="published_at" />
            </DatagridConfigurable>
        </List>
    )
}