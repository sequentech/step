import React, { ComponentType, useEffect, useRef, useState } from "react"

import { DatagridConfigurable, Empty, List, TextField } from 'react-admin'

import { PublishActions } from './PublishActions'
import { HeaderTitle } from '@/components/HeaderTitle'
import { EPublishActionsType } from './EPublishActionsType'

const OMIT_FIELDS: any = []

export const PublishList = () => {
    return (
        <List
            resource="sequent_backend_tally_session"
            actions={
                <PublishActions 
                    status={9}
                    onPublish={() => null} 
                    onGenerate={() => null}
                    type={EPublishActionsType.List}
                />
            }
            empty={<Empty />}
            sx={{flexGrow: 2}}
            filter={{
            }}
        >
            <HeaderTitle title={"electionEventScreen.tally.title"} subtitle="" />

            <DatagridConfigurable omit={OMIT_FIELDS}>
                <TextField source="tenant_id" />
            </DatagridConfigurable>
        </List>
    )
}