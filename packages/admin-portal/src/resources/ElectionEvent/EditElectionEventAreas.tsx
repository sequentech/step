import React from 'react'
import {List} from "react-admin"
import { EditElectionEventAreasList } from './EditElectionEventAreasList'
import { ListArea } from '../Area/ListArea'

export const EditElectionEventAreas: React.FC = () => {
    return (
        <List>
            <ListArea />
        </List>
    )
}