import React from 'react'
import { ShowBase } from 'react-admin'
import { ShowElectionEventTabs } from './EditElectionEventDashboard'


export const ElectionEventBaseTabs: React.FC = () => {
    return (
        <ShowBase>
            <ShowElectionEventTabs />
        </ShowBase>
    )
}
