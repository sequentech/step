import React from 'react'
import { ShowBase } from 'react-admin'
import { ElectionEventTabs } from './ElectionEventTabs'


export const ElectionEventBaseTabs: React.FC = () => {
    return (
        <ShowBase>
            <ElectionEventTabs />
        </ShowBase>
    )
}
