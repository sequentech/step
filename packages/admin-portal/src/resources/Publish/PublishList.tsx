import React from "react"

import { Typography } from '@mui/material'
import { useTranslation } from 'react-i18next'
import { Button, DatagridConfigurable, BooleanField, List, TextField } from 'react-admin'

import { PublishActions } from './PublishActions'
import { EPublishActionsType } from './EPublishType'
import { HeaderTitle } from '@/components/HeaderTitle'
import { ResourceListStyles } from '@/components/styles/ResourceListStyles'
import { useActionPermissions } from '../ElectionEvent/EditElectionEventKeys'

const OMIT_FIELDS: any = []

type TPublishList = {
    status: number
    onGenerate: () => void
    electionId?: number|string
    electionEventId: number|string|undefined
}

export const PublishList: React.FC<TPublishList> = ({ 
    status,
    electionId,
    electionEventId,
    onGenerate = () => null,
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
                        label={t('publish.empty.action')}
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
            <HeaderTitle title={"publish.header.history"} subtitle="" />

            <DatagridConfigurable omit={OMIT_FIELDS}>
                <TextField source="tenant_id" />
                <BooleanField source="is_generated" />
                <TextField source="published_at" />
            </DatagridConfigurable>
        </List>
    )
}