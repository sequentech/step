import React from 'react'
import { EditBase } from 'react-admin'
import { EditElectionEventDataForm } from './EditElectionEventDataForm'

export const EditElectionEventData: React.FC = () => {
    return (
        <EditBase redirect={"."}>
            <EditElectionEventDataForm />
        </EditBase>
    )
}